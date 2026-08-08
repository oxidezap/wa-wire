//! Running the comparison.

extern crate alloc;
use alloc::vec::Vec;

use wa_wire_adapter::Capability;
use wa_wire_codec::{Parser, TokenTable};
use wa_wire_contract::{Direction, EnvelopeRef, FrameOrigin, NodePath, PlaintextEntry};
use wa_wire_l1::{DeriveError, Event, OutgoingEvent, derive, derive_outgoing};

use crate::comparability;
use crate::divergence::Divergence;
use crate::profile::{ComparisonProfile, Incomparable, Verdict};
use crate::recording::Recording;

/// The token dictionary each side's frames were encoded against.
///
/// One per recording rather than one per comparison (D-082): the dictionary
/// travels with the WhatsApp client version, and an upgrade gate compares
/// exactly the builds where it may have moved. Two builds writing different
/// token indices for one value is a dictionary difference, not an engine one.
#[derive(Debug, Clone, Copy)]
pub struct Tables<'a> {
    /// The first recording's dictionary.
    pub left: TokenTable<'a>,
    /// The second's.
    pub right: TokenTable<'a>,
}

impl<'a> Tables<'a> {
    /// One dictionary for both sides — correct only when both were encoded
    /// against it, which [`Comparability`] is what actually establishes.
    ///
    /// [`Comparability`]: crate::Comparability
    #[must_use]
    pub const fn shared(table: TokenTable<'a>) -> Self {
        Self {
            left: table,
            right: table,
        }
    }
}

/// What a comparison found.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Report<'a> {
    divergences: Vec<Divergence<'a>>,
    compared: usize,
    incomparable: Option<Incomparable>,
}

impl<'a> Report<'a> {
    /// The verdict under `profile`.
    ///
    /// Comparability is checked first and short-circuits everything else: a
    /// comparison between unlike things produces findings that read exactly
    /// like real ones, so reporting them as agreement or as failure would both
    /// be wrong.
    #[must_use]
    pub fn evaluate(&self, profile: ComparisonProfile) -> Verdict {
        if let Some(reason) = self.incomparable {
            return Verdict::Incomparable(reason);
        }
        if self
            .divergences
            .iter()
            .any(|divergence| profile.is_failure(divergence))
        {
            return Verdict::Fail;
        }
        Verdict::Pass
    }

    /// Whether two engines agreed on everything that counts.
    ///
    /// The [`Interop`](ComparisonProfile::Interop) shorthand. Two things do not
    /// stop this being true: a frame difference, because two encodings of one
    /// stanza are both valid, and a plaintext coverage difference, because how
    /// much an adapter can observe is not something an engine got wrong.
    #[must_use]
    pub fn agrees(&self) -> bool {
        self.evaluate(ComparisonProfile::Interop).is_pass()
    }

    /// Why the recordings could not be compared, if they could not.
    #[must_use]
    pub const fn incomparable(&self) -> Option<Incomparable> {
        self.incomparable
    }

    /// Whether the recordings were byte-identical as well as equivalent.
    #[must_use]
    pub fn is_identical(&self) -> bool {
        self.divergences.is_empty()
    }

    /// Everything found, in the order it was found.
    pub fn divergences(&self) -> impl Iterator<Item = &Divergence<'a>> {
        self.divergences.iter()
    }

    /// Only the ones that fail under `profile`.
    pub fn failures(&self, profile: ComparisonProfile) -> impl Iterator<Item = &Divergence<'a>> {
        self.divergences
            .iter()
            .filter(move |d| profile.is_failure(d))
    }

    /// Only the ones where the candidate did better than the baseline.
    ///
    /// Reported rather than folded into silence: a gate that cannot say what
    /// improved can only ever deliver bad news.
    pub fn improvements(
        &self,
        profile: ComparisonProfile,
    ) -> impl Iterator<Item = &Divergence<'a>> {
        self.divergences
            .iter()
            .filter(move |d| profile.is_improvement(d))
    }

    /// Only the ones that fail two engines against each other.
    pub fn faults(&self) -> impl Iterator<Item = &Divergence<'a>> {
        self.failures(ComparisonProfile::Interop)
    }

    /// How many stanzas were compared.
    #[must_use]
    pub const fn compared(&self) -> usize {
        self.compared
    }

    fn push(&mut self, divergence: Divergence<'a>) {
        self.divergences.push(divergence);
    }
}

