//! Telling a stanza that arrived twice from two stanzas that look alike.
//!
//! Moving a session from one engine to another is stop-the-world: the old
//! connection is torn down and a new one is brought up (RFC-003). Inbound
//! traffic survives that, because the server queues it. Acknowledgements in
//! flight do not — the server has no way to know the ack it sent was read, so
//! after the window it sends it again. R3 makes deduplicating those mandatory
//! rather than a refinement, and here is where it lives.
//!
//! # Why not inside `derive`
//!
//! Because [`derive`](crate::derive) is a pure function of one stanza (D-010),
//! and telling a redelivery from a first arrival is exactly the knowledge one
//! stanza does not carry. Keeping the two apart is what lets the derivation stay
//! the thing four engines can be compared on: a stateful `derive` would give
//! different answers to the same input depending on what it had seen, and
//! nothing could be replayed.
//!
//! So this is a separate thing a caller drives, and a caller that does not need
//! it pays nothing for it.
//!
//! # What it can and cannot tell
//!
//! A stanza is identified by its tag and its `id` attribute. Two arrivals with
//! the same pair are the same stanza; an `<ack>` and a `<receipt>` that share
//! an id are not, which is why the tag is part of the key.
//!
//! Two things are outside that. A stanza with no `id` cannot be tracked, and is
//! reported as [`Untracked`](Admission::Untracked) rather than guessed at.
//! And the window is bounded, so an id that falls out of it and comes back
//! reads as new — which is the honest trade for holding no allocation and no
//! unbounded state.

use wa_wire_codec::NodeRef;

/// The longest tag this can hold.
///
/// The four modelled tags are at most seven bytes; the margin is for a tag this
/// build does not model, which is still a stanza a caller may want deduplicated.
const MAX_TAG: usize = 24;

/// The longest `id` this can hold.
///
/// WhatsApp writes ids as hexadecimal runs well under this. A longer one is
/// reported as untracked rather than truncated, because a truncated id would
/// collide with every other id sharing its prefix and report real traffic as a
/// duplicate — the one error here that loses a message.
const MAX_ID: usize = 64;

/// What one stanza was, against everything still in the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Admission {
    /// Not in the window. The caller should process it.
    New,
    /// Already in the window: the same tag and id arrived before.
    Duplicate,
    /// Cannot be tracked, so nothing is claimed either way.
    ///
    /// A stanza with no `id`, or one whose tag or id is longer than this can
    /// hold. Distinct from [`New`](Self::New) because a caller counting
    /// duplicates should not read this as evidence there were none — the same
    /// reason the gate reports `incomparable` rather than folding it into a
    /// verdict.
    Untracked,
}

impl Admission {
    /// Whether the caller should process this stanza.
    ///
    /// Untracked counts as yes: refusing a stanza this cannot identify would
    /// drop real traffic to avoid a duplicate it never detected.
    #[must_use]
    pub const fn should_process(self) -> bool {
        matches!(self, Self::New | Self::Untracked)
    }
}

/// One remembered stanza.
///
/// Bytes inline rather than a reference: a stanza's id borrows the frame it
/// arrived in, and the whole point is to outlive that frame.
#[derive(Clone, Copy)]
struct Slot {
    tag: [u8; MAX_TAG],
    tag_len: u8,
    id: [u8; MAX_ID],
    id_len: u8,
}

impl Slot {
    const EMPTY: Self = Self {
        tag: [0; MAX_TAG],
        tag_len: 0,
        id: [0; MAX_ID],
        id_len: 0,
    };

    fn matches(&self, tag: &[u8], id: &[u8]) -> bool {
        usize::from(self.tag_len) == tag.len()
            && usize::from(self.id_len) == id.len()
            && self.tag.get(..tag.len()) == Some(tag)
            && self.id.get(..id.len()) == Some(id)
    }
}

