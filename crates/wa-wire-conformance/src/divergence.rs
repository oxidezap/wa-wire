//! What it means for two engines to disagree.
//!
//! Each variant says which layer the disagreement is at and which stanza it is
//! about, because those decide what to do next. A frame difference may be two
//! valid encodings of one stanza; a derivation difference is a bug in exactly
//! one of the two.

use core::fmt;

use wa_wire_l1::DeriveError;

/// Where a disagreement was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Layer {
    /// The frame bytes an engine forwarded.
    ///
    /// A difference here is worth knowing but is not on its own a fault: two
    /// encodings of one stanza are both valid, and what matters is whether they
    /// derive to the same thing.
    L0,
    /// The event derived from the frame.
    ///
    /// A difference here is a fault. The derivation is a pure function of the
    /// stanza, so two engines cannot both be right.
    L1,
}

impl Layer {
    /// Whether a difference at this layer is necessarily a fault.
    #[must_use]
    pub const fn is_fault(self) -> bool {
        matches!(self, Self::L1)
    }

    /// A stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::L0 => "L0",
            Self::L1 => "L1",
        }
    }
}

impl fmt::Display for Layer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// One way two recordings disagreed.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Divergence<'a> {
    /// The recordings hold different numbers of stanzas.
    ///
    /// Reported once and further comparison stops: after a missing stanza every
    /// later index is off by one, and reporting all of them would say the same
    /// thing many times.
    Length {
        /// Stanzas in the first recording.
        left: usize,
        /// Stanzas in the second.
        right: usize,
    },
    /// An envelope did not decode.
    MalformedEnvelope {
        /// Which recording, by adapter id.
        adapter: &'a str,
        /// Which stanza.
        index: usize,
    },
    /// A frame did not parse.
    UnparsableFrame {
        /// Which recording, by adapter id.
        adapter: &'a str,
        /// Which stanza.
        index: usize,
    },
    /// The two engines forwarded different bytes for the same stanza.
    Frame {
        /// Which stanza.
        index: usize,
        /// Bytes in the first recording's frame.
        left_len: usize,
        /// Bytes in the second's.
        right_len: usize,
    },
    /// The two engines derived different events from the same stanza.
    Derivation {
        /// Which stanza.
        index: usize,
        /// The tag both frames carried, when they agreed on it.
        tag: Option<&'a str>,
    },
    /// One engine derived an event where the other reported an error.
    DerivationOutcome {
        /// Which stanza.
        index: usize,
        /// What the first recording produced.
        left: Option<DeriveError>,
        /// What the second produced.
        right: Option<DeriveError>,
    },
    /// The recordings came from different spec builds, so any L1 difference may
    /// be explained by that rather than by either engine.
    ///
    /// Reported first, because it changes how everything after it reads.
    Provenance {
        /// The first recording's derivation digest.
        left: &'a str,
        /// The second's.
        right: &'a str,
    },
}

impl Divergence<'_> {
    /// Which layer this is about.
    #[must_use]
    pub const fn layer(&self) -> Layer {
        match self {
            Self::Frame { .. }
            | Self::MalformedEnvelope { .. }
            | Self::UnparsableFrame { .. }
            | Self::Length { .. } => Layer::L0,
            Self::Derivation { .. } | Self::DerivationOutcome { .. } | Self::Provenance { .. } => {
                Layer::L1
            }
        }
    }

    /// Whether this is necessarily a fault in one of the engines.
    ///
    /// A frame difference is not: two encodings of one stanza are both valid.
    /// A provenance difference is not either — it says the comparison itself
    /// was between unlike things.
    #[must_use]
    pub const fn is_fault(&self) -> bool {
        match self {
            // A derivation difference means one engine is wrong; a frame that
            // will not decode or parse means one engine emitted something
            // unusable. Both need someone to look.
            Self::Derivation { .. }
            | Self::DerivationOutcome { .. }
            | Self::MalformedEnvelope { .. }
            | Self::UnparsableFrame { .. } => true,
            Self::Frame { .. } | Self::Length { .. } | Self::Provenance { .. } => false,
        }
    }

    /// Which stanza this is about, when it is about one.
    #[must_use]
    pub const fn index(&self) -> Option<usize> {
        match self {
            Self::MalformedEnvelope { index, .. }
            | Self::UnparsableFrame { index, .. }
            | Self::Frame { index, .. }
            | Self::Derivation { index, .. }
            | Self::DerivationOutcome { index, .. } => Some(*index),
            Self::Length { .. } | Self::Provenance { .. } => None,
        }
    }
}

