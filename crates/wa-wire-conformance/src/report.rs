//! Running the comparison.

extern crate alloc;
use alloc::vec::Vec;

use wa_wire_codec::{Parser, TokenTable};
use wa_wire_contract::EnvelopeRef;
use wa_wire_l1::{DeriveError, Event, derive};

use crate::divergence::Divergence;
use crate::recording::Recording;

/// What a comparison found.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Report<'a> {
    divergences: Vec<Divergence<'a>>,
    compared: usize,
}

impl<'a> Report<'a> {
    /// Whether the recordings agreed on everything that counts.
    ///
    /// A frame difference does not stop this being true: two encodings of one
    /// stanza are both valid, and what matters is whether they mean the same.
    #[must_use]
    pub fn agrees(&self) -> bool {
        !self.divergences.iter().any(Divergence::is_fault)
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

    /// Only the ones that are necessarily a fault in one engine.
    pub fn faults(&self) -> impl Iterator<Item = &Divergence<'a>> {
        self.divergences.iter().filter(|d| d.is_fault())
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
/// engines fed one capture, not two engines watching two sessions.
#[must_use]
pub fn compare<'a>(
    left: &Recording<'a>,
    right: &Recording<'a>,
    table: TokenTable<'a>,
) -> Report<'a> {
    let mut report = Report::default();

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

    let parser = Parser::new(table);
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

        compare_derivations(&mut report, &parser, index, left, right, a, b);
    }

    report
}

fn compare_derivations<'a>(
    report: &mut Report<'a>,
    parser: &Parser<'a>,
    index: usize,
    left: &Recording<'a>,
    right: &Recording<'a>,
    a: EnvelopeRef<'a>,
    b: EnvelopeRef<'a>,
) {
    let parsed = (parser.parse(a.frame()), parser.parse(b.frame()));
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
    compare(recording, recording, table)
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