/// The stanzas seen recently, so a redelivery can be told from an arrival.
///
/// `WINDOW` is how many are remembered. Size it by what a handoff can leave in
/// flight rather than by how long a session runs: the acks that duplicate are
/// the ones the old connection had not finished sending, which is a handful,
/// and the default is generous against that.
///
/// Holds no allocation and never grows. On a `WINDOW` of 64 that is about six
/// kilobytes, which is the price of not having a heap.
///
/// ```
/// use wa_wire_codec::NodeRef;
/// use wa_wire_l1::dedup::SeenStanzas;
///
/// fn on_stanza(seen: &mut SeenStanzas<64>, node: &NodeRef<'_>) {
///     if seen.admit(node).should_process() {
///         // derive it, forward it, count it
///     }
/// }
/// ```
#[derive(Clone)]
pub struct SeenStanzas<const WINDOW: usize = 64> {
    slots: [Slot; WINDOW],
    /// Where the next stanza goes, wrapping.
    next: usize,
    /// How many slots hold something, until the ring has been round once.
    filled: usize,
}

impl<const WINDOW: usize> Default for SeenStanzas<WINDOW> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const WINDOW: usize> SeenStanzas<WINDOW> {
    /// A window remembering nothing yet.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            slots: [Slot::EMPTY; WINDOW],
            next: 0,
            filled: 0,
        }
    }

    /// How many stanzas this remembers.
    #[must_use]
    pub const fn window(&self) -> usize {
        WINDOW
    }

    /// How many it is currently holding.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.filled
    }

    /// Whether it has seen nothing yet.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.filled == 0
    }

    /// Forget everything.
    ///
    /// For a caller starting a session rather than resuming one: ids do not
    /// carry meaning across a pairing, so keeping them would only spend window
    /// on stanzas that cannot recur.
    pub fn clear(&mut self) {
        self.next = 0;
        self.filled = 0;
    }

    /// Offer one stanza, and say whether it has been seen.
    ///
    /// A new stanza is remembered, evicting the oldest when the window is full.
    /// A duplicate is **not** remembered again: re-inserting it would push out
    /// an older entry each time a redelivery arrived, and a burst of them would
    /// empty the window of everything it was there to recognise.
    pub fn admit(&mut self, node: &NodeRef<'_>) -> Admission {
        let (Some(tag), Some(id)) = (
            node.tag().as_str().map(str::as_bytes),
            node.attr("id").and_then(wa_wire_codec::Value::as_str),
        ) else {
            return Admission::Untracked;
        };
        let id = id.as_bytes();

        if tag.len() > MAX_TAG || id.len() > MAX_ID {
            return Admission::Untracked;
        }

        if self
            .slots
            .iter()
            .take(self.filled)
            .any(|slot| slot.matches(tag, id))
        {
            return Admission::Duplicate;
        }

        self.remember(tag, id);
        Admission::New
    }

    fn remember(&mut self, tag: &[u8], id: &[u8]) {
        let Some(slot) = self.slots.get_mut(self.next) else {
            // A zero-length window remembers nothing, which is a caller saying
            // it wants no deduplication rather than an error.
            return;
        };

        *slot = Slot::EMPTY;
        if let Some(into) = slot.tag.get_mut(..tag.len()) {
            into.copy_from_slice(tag);
        }
        if let Some(into) = slot.id.get_mut(..id.len()) {
            into.copy_from_slice(id);
        }
        // Both fit: the caller checked before reaching here.
        slot.tag_len = u8::try_from(tag.len()).unwrap_or(0);
        slot.id_len = u8::try_from(id.len()).unwrap_or(0);

        // Wrapped by comparison rather than by `%`: a modulo here would be a
        // second place the zero-window case has to be right in.
        let advanced = self.next.saturating_add(1);
        self.next = if advanced >= WINDOW { 0 } else { advanced };
        self.filled = self.filled.saturating_add(1).min(WINDOW);
    }
}

// The slots are deliberately absent: they hold ids, and an id is traffic.
#[expect(
    clippy::missing_fields_in_debug,
    reason = "the ids are traffic and do not belong in a log line"
)]
impl<const WINDOW: usize> core::fmt::Debug for SeenStanzas<WINDOW> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // The ids themselves are traffic, so they stay out of a log line.
        formatter
            .debug_struct("SeenStanzas")
            .field("window", &WINDOW)
            .field("held", &self.filled)
            .finish()
    }
}

#[cfg(test)]
#[path = "dedup_tests.rs"]
mod tests;
