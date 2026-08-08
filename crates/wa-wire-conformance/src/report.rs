//! Running the comparison.

extern crate alloc;
use alloc::vec::Vec;

use wa_wire_codec::{Parser, TokenTable};
use wa_wire_contract::{Direction, EnvelopeRef, FrameOrigin, NodePath, PlaintextEntry};
use wa_wire_l1::{DeriveError, Event, derive};

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

    if left.len() != right.len() {
        report.push(Divergence::Length {
            left: left.len(),
            right: right.len(),
        });
        // Past a length difference every later index compares unrelated
        // stanzas, so one report beats a hundred that all say the same thing.
        return report;
    }

    let (left_parser, right_parser) = (Parser::new(tables.left), Parser::new(tables.right));
    for index in 0..left.len() {
        report.compared = report.compared.saturating_add(1);

        let (Some(a), Some(b)) = (left.envelope(index), right.envelope(index)) else {
            if left.envelope(index).is_none() {
                report.push(Divergence::MalformedEnvelope {
                    adapter: left.id(),
                    index,
                });
            }
            if right.envelope(index).is_none() {
                report.push(Divergence::MalformedEnvelope {
                    adapter: right.id(),
                    index,
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

        compare_envelopes(&mut report, index, a, b);

        // Outbound stanzas are compared at L0 and never derived.
        //
        // The derivation comes from whatspec's `incoming` domain, which records
        // how WA Web parses what the *server* sends. An outbound stanza is a
        // different grammar wearing the same tags: an `<ack>` inbound is the
        // server acknowledging our send, an `<ack>` outbound is us
        // acknowledging a delivery. Nothing in `derive` is told which way a
        // stanza travelled, so an outbound one does not fail to derive — it
        // derives confidently and wrongly, and two engines agreeing on a wrong
        // reading would report as agreement.
        //
        // Deriving these needs whatspec's request-side domains, which nothing
        // here generates from yet. Until then the bytes are the claim.
        if a.flags().direction == Direction::Inbound {
            compare_derivations(
                &mut report,
                (&left_parser, &right_parser),
                index,
                left,
                right,
                a,
                b,
            );
        }
    }

    report
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

    match (derive(&left_node), derive(&right_node)) {
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
        let node = parser.parse(envelope?.frame()).ok()?;
        Some(derive(&node))
    })
}

#[cfg(test)]
#[path = "report_tests.rs"]
mod tests;
