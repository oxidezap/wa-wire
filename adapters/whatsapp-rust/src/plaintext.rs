//! Joining a frame to the plaintexts decrypted out of it.
//!
//! The two arrive separately and cannot be made to arrive together. A stanza is
//! decoded — and its frame observable — before Signal has run; a plaintext
//! exists only afterwards. An adapter that wants to emit L0-plain has to hold
//! the frame until its plaintexts catch up.
//!
//! # Knowing when to stop waiting
//!
//! The frame itself says how many `<enc>` nodes the stanza has, so the common
//! case — every one decrypts — closes by counting, with no clock involved: the
//! last plaintext completes the table and the envelope goes out immediately.
//!
//! What has no signal is an `<enc>` that will never produce a plaintext. The
//! engine drops one silently in several places (an unrecognised type, an empty
//! body, a custom handler taking it, a session duplicate), and its
//! `UndecryptableMessage` event is per *message* — deduplicated by id, and only
//! dispatched when the whole message failed — so it cannot close a per-node
//! table either. Something has to give up.
//!
//! Giving up is measured in stanzas, not milliseconds. The engine processes the
//! receive path in order, so a message whose plaintexts have not arrived after
//! [`DEFAULT_LOOKAHEAD`] later stanzas is one whose plaintexts are not coming.
//! A stanza count is also the same on every machine, which a duration is not —
//! and this crate's output is meant to be compared against other engines'.
//!
//! # Cost
//!
//! One entry per in-flight `<message>`, holding the frame it already had. A
//! stanza with no `<enc>` never enters the buffer, and the sweep is amortised:
//! entries are checked as later stanzas arrive, not on a timer.
//!
//! Nothing is parsed here. The engine hands over the tree it already decoded,
//! so counting `<enc>` children is a walk over that — and a second parser could
//! not disagree with the first about what the stanza contains.
//!
//! # Fan-out stanzas are left as L0-wire
//!
//! A fan-out `<message>` carries a copy per device under
//! `<participants><to jid=…>`, and the engine enumerates the ones addressed to
//! *its* device after the direct children. Reproducing that numbering needs the
//! device's own JID, which an adapter installed as a plugin does not have — so
//! for those stanzas the index cannot be resolved to a node with certainty.
//!
//! They are emitted immediately with no plaintext table. A frame without
//! payloads is a smaller claim than a payload on the wrong `<enc>`, which would
//! read as a message from the wrong device.

use std::num::NonZeroUsize;
use std::sync::Arc;

use wa_wire_adapter::{NodePathBuf, Plaintext, PlaintextStatus, RawStanza, StanzaSink};
use whatsapp_rust::OwnedNodeRef;
use whatsapp_rust::bytes::Bytes;

/// How many later stanzas a pending message tolerates before it is emitted with
/// whatever it has.
///
/// Sized for the widest real fan-out rather than tuned: the largest stanza in
/// the reference capture carries a few thousand nodes, but a single message's
/// plaintexts all arrive within its own processing, so anything past a handful
/// of intervening stanzas already means they are not coming.
pub const DEFAULT_LOOKAHEAD: usize = 64;

impl Default for PlaintextJoiner {
    fn default() -> Self {
        Self::new()
    }
}

/// One `<enc>` the engine could not turn into a plaintext, and why.
///
/// The counterpart of [`DecryptedEnc`], reported from the same loop and
/// numbered the same way, which is what lets one slot take either.
#[derive(Debug, Clone)]
pub struct FailedEnc {
    /// The stanza id this belongs to.
    pub message_id: String,
    /// Which `<enc>` of the stanza it was, counting from zero.
    pub enc_index: usize,
    /// What the boundary can say about it.
    pub status: PlaintextStatus,
}

/// One decrypted payload, as the engine reports it.
#[derive(Debug, Clone)]
pub struct DecryptedEnc {
    /// The stanza id this belongs to.
    pub message_id: String,
    /// Which `<enc>` of the stanza produced it, counting from zero.
    pub enc_index: usize,
    /// The plaintext.
    pub payload: Bytes,
}

/// A frame waiting for the plaintexts decrypted out of it.
struct Pending {
    message_id: String,
    frame: Bytes,
    /// One slot per `<enc>`, in stanza order. Empty until the engine settles it
    /// one way or the other.
    slots: Vec<Slot>,
    /// Index of each `<enc>` among the root's children, so a slot can be
    /// addressed without re-walking the frame.
    child_indices: Vec<u16>,
    /// How many stanzas have gone by since this one arrived.
    age: usize,
    /// Whether the wait is over, payloads or not.
    given_up: bool,
}

/// What became of one `<enc>`.
#[derive(Debug, Clone)]
enum Slot {
    /// Nothing has arrived for it yet.
    Waiting,
    /// Signal produced these bytes.
    Payload(Bytes),
    /// The engine said it produced none, and why.
    Cause(PlaintextStatus),
}