/// Compare two recordings of the same traffic.
///
/// Both recordings must be of the *same* stanzas, in the same order — two
/// engines fed one capture, not two engines watching two sessions. That used to
/// be a precondition only a doc comment stated; a recording read from a
/// container now declares it, and this checks it (D-078).
///
/// A pair that may not be compared still produces a report: the divergences are
/// collected as usual, and [`evaluate`](Report::evaluate) returns
/// [`Incomparable`] rather than a verdict about them. Collecting them anyway is
/// what lets a human see *why* two recordings were not alike.
#[must_use]
pub fn compare<'a>(left: &Recording<'a>, right: &Recording<'a>, tables: Tables<'a>) -> Report<'a> {
    let mut report = Report {
        incomparable: comparability::check(left.comparability(), right.comparability()),
        ..Report::default()
    };

    // Reported first: it changes how every L1 difference after it reads.
    if let (Some(a), Some(b)) = (left.adapter().provenance, right.adapter().provenance)
        && !a.matches(&b)
    {
        report.push(Divergence::Provenance {
            left: a.manifest_hash,
            right: b.manifest_hash,
        });
    }

    // Each direction is its own sequence, compared against its counterpart.
    //
    // One merged sequence would align by position across both, and position is
    // not comparable across them: an engine dispatches what it received from
    // the read path and what it sent from the send path, with no ordering
    // between the two. The same traffic replayed twice can interleave
    // differently, and an index-aligned comparison would call that a direction
    // divergence and a frame divergence on every stanza after it.
    //
    // Within a direction the order is the engine's own and is deterministic,
    // which is what makes the comparison mean something.
    let (left_in, left_out) = split_by_direction(left);
    let (right_in, right_out) = split_by_direction(right);

    // A recording carrying a direction its own manifest does not claim is
    // inconsistent with itself, and that is a fault whichever profile is
    // asking. The adapter's `verify` refuses to emit one; a recording can
    // still be assembled by hand or by a build that predates the check.
    for (recording, outbound) in [(left, &left_out), (right, &right_out)] {
        if !outbound.is_empty() && !recording.adapter().has(Capability::L0OutboundObserved) {
            report.push(Divergence::UndeclaredDirection {
                adapter: recording.id(),
                count: outbound.len(),
                direction: Direction::Outbound,
            });
        }
    }

    // Whether an outbound difference is a difference at all depends on who was
    // watching. Two adapters that both observe the outbound half and disagree
    // on how much of it there was have found something; one that cannot see it
    // at all has not — until an engine grew an outbound observation point,
    // none of them could, and counting that as missing stanzas blames an
    // engine for its observer.
    let both_observe = left.adapter().has(Capability::L0OutboundObserved)
        && right.adapter().has(Capability::L0OutboundObserved);
    if left_out.len() != right_out.len() && !both_observe {
        report.push(Divergence::DirectionCoverage {
            only_left: left_out.len(),
            only_right: right_out.len(),
            direction: Direction::Outbound,
        });
    }

    let (left_parser, right_parser) = (Parser::new(tables.left), Parser::new(tables.right));
    for (direction, lefts, rights) in [
        (Direction::Inbound, &left_in, &right_in),
        (Direction::Outbound, &left_out, &right_out),
    ] {
        if lefts.is_empty() && rights.is_empty() {
            continue;
        }
        // The exemption is the *outbound* sequence's alone.
        //
        // Every adapter observes the inbound half — it is what `l0.inbound.tap`
        // means and no engine has ever lacked it — so a recording with none
        // against one with many is a loss, not a difference in reach. Letting
        // the exemption cover inbound too made a total loss of the input
        // produce no divergences at all, and a `Pass`.
        let uncovered = direction == Direction::Outbound && !both_observe;
        if (lefts.is_empty() || rights.is_empty()) && uncovered {
            // One side could not see this direction. Already reported as
            // coverage; comparing an empty sequence against a full one would
            // add a length finding saying the same thing in worse terms.
            continue;
        }
        compare_sequence(
            &mut report,
            (&left_parser, &right_parser),
            left,
            right,
            lefts,
            rights,
        );
    }

    report
}

/// A recording's envelope positions, split by which way each stanza travelled.
///
/// An envelope that will not decode is counted as neither: it is reported as
/// malformed by the comparison that reaches it, and guessing a direction for
/// it would put it opposite a stanza it has nothing to do with.
fn split_by_direction(recording: &Recording<'_>) -> (Vec<usize>, Vec<usize>) {
    let mut inbound = Vec::new();
    let mut outbound = Vec::new();
    for index in 0..recording.len() {
        // An envelope that will not decode goes with the inbound half: it is
        // reported as malformed by the comparison that reaches it, and every
        // recording has an inbound half to reach it from.
        if recording.envelope(index).map(|e| e.flags().direction) == Some(Direction::Outbound) {
            outbound.push(index);
        } else {
            inbound.push(index);
        }
    }
    (inbound, outbound)
}

