//! What it means for two engines to disagree.
//!
//! Each variant says which layer the disagreement is at and which stanza it is
//! about, because those decide what to do next. A frame difference may be two
//! valid encodings of one stanza; a derivation difference is a bug in exactly
//! one of the two.

use core::fmt;

use wa_wire_contract::{Direction, NodePath, PlaintextStatus};
use wa_wire_l1::DeriveError;

/// Where a disagreement was found.
///
/// The layer says what kind of thing disagreed, not how bad it is: some L0
/// differences are two valid encodings of one stanza and some are one engine
/// losing traffic. Whether a finding is a fault is
/// [`ComparisonProfile::is_failure`], per divergence and per profile.
///
/// [`ComparisonProfile::is_failure`]: crate::ComparisonProfile::is_failure
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Layer {
    /// The envelope an engine forwarded: its frame bytes, its direction, and
    /// the plaintext it carried.
    L0,
    /// The event derived from the frame.
    ///
    /// A difference here is always a fault. The derivation is a pure function
    /// of the stanza, so two engines cannot both be right.
    L1,
}

impl Layer {
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
    /// A fault, though it does not say whose: both recordings are of the same
    /// traffic, so one engine dropped a stanza or invented one. Which of the
    /// two needs a human, but the run must not pass either way.
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
    /// The two engines disagree about which way the same stanza travelled.
    ///
    /// A fault: direction is a property of the stanza, not of the engine, so
    /// one of them mislabelled it and a consumer reading either would be
    /// wrong about the other.
    Direction {
        /// Which stanza.
        index: usize,
        /// What the first recording said.
        left: Direction,
        /// What the second said.
        right: Direction,
    },
    /// Both engines decrypted the same `<enc>` and got different bytes.
    ///
    /// A fault, and the sharpest one the L0 comparison can make: the two
    /// agreed the payload was usable, so this is not a limitation of either
    /// adapter's observation.
    Plaintext {
        /// Which stanza.
        index: usize,
        /// The node both entries addressed.
        path: NodePath<'a>,
        /// Bytes in the first recording's payload.
        left_len: usize,
        /// Bytes in the second's.
        right_len: usize,
    },
    /// The two describe the same frame's bytes as coming from different
    /// places: one verbatim from its engine's decoder, one re-encoded.
    ///
    /// Recorded rather than judged. Between two engines this is how they
    /// differ by design and says nothing; between two builds of one adapter,
    /// `degraded` means it stopped reaching its own buffer.
    FrameOrigin {
        /// Which stanza.
        index: usize,
        /// Whether the *second* recording is the re-encoded one — a loss, if
        /// the two are builds of the same adapter.
        degraded: bool,
    },
    /// Both engines failed to produce plaintext for a node, and reported
    /// different reasons.
    ///
    /// Neither carries a payload, so nothing about the traffic differs. What
    /// differs is how much each could say about the failure, which between
    /// versions of one adapter is diagnostic ground lost or gained.
    PlaintextStatus {
        /// Which stanza.
        index: usize,
        /// The node both entries addressed.
        path: NodePath<'a>,
        /// What the first recording reported.
        left: PlaintextStatus,
        /// What the second reported.
        right: PlaintextStatus,
    },
    /// One engine reported usable plaintext for a node the other did not.
    ///
    /// Not a fault. How much an adapter can observe is a property of the
    /// adapter, which is why [`PlaintextStatus::Unobserved`] exists and why
    /// D-055 leaves a fan-out stanza with no table at all. Worth reporting
    /// because it says how much of the L0-plain comparison actually ran.
    ///
    /// [`PlaintextStatus::Unobserved`]: wa_wire_contract::PlaintextStatus::Unobserved
    PlaintextCoverage {
        /// Which stanza.
        index: usize,
        /// Nodes only the first recording has usable plaintext for.
        only_left: usize,
        /// Nodes only the second has.
        only_right: usize,
    },
    /// A recording carries stanzas travelling a way its manifest does not
    /// claim to observe.
    ///
    /// Inconsistent with itself rather than with the other recording, so it is
    /// a fault under every profile: nothing downstream can tell whether the
    /// records are real or an artefact of however the file was assembled.
    UndeclaredDirection {
        /// Which adapter said one thing and recorded another.
        adapter: &'a str,
        /// How many such stanzas it carries.
        count: usize,
        /// The direction it did not claim.
        direction: wa_wire_contract::Direction,
    },
    /// One recording observes a direction the other does not.
    ///
    /// Not a fault in the engine: an adapter can only report what its engine
    /// exposes, and until one of them grew an outbound observation point every
    /// recording held the inbound half of a session. Counting the difference
    /// as missing stanzas would blame an engine for its observer.
    DirectionCoverage {
        /// Stanzas only the first recording observed travelling this way.
        only_left: usize,
        /// Stanzas only the second observed.
        only_right: usize,
        /// The direction neither could be compared on.
        direction: wa_wire_contract::Direction,
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
            | Self::Length { .. }
            | Self::Direction { .. }
            | Self::Plaintext { .. }
            | Self::FrameOrigin { .. }
            | Self::PlaintextStatus { .. }
            | Self::PlaintextCoverage { .. }
            | Self::DirectionCoverage { .. }
            | Self::UndeclaredDirection { .. } => Layer::L0,
            Self::Derivation { .. } | Self::DerivationOutcome { .. } | Self::Provenance { .. } => {
                Layer::L1
            }
        }
    }

    /// Which stanza this is about, when it is about one.
    #[must_use]
    pub const fn index(&self) -> Option<usize> {
        match self {
            Self::MalformedEnvelope { index, .. }
            | Self::UnparsableFrame { index, .. }
            | Self::Frame { index, .. }
            | Self::Direction { index, .. }
            | Self::Plaintext { index, .. }
            | Self::FrameOrigin { index, .. }
            | Self::PlaintextStatus { index, .. }
            | Self::PlaintextCoverage { index, .. }
            | Self::Derivation { index, .. }
            | Self::DerivationOutcome { index, .. } => Some(*index),
            Self::Length { .. }
            | Self::Provenance { .. }
            | Self::DirectionCoverage { .. }
            | Self::UndeclaredDirection { .. } => None,
        }
    }
}

