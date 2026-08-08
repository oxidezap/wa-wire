//! Which findings count as failures, and who decides.
//!
//! RFC-005 was written for one question: do two engines agree? The recording
//! container makes a second one mechanical — did this version regress against
//! that one? — and the two want **opposite answers from the same evidence**.
//!
//! Between two engines, differing frame bytes are two valid encodings of one
//! stanza. Between two versions of one engine, they are the encoder changing
//! under you. So faultiness cannot be a property of the divergence; the
//! comparator records facts and the profile judges them (D-080).

use core::fmt;

use crate::divergence::Divergence;

/// What a comparison is being asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ComparisonProfile {
    /// Two engines, one input: do they mean the same thing?
    ///
    /// Symmetric — neither side is the reference. Tolerant of everything that
    /// legitimately differs between implementations: how a stanza is encoded,
    /// whether an adapter reaches its engine's own buffer, and how much of the
    /// decryption it can observe.
    #[default]
    Interop,
    /// One engine, two builds: did the newer one lose anything?
    ///
    /// **Directional**: `left` is the baseline and `right` is the candidate.
    /// Intolerant of everything `Interop` allows, because the same code
    /// producing different output is the definition of a regression — and
    /// generous in the other direction, since a candidate that observes *more*
    /// has improved rather than diverged.
    Regression,
}

impl ComparisonProfile {
    /// Whether this divergence fails the run under this profile.
    // One arm per finding, even where two are judged alike: each carries the
    // reason it is judged that way, and collapsing them would leave the table
    // shorter and the reasoning nowhere.
    #[allow(clippy::match_same_arms)]
    #[must_use]
    pub const fn is_failure(self, divergence: &Divergence<'_>) -> bool {
        match divergence {
            // One engine lost, invented or mislabelled a stanza; one emitted
            // something unusable; or the derivation — a pure function — gave
            // two answers. None of these is ever tolerable.
            Divergence::Length { .. }
            | Divergence::Direction { .. }
            | Divergence::Plaintext { .. }
            | Divergence::MalformedEnvelope { .. }
            | Divergence::UnparsableFrame { .. }
            | Divergence::Derivation { .. }
            | Divergence::DerivationOutcome { .. } => true,

            // Two encodings of one stanza are both valid; the same encoder
            // producing different bytes is not.
            Divergence::Frame { .. } => matches!(self, Self::Regression),

            // Coverage lost by the candidate is a regression. Coverage gained
            // is an improvement, and an improvement must not fail a gate.
            Divergence::PlaintextCoverage { only_left, .. } => {
                matches!(self, Self::Regression) && *only_left > 0
            }

            // An adapter that stopped reaching its engine's own buffer has
            // degraded; two adapters differing on it were built differently.
            Divergence::FrameOrigin { degraded, .. } => {
                matches!(self, Self::Regression) && *degraded
            }

            // Between engines, which failure each reports says nothing.
            // Between versions, an adapter that stopped knowing *why* a
            // payload was missing has lost diagnostic ground.
            Divergence::PlaintextStatus { .. } => matches!(self, Self::Regression),

            // Never a failure: it says the comparison itself was between
            // unlike things, which `Verdict::Incomparable` reports instead.
            Divergence::Provenance { .. } => false,
        }
    }

    /// Whether this divergence is the candidate doing better than the baseline.
    ///
    /// Only meaningful under [`Regression`](Self::Regression); an improvement
    /// is reported and passes.
    #[must_use]
    pub const fn is_improvement(self, divergence: &Divergence<'_>) -> bool {
        match divergence {
            Divergence::PlaintextCoverage {
                only_left,
                only_right,
                ..
            } => matches!(self, Self::Regression) && *only_left == 0 && *only_right > 0,
            Divergence::FrameOrigin { degraded, .. } => {
                matches!(self, Self::Regression) && !*degraded
            }
            _ => false,
        }
    }

    /// A stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Interop => "interop",
            Self::Regression => "regression",
        }
    }
}

impl fmt::Display for ComparisonProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// What a comparison concluded.
///
/// Three-valued on purpose. A boolean would have to fold "these were unlike
/// things" into one of the other two, and folding it into agreement — which is
/// what a report whose provenance mismatch is merely a warning does — reports a
/// green result from a comparison that never happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Verdict {
    /// Nothing failed under this profile.
    Pass,
    /// At least one finding fails under this profile.
    Fail,
    /// The two recordings may not be compared at all.
    Incomparable(Incomparable),
}

impl Verdict {
    /// Whether the run passed.
    ///
    /// `Incomparable` is **not** a pass: nothing was established.
    #[must_use]
    pub const fn is_pass(self) -> bool {
        matches!(self, Self::Pass)
    }

    /// Why the recordings could not be compared, if that is the verdict.
    #[must_use]
    pub const fn incomparable(self) -> Option<Incomparable> {
        match self {
            Self::Incomparable(reason) => Some(reason),
            _ => None,
        }
    }
}

impl fmt::Display for Verdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pass => f.write_str("pass"),
            Self::Fail => f.write_str("fail"),
            Self::Incomparable(reason) => write!(f, "incomparable: {reason}"),
        }
    }
}

/// Why two recordings may not be compared.
///
/// Every one of these makes a verdict meaningless rather than negative, which
/// is why they are kept apart from failure. Reporting "these disagree" when the
/// truth is "these were never comparable" is the error the whole container
/// exists to prevent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Incomparable {
    /// One or both sides do not say what traffic they are a replay of, so
    /// nothing establishes that they saw the same input (D-079).
    UndeclaredInput,
    /// Both declare an input, and they are different recordings of different
    /// traffic.
    DifferentInput,
    /// One is a capture and the other a replay, or one is sanitized and the
    /// other is not.
    DifferentArtifactClass,
    /// Both are sanitized, by different transformations.
    DifferentTransform,
    /// The frames were encoded against different token dictionaries, so a
    /// difference in bytes may be the dictionary rather than the engine.
    DifferentDictionary,
    /// The two agree on a dictionary the host does not have.
    ///
    /// Reported by a host rather than by [`check`], which cannot know what
    /// tables are available where it runs. Attempting the comparison anyway
    /// would parse the frames with the wrong dictionary and attribute the
    /// result to an engine.
    ///
    /// [`check`]: crate::comparability::check
    UnresolvableDictionary,
    /// A recording carries a critical metadata tag this build cannot read, so
    /// something load-bearing was skipped (D-077).
    UnknownCriticalTag,
    /// A recording is missing records: interrupted, or damaged.
    NotWhole,
}

impl Incomparable {
    /// A stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::UndeclaredInput => "neither side declares its input",
            Self::DifferentInput => "different input traffic",
            Self::DifferentArtifactClass => "different artifact classes",
            Self::DifferentTransform => "different sanitizing transforms",
            Self::DifferentDictionary => "different token dictionaries",
            Self::UnresolvableDictionary => "a token dictionary this host does not have",
            Self::UnknownCriticalTag => "a critical metadata tag was not understood",
            Self::NotWhole => "a recording is truncated or damaged",
        }
    }
}

impl fmt::Display for Incomparable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg(test)]
#[path = "profile_tests.rs"]
mod tests;