impl fmt::Display for Divergence<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Length { left, right } => write!(
                f,
                "[L0] recordings differ in length: {left} stanza(s) against {right}"
            ),
            Self::MalformedEnvelope { adapter, index } => write!(
                f,
                "[L0] stanza {index}: envelope from `{adapter}` does not decode"
            ),
            Self::UnparsableFrame { adapter, index } => write!(
                f,
                "[L0] stanza {index}: frame from `{adapter}` does not parse"
            ),
            Self::Frame {
                index,
                left_len,
                right_len,
            } => write!(
                f,
                "[L0] stanza {index}: frames differ ({left_len} bytes against {right_len}) \
                 — not a fault on its own"
            ),
            Self::Derivation { index, tag } => match tag {
                Some(tag) => write!(f, "[L1] stanza {index} <{tag}>: derived events differ"),
                None => write!(f, "[L1] stanza {index}: derived events differ"),
            },
            Self::DerivationOutcome { index, left, right } => {
                write!(f, "[L1] stanza {index}: ")?;
                match (left, right) {
                    (None, Some(error)) => write!(f, "one derived, the other failed: {error}"),
                    (Some(error), None) => write!(f, "one failed: {error}, the other derived"),
                    (Some(a), Some(b)) => write!(f, "both failed, differently: {a} against {b}"),
                    (None, None) => f.write_str("outcomes differ"),
                }
            }
            Self::Provenance { left, right } => write!(
                f,
                "[L1] recordings derive from different specs ({left} against {right}); \
                 any L1 difference may be that rather than an engine"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate alloc;
    use alloc::string::ToString;

    #[test]
    fn layers_say_whether_a_difference_is_a_fault() {
        assert!(!Layer::L0.is_fault(), "two encodings can both be valid");
        assert!(Layer::L1.is_fault(), "a pure function has one answer");
        assert_eq!(Layer::L0.name(), "L0");
        assert_eq!(Layer::L1.to_string(), "L1");
    }

    #[test]
    fn each_divergence_reports_its_layer() {
        assert_eq!(Divergence::Length { left: 1, right: 2 }.layer(), Layer::L0);
        assert_eq!(
            Divergence::Frame {
                index: 0,
                left_len: 1,
                right_len: 2
            }
            .layer(),
            Layer::L0
        );
        assert_eq!(
            Divergence::Derivation {
                index: 0,
                tag: Some("receipt")
            }
            .layer(),
            Layer::L1
        );
        assert_eq!(
            Divergence::Provenance {
                left: "a",
                right: "b"
            }
            .layer(),
            Layer::L1
        );
        assert_eq!(
            Divergence::MalformedEnvelope {
                adapter: "x",
                index: 0
            }
            .layer(),
            Layer::L0
        );
        assert_eq!(
            Divergence::UnparsableFrame {
                adapter: "x",
                index: 0
            }
            .layer(),
            Layer::L0
        );
        assert_eq!(
            Divergence::DerivationOutcome {
                index: 0,
                left: None,
                right: None
            }
            .layer(),
            Layer::L1
        );
    }

    #[test]
    fn only_some_divergences_are_faults() {
        // A frame difference means two valid encodings; a derivation difference
        // means one engine is wrong.
        assert!(
            !Divergence::Frame {
                index: 0,
                left_len: 1,
                right_len: 2
            }
            .is_fault()
        );
        assert!(!Divergence::Length { left: 1, right: 2 }.is_fault());
        assert!(
            !Divergence::Provenance {
                left: "a",
                right: "b"
            }
            .is_fault()
        );

        assert!(
            Divergence::Derivation {
                index: 0,
                tag: None
            }
            .is_fault()
        );
        assert!(
            Divergence::DerivationOutcome {
                index: 0,
                left: None,
                right: None
            }
            .is_fault()
        );
        assert!(
            Divergence::MalformedEnvelope {
                adapter: "x",
                index: 0
            }
            .is_fault()
        );
        assert!(
            Divergence::UnparsableFrame {
                adapter: "x",
                index: 0
            }
            .is_fault()
        );
    }

    #[test]
    fn a_divergence_names_the_stanza_when_it_is_about_one() {
        assert_eq!(
            Divergence::Frame {
                index: 7,
                left_len: 1,
                right_len: 2
            }
            .index(),
            Some(7)
        );
        assert_eq!(
            Divergence::Derivation {
                index: 3,
                tag: None
            }
            .index(),
            Some(3)
        );
        assert_eq!(
            Divergence::MalformedEnvelope {
                adapter: "a",
                index: 2
            }
            .index(),
            Some(2)
        );
        assert_eq!(
            Divergence::UnparsableFrame {
                adapter: "a",
                index: 1
            }
            .index(),
            Some(1)
        );
        assert_eq!(
            Divergence::DerivationOutcome {
                index: 4,
                left: None,
                right: None
            }
            .index(),
            Some(4)
        );

        // These are about the pair, not a stanza.
        assert_eq!(Divergence::Length { left: 1, right: 2 }.index(), None);
        assert_eq!(
            Divergence::Provenance {
                left: "a",
                right: "b"
            }
            .index(),
            None
        );
    }

    #[test]
    fn every_divergence_renders_its_layer_and_detail() {
        let cases: [(Divergence<'_>, &[&str]); 8] = [
            (Divergence::Length { left: 3, right: 5 }, &["L0", "3", "5"]),
            (
                Divergence::MalformedEnvelope {
                    adapter: "zapo",
                    index: 2,
                },
                &["L0", "zapo", "2"],
            ),
            (
                Divergence::UnparsableFrame {
                    adapter: "baileys",
                    index: 4,
                },
                &["L0", "baileys", "4"],
            ),
            (
                Divergence::Frame {
                    index: 1,
                    left_len: 10,
                    right_len: 12,
                },
                &["L0", "1", "10", "12", "not a fault"],
            ),
            (
                Divergence::Derivation {
                    index: 6,
                    tag: Some("receipt"),
                },
                &["L1", "6", "receipt"],
            ),
            (
                Divergence::Derivation {
                    index: 6,
                    tag: None,
                },
                &["L1", "6"],
            ),
            (
                Divergence::DerivationOutcome {
                    index: 8,
                    left: None,
                    right: Some(wa_wire_l1::DeriveError::UnknownStanza),
                },
                &["L1", "8", "the other failed"],
            ),
            (
                Divergence::Provenance {
                    left: "sha256:a",
                    right: "sha256:b",
                },
                &["L1", "sha256:a", "sha256:b"],
            ),
        ];
        for (divergence, fragments) in cases {
            let text = divergence.to_string();
            for fragment in fragments {
                assert!(text.contains(fragment), "{text:?} lacks {fragment:?}");
            }
        }
    }

    #[test]
    fn both_failure_directions_render_distinctly() {
        let one_failed = Divergence::DerivationOutcome {
            index: 0,
            left: Some(wa_wire_l1::DeriveError::UnknownStanza),
            right: None,
        }
        .to_string();
        assert!(one_failed.contains("one failed"));

        let both = Divergence::DerivationOutcome {
            index: 0,
            left: Some(wa_wire_l1::DeriveError::UnknownStanza),
            right: Some(wa_wire_l1::DeriveError::MissingAttr { key: "id" }),
        }
        .to_string();
        assert!(both.contains("both failed"));

        let neither = Divergence::DerivationOutcome {
            index: 0,
            left: None,
            right: None,
        }
        .to_string();
        assert!(neither.contains("outcomes differ"));
    }

    #[test]
    fn divergences_are_comparable() {
        assert_eq!(
            Divergence::Length { left: 1, right: 2 },
            Divergence::Length { left: 1, right: 2 }
        );
        assert_ne!(
            Divergence::Length { left: 1, right: 2 },
            Divergence::Length { left: 1, right: 3 }
        );
        assert!(!alloc::format!("{:?}", Layer::L0).is_empty());
    }
}