impl Divergence<'_> {
    /// The two coverage findings, split out so the main `Display` stays under
    /// the line budget. Both say the same kind of thing — what one side could
    /// see and the other could not — and neither is about a stanza.
    fn fmt_coverage(&self, f: &mut fmt::Formatter<'_>) -> Option<fmt::Result> {
        match self {
            Self::UndeclaredDirection {
                adapter,
                count,
                direction,
            } => Some(write!(
                f,
                "[L0] {adapter} recorded {count} {direction} stanza(s) without declaring it \
                 observes them"
            )),
            Self::DirectionCoverage {
                only_left,
                only_right,
                direction,
            } => Some(write!(
                f,
                "[L0] {direction} coverage differs: {only_left} stanza(s) observed only by one \
                 side, {only_right} only by the other — the observer's reach, not the engine's"
            )),
            _ => None,
        }
    }
}

impl fmt::Display for Divergence<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(done) = self.fmt_coverage(f) {
            return done;
        }
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
            Self::Direction { index, left, right } => write!(
                f,
                "[L0] stanza {index}: directions differ ({left:?} against {right:?})"
            ),
            Self::Plaintext {
                index,
                path,
                left_len,
                right_len,
            } => write!(
                f,
                "[L0] stanza {index} at {path}: both decrypted, and the plaintexts differ \
                 ({left_len} bytes against {right_len})"
            ),
            Self::FrameOrigin { index, degraded } => write!(
                f,
                "[L0] stanza {index}: frame origins differ ({}) — only a loss if the two are \
                 builds of one adapter",
                if *degraded {
                    "the second re-encoded"
                } else {
                    "the first re-encoded"
                }
            ),
            Self::PlaintextStatus {
                index,
                path,
                left,
                right,
            } => write!(
                f,
                "[L0] stanza {index} at {path}: neither decrypted, and they say so differently \
                 ({left} against {right})"
            ),
            Self::PlaintextCoverage {
                index,
                only_left,
                only_right,
            } => write!(
                f,
                "[L0] stanza {index}: plaintext coverage differs ({only_left} node(s) only on \
                 one side, {only_right} only on the other) — not a fault on its own"
            ),
            // Handled by `fmt_coverage` above.
            Self::UndeclaredDirection { .. } | Self::DirectionCoverage { .. } => Ok(()),
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

    fn path(components: &[u16]) -> alloc::vec::Vec<u8> {
        components.iter().flat_map(|c| c.to_le_bytes()).collect()
    }

    #[test]
    fn layers_name_themselves() {
        assert_eq!(Layer::L0.name(), "L0");
        assert_eq!(Layer::L1.to_string(), "L1");
        assert_ne!(Layer::L0, Layer::L1);
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
        let p = path(&[0]);
        assert_eq!(
            Divergence::Direction {
                index: 0,
                left: Direction::Inbound,
                right: Direction::Outbound
            }
            .layer(),
            Layer::L0
        );
        assert_eq!(
            Divergence::Plaintext {
                index: 0,
                path: NodePath::from_le_bytes(&p),
                left_len: 1,
                right_len: 2
            }
            .layer(),
            Layer::L0
        );
        assert_eq!(
            Divergence::PlaintextCoverage {
                index: 0,
                only_left: 1,
                only_right: 0
            }
            .layer(),
            Layer::L0
        );
        assert_eq!(
            Divergence::FrameOrigin {
                index: 0,
                degraded: false
            }
            .layer(),
            Layer::L0
        );
        assert_eq!(
            Divergence::PlaintextStatus {
                index: 0,
                path: NodePath::from_le_bytes(&p),
                left: PlaintextStatus::Unsupported,
                right: PlaintextStatus::Unobserved
            }
            .layer(),
            Layer::L0
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

        let p = path(&[0]);
        assert_eq!(
            Divergence::Direction {
                index: 9,
                left: Direction::Inbound,
                right: Direction::Outbound
            }
            .index(),
            Some(9)
        );
        assert_eq!(
            Divergence::Plaintext {
                index: 10,
                path: NodePath::from_le_bytes(&p),
                left_len: 1,
                right_len: 2
            }
            .index(),
            Some(10)
        );
        assert_eq!(
            Divergence::PlaintextCoverage {
                index: 11,
                only_left: 1,
                only_right: 0
            }
            .index(),
            Some(11)
        );
        assert_eq!(
            Divergence::FrameOrigin {
                index: 12,
                degraded: true
            }
            .index(),
            Some(12)
        );
        assert_eq!(
            Divergence::PlaintextStatus {
                index: 13,
                path: NodePath::from_le_bytes(&p),
                left: PlaintextStatus::DecryptFailed,
                right: PlaintextStatus::Unobserved
            }
            .index(),
            Some(13)
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
    fn every_l0_divergence_renders_its_layer_and_detail() {
        let p = path(&[2, 1]);
        let cases: [(Divergence<'_>, &[&str]); 10] = [
            (
                Divergence::FrameOrigin {
                    index: 2,
                    degraded: true,
                },
                &["L0", "2", "the second re-encoded"],
            ),
            (
                Divergence::FrameOrigin {
                    index: 2,
                    degraded: false,
                },
                &["L0", "2", "the first re-encoded"],
            ),
            (
                Divergence::PlaintextStatus {
                    index: 4,
                    path: NodePath::from_le_bytes(&p),
                    left: PlaintextStatus::DecryptFailed,
                    right: PlaintextStatus::Unobserved,
                },
                &["L0", "4", "/2/1", "decrypt-failed", "unobserved"],
            ),
            (Divergence::Length { left: 3, right: 5 }, &["L0", "3", "5"]),
            (
                Divergence::Direction {
                    index: 5,
                    left: Direction::Inbound,
                    right: Direction::Outbound,
                },
                &["L0", "5", "Inbound", "Outbound"],
            ),
            (
                Divergence::Plaintext {
                    index: 9,
                    path: NodePath::from_le_bytes(&p),
                    left_len: 40,
                    right_len: 41,
                },
                &["L0", "9", "/2/1", "40", "41"],
            ),
            (
                Divergence::PlaintextCoverage {
                    index: 3,
                    only_left: 2,
                    only_right: 0,
                },
                &["L0", "3", "2", "not a fault"],
            ),
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
        ];
        assert_renders(cases);
    }

    #[test]
    fn every_l1_divergence_renders_its_layer_and_detail() {
        let cases: [(Divergence<'_>, &[&str]); 4] = [
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
        assert_renders(cases);
    }

    /// Every rendering has to name its layer and every value it carries: a
    /// report is the only place a divergence is ever read from.
    fn assert_renders<const N: usize>(cases: [(Divergence<'_>, &[&str]); N]) {
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