/// Compare one direction's stanzas, pairwise and in order.
fn compare_sequence<'a>(
    report: &mut Report<'a>,
    parsers: (&Parser<'a>, &Parser<'a>),
    left: &Recording<'a>,
    right: &Recording<'a>,
    lefts: &[usize],
    rights: &[usize],
) {
    if lefts.len() != rights.len() {
        report.push(Divergence::Length {
            left: lefts.len(),
            right: rights.len(),
        });
        // Past a length difference every later position compares unrelated
        // stanzas, so one report beats a hundred that all say the same thing.
        return;
    }

    for (&index, &other) in lefts.iter().zip(rights) {
        report.compared = report.compared.saturating_add(1);

        let (Some(a), Some(b)) = (left.envelope(index), right.envelope(other)) else {
            if left.envelope(index).is_none() {
                report.push(Divergence::MalformedEnvelope {
                    adapter: left.id(),
                    index,
                });
            }
            if right.envelope(other).is_none() {
                report.push(Divergence::MalformedEnvelope {
                    adapter: right.id(),
                    index: other,
                });
            }
            continue;
        };

        if a.frame() != b.frame() {
            report.push(Divergence::Frame {
                index,
                left_len: a.frame().len(),
                right_len: b.frame().len(),
            });
        }

        compare_envelopes(report, index, a, b);
        compare_derivations(report, parsers, index, left, right, a, b);
    }
}

/// Compare everything in the envelope other than the frame bytes.
///
/// The frame is not the whole of L0. Two engines can forward identical bytes
/// and still describe them differently, and the plaintext table is the entire
/// difference between L0-wire and L0-plain — an adapter that quietly stopped
/// producing one would look perfect to a frame-only comparison.
fn compare_envelopes<'a>(
    report: &mut Report<'a>,
    index: usize,
    a: EnvelopeRef<'a>,
    b: EnvelopeRef<'a>,
) {
    // Recorded, not judged. Between two engines this is how they differ by
    // design; between two builds of one adapter it is the newer one having
    // stopped reaching its engine's own buffer. The profile decides.
    if a.flags().frame_origin != b.flags().frame_origin {
        report.push(Divergence::FrameOrigin {
            index,
            degraded: matches!(b.flags().frame_origin, FrameOrigin::ReEncoded),
        });
    }

    if a.flags().direction != b.flags().direction {
        report.push(Divergence::Direction {
            index,
            left: a.flags().direction,
            right: b.flags().direction,
        });
    }

    let mut only_left = 0usize;
    for left in usable(a) {
        match usable_at(b, left.path) {
            Some(right) => {
                if left.payload != right.payload {
                    report.push(Divergence::Plaintext {
                        index,
                        path: left.path,
                        left_len: left.payload.len(),
                        right_len: right.payload.len(),
                    });
                }
            }
            None => only_left = only_left.saturating_add(1),
        }
    }
    let only_right = usable(b)
        .filter(|right| usable_at(a, right.path).is_none())
        .count();

    // Neither side has a payload here, so nothing about the traffic differs.
    // What differs is how much each could say about the failure — which
    // between engines says nothing and between versions is ground lost.
    for left in a.entries().filter(|entry| !entry.status.is_ok()) {
        if let Some(right) = b.entry_at(left.path)
            && !right.status.is_ok()
            && left.status != right.status
        {
            report.push(Divergence::PlaintextStatus {
                index,
                path: left.path,
                left: left.status,
                right: right.status,
            });
        }
    }

    if only_left != 0 || only_right != 0 {
        report.push(Divergence::PlaintextCoverage {
            index,
            only_left,
            only_right,
        });
    }
}

/// The entries carrying plaintext anyone can compare.
///
/// A non-`Ok` status carries no payload by contract, so the only thing two of
/// them could be compared on is which failure each engine reported — and that
/// is coverage, counted separately.
fn usable(envelope: EnvelopeRef<'_>) -> impl Iterator<Item = PlaintextEntry<'_>> + use<'_> {
    envelope.entries().filter(|entry| entry.status.is_ok())
}

fn usable_at<'a>(envelope: EnvelopeRef<'a>, path: NodePath<'_>) -> Option<PlaintextEntry<'a>> {
    envelope.entry_at(path).filter(|entry| entry.status.is_ok())
}