impl Slot {
    /// Whether the engine has said anything about this `<enc>`.
    const fn is_settled(&self) -> bool {
        !matches!(self, Self::Waiting)
    }
}

impl Pending {
    /// Whether this stanza is finished and only waiting for its turn.
    fn is_complete(&self) -> bool {
        self.given_up || self.slots.iter().all(Slot::is_settled)
    }

    /// Whether every slot was settled, as against given up on.
    fn is_whole(&self) -> bool {
        self.slots.iter().all(Slot::is_settled)
    }
}

/// Holds frames until their plaintexts arrive, then emits one envelope each.
///
/// Not internally synchronised: the engine dispatches from its receive path, so
/// a caller shares this behind the same lock it holds the sink with. That also
/// keeps a frame and its plaintexts from interleaving with another stanza's.
pub struct PlaintextJoiner {
    pending: Vec<Pending>,
    lookahead: NonZeroUsize,
    /// Frames given up on, for a caller that wants to know it is happening.
    abandoned: u64,
}

impl PlaintextJoiner {
    /// A joiner with [`DEFAULT_LOOKAHEAD`].
    #[must_use]
    pub fn new() -> Self {
        Self::with_lookahead(NonZeroUsize::new(DEFAULT_LOOKAHEAD).unwrap_or(NonZeroUsize::MIN))
    }

    /// A joiner that gives up after `lookahead` later stanzas.
    ///
    /// Lower values give up sooner on a message whose plaintexts are slow, and
    /// nothing here waits on wall-clock time, so the only cost of a larger one
    /// is holding a frame longer.
    #[must_use]
    pub fn with_lookahead(lookahead: NonZeroUsize) -> Self {
        Self {
            pending: Vec::new(),
            lookahead,
            abandoned: 0,
        }
    }

    /// How many frames were emitted without all of their plaintexts.
    #[must_use]
    pub const fn abandoned(&self) -> u64 {
        self.abandoned
    }

    /// How many stanzas are waiting on payloads.
    #[must_use]
    pub fn pending(&self) -> usize {
        self.pending
            .iter()
            .filter(|pending| !pending.is_complete())
            .count()
    }

    /// How many are queued: waiting, or merely behind one that is.
    ///
    /// Distinct from [`Self::pending`] since stanzas leave in arrival order —
    /// a finished one behind an unfinished one is not waiting on anything, and
    /// is not gone either.
    #[must_use]
    pub fn queued(&self) -> usize {
        self.pending.len()
    }

    /// Take a decoded stanza.
    ///
    /// A stanza with `<enc>` children waits for its plaintexts; anything else
    /// is finished on arrival. Either way it takes its place in the queue, and
    /// the queue drains in order.
    ///
    /// Emitting an unheld stanza the moment it arrived would put it ahead of a
    /// held one that came first, and a recording compared position by position
    /// reports that as a divergence in whichever engine happened to be slower.
    /// What leaves is what arrived, in that order.
    pub fn accept_frame<S: StanzaSink>(&mut self, node: &Arc<OwnedNodeRef>, sink: &mut S) {
        // Ages first, so a stanza given up on leaves ahead of the one that
        // aged it out — which is where the wire put it.
        self.age_pending();

        self.pending.push(Self::begin(node).unwrap_or_else(|| Pending {
            message_id: String::new(),
            frame: node.backing_bytes(),
            slots: Vec::new(),
            child_indices: Vec::new(),
            age: 0,
            given_up: false,
        }));
        self.drain(sink);
    }

    /// Take a plaintext the engine decrypted.
    ///
    /// Completing a message's last slot emits it immediately. A plaintext for a
    /// message that is not waiting — one already given up on, or one whose
    /// frame was never seen — is dropped: there is no frame to attach it to,
    /// and inventing one would be worse than losing it.
    pub fn accept_plaintext<S: StanzaSink>(&mut self, decrypted: &DecryptedEnc, sink: &mut S) {
        let Some(position) = self
            .pending
            .iter()
            .position(|pending| pending.message_id == decrypted.message_id)
        else {
            return;
        };
        let pending = &mut self.pending[position];
        let Some(slot) = pending.slots.get_mut(decrypted.enc_index) else {
            // The engine reported an `<enc>` the frame does not have, which
            // means the two disagree about the stanza. Keeping the frame is the
            // conservative half of that: it still emits, just without this one.
            return;
        };
        *slot = Slot::Payload(decrypted.payload.clone());
        self.drain(sink);
    }

    /// Take one `<enc>` the engine could not decrypt, with the cause it gave.
    ///
    /// A cause never displaces a plaintext. The engine reports both for the
    /// same `<enc>` when the bytes existed and would not parse, and the bytes
    /// are the more useful half — a consumer holding them can decide for
    /// itself, where a consumer holding only "unusable" cannot.
    pub fn accept_failure<S: StanzaSink>(&mut self, failed: &FailedEnc, sink: &mut S) {
        let Some(position) = self
            .pending
            .iter()
            .position(|pending| pending.message_id == failed.message_id)
        else {
            return;
        };
        let pending = &mut self.pending[position];
        let Some(slot) = pending.slots.get_mut(failed.enc_index) else {
            // Same disagreement as above, handled the same way.
            return;
        };
        if matches!(slot, Slot::Waiting) {
            *slot = Slot::Cause(failed.status);
        }
        self.drain(sink);
    }

    /// Emit every frame still waiting, complete or not.
    ///
    /// For a caller shutting the adapter down: whatever is buffered is the last
    /// anyone will hear about those stanzas.
    pub fn flush<S: StanzaSink>(&mut self, sink: &mut S) {
        for pending in core::mem::take(&mut self.pending) {
            if !pending.is_whole() && !pending.given_up {
                self.abandoned = self.abandoned.saturating_add(1);
            }
            emit(&pending, sink);
        }
    }

    /// Take the finished stanzas off the front of the queue.
    ///
    /// The front, and only the front: a finished stanza behind an unfinished
    /// one waits, because the unfinished one arrived first.
    fn drain<S: StanzaSink>(&mut self, sink: &mut S) {
        let ready = self
            .pending
            .iter()
            .take_while(|pending| pending.is_complete())
            .count();
        for pending in self.pending.drain(..ready) {
            emit(&pending, sink);
        }
    }

    /// Start holding `node`'s frame, or `None` if it has nothing to wait for.
    ///
    /// Only a stanza with both an `id` and at least one `<enc>` waits: without
    /// an id no plaintext could be matched back to it, and without an `<enc>`
    /// there is nothing to wait for. A fan-out stanza does not wait either —
    /// see the module documentation.
    fn begin(node: &Arc<OwnedNodeRef>) -> Option<Pending> {
        let root = node.get();
        let message_id = root.get_attr("id")?.as_str();
        let children = root.children()?;
        if children
            .iter()
            .any(|child| child.tag.as_ref() == "participants")
        {
            return None;
        }
        let child_indices: Vec<u16> = children
            .iter()
            .enumerate()
            .filter(|(_, child)| child.tag.as_ref() == "enc")
            .map(|(index, _)| u16::try_from(index).ok())
            .collect::<Option<_>>()?;
        if child_indices.is_empty() {
            return None;
        }

        Some(Pending {
            message_id: message_id.into_owned(),
            frame: node.backing_bytes(),
            slots: vec![Slot::Waiting; child_indices.len()],
            child_indices,
            age: 0,
            given_up: false,
        })
    }

    /// Age every waiting frame by one stanza, emitting those that ran out.
    /// Age everything waiting, marking whatever ran out of patience as done.
    ///
    /// Marked rather than emitted: a stanza given up on still leaves in its own
    /// place in the queue, and `drain` is the only thing that emits.
    fn age_pending(&mut self) {
        let lookahead = self.lookahead.get();
        for pending in &mut self.pending {
            if pending.given_up || pending.is_complete() {
                continue;
            }
            pending.age = pending.age.saturating_add(1);
            if pending.age > lookahead {
                pending.given_up = true;
                self.abandoned = self.abandoned.saturating_add(1);
            }
        }
    }
}

/// Hand one pending frame to the sink, with the table it accumulated.
fn emit<S: StanzaSink>(pending: &Pending, sink: &mut S) {
    let mut paths = Vec::with_capacity(pending.slots.len());
    for child_index in &pending.child_indices {
        let mut path = NodePathBuf::new();
        // One component, and the buffer holds far more than one.
        let _ = path.push(*child_index);
        paths.push(path);
    }

    let mut plaintexts = Vec::with_capacity(pending.slots.len());
    for (slot, path) in pending.slots.iter().zip(&paths) {
        plaintexts.push(match slot {
            Slot::Payload(payload) => Plaintext::ok(path.as_path(), payload),
            Slot::Cause(PlaintextStatus::Unsupported) => Plaintext::unsupported(path.as_path()),
            Slot::Cause(_) => Plaintext::failed(path.as_path()),
            // Nothing arrived, for or against. The engine reports both halves,
            // so this is the case where neither reached us — the adapter
            // stopped watching, or the stanza was given up on.
            Slot::Waiting => Plaintext::unobserved(path.as_path()),
        });
    }

    sink.accept(RawStanza::inbound(&pending.frame).with_plaintexts(&plaintexts));
}

#[cfg(test)]
#[path = "plaintext_tests.rs"]
mod tests;