fn compare_derivations<'a>(
    report: &mut Report<'a>,
    parsers: (&Parser<'a>, &Parser<'a>),
    index: usize,
    left: &Recording<'a>,
    right: &Recording<'a>,
    a: EnvelopeRef<'a>,
    b: EnvelopeRef<'a>,
) {
    let parsed = (parsers.0.parse(a.frame()), parsers.1.parse(b.frame()));
    let (Ok(left_node), Ok(right_node)) = parsed else {
        if parsed.0.is_err() {
            report.push(Divergence::UnparsableFrame {
                adapter: left.id(),
                index,
            });
        }
        if parsed.1.is_err() {
            report.push(Divergence::UnparsableFrame {
                adapter: right.id(),
                index,
            });
        }
        return;
    };

    match a.flags().direction {
        Direction::Inbound => {
            compare_derived(report, index, derive(&left_node), derive(&right_node));
        }
        Direction::Outbound => compare_derived(
            report,
            index,
            derive_outgoing(&left_node),
            derive_outgoing(&right_node),
        ),
    }
}

/// What two engines derived from one stanza, judged.
///
/// Generic over the derivation so that the rule lives in one place: inbound and
/// outbound read different grammars and produce different types, and writing
/// the comparison twice is how the two would drift apart on what counts as a
/// finding.
fn compare_derived<T: Derived>(
    report: &mut Report<'_>,
    index: usize,
    left: Result<T, DeriveError>,
    right: Result<T, DeriveError>,
) {
    match (left, right) {
        (Ok(a), Ok(b)) => {
            if !a.semantic_eq(&b) {
                report.push(Divergence::Derivation {
                    index,
                    tag: (a.tag() == b.tag()).then_some(a.tag()),
                });
            }
        }
        // Both engines failing the same way is agreement, not a finding: they
        // are consistent about a stanza neither models.
        (Err(a), Err(b)) if a == b => {}
        (a, b) => report.push(Divergence::DerivationOutcome {
            index,
            left: a.err(),
            right: b.err(),
        }),
    }
}

/// A derived event, whichever direction it travelled.
///
/// Private: the two derivations are separate public types on purpose, and a
/// shared trait in `wa-wire-l1` would invite a caller to write code that does
/// not know which way a stanza went — which is the one thing it must know.
trait Derived {
    fn tag(&self) -> &'static str;
    fn semantic_eq(&self, other: &Self) -> bool;
}

impl Derived for Event<'_> {
    fn tag(&self) -> &'static str {
        Event::tag(self)
    }
    fn semantic_eq(&self, other: &Self) -> bool {
        Event::semantic_eq(self, other)
    }
}

impl Derived for OutgoingEvent<'_> {
    fn tag(&self) -> &'static str {
        OutgoingEvent::tag(self)
    }
    fn semantic_eq(&self, other: &Self) -> bool {
        OutgoingEvent::semantic_eq(self, other)
    }
}

/// Replay one recording against itself.
///
/// Checks the property every other comparison rests on: derivation is a pure
/// function of the stanza. If replaying a capture through the same code twice
/// gave different answers, comparing two engines would mean nothing.
#[must_use]
pub fn replay<'a>(recording: &Recording<'a>, table: TokenTable<'a>) -> Report<'a> {
    // One dictionary by construction: it is the same recording twice.
    compare(recording, recording, Tables::shared(table))
}

/// Derive every stanza in a recording, in order.
///
/// Yields `None` where an envelope did not decode or a frame did not parse, so
/// positions stay aligned with the recording.
pub fn derive_all<'a>(
    recording: &Recording<'a>,
    table: TokenTable<'a>,
) -> impl Iterator<Item = Option<Result<Event<'a>, DeriveError>>> + use<'a> {
    let parser = Parser::new(table);
    recording.envelopes().map(move |envelope| {
        let envelope = envelope?;
        // The inbound derivation, and only for what came in. An outbound
        // `<ack>` satisfies these shapes and means the opposite — the server
        // acknowledging our send against us acknowledging a delivery — so
        // handing one back as an `Event` would be the confident wrong answer
        // this crate takes trouble elsewhere to avoid.
        //
        // `None` for an outbound envelope, which is the same thing this
        // iterator already says about one that will not decode: not derived
        // here, rather than did not derive. A caller wanting the other half
        // wants `derive_outgoing` and a different return type.
        if envelope.flags().direction != Direction::Inbound {
            return None;
        }
        let node = parser.parse(envelope.frame()).ok()?;
        Some(derive(&node))
    })
}

#[cfg(test)]
#[path = "report_tests.rs"]
mod tests;
