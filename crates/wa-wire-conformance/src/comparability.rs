//! Whether two recordings may be compared at all.
//!
//! [`compare`](crate::compare) documents a precondition it cannot check:
//!
//! > Both recordings must be of the *same* stanzas, in the same order.
//!
//! That is fine for a test with both sides in view and useless for a gate
//! running unattended, where the two recordings were made by different builds
//! on different machines and nobody is watching. So the recordings declare it
//! and the comparison checks it (D-078).
//!
//! # Vouched, or checked
//!
//! A recording built in memory carries no declaration: the caller assembled
//! both sides and is asserting they match. A recording read from a container
//! carries one, and it is checked.
//!
//! Mixing the two is refused rather than half-checked. A verified claim
//! compared against an unverified one is unverified overall, and a gate that
//! reported otherwise would be worse than one that reported nothing.

use wa_wire_recording::{ArtifactClass, Integrity, RecordingRef};

use crate::profile::Incomparable;

/// What a recording declares about its own comparability.
///
/// Every field here comes from a critical metadata tag, because every one of
/// them can turn a difference between two recordings into something other than
/// an engine fault.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Comparability<'a> {
    /// The traffic this recording is a replay of. `None` for a capture, which
    /// is what makes a capture an input rather than a result (D-079).
    pub input_digest: Option<&'a [u8]>,
    /// How the recording came to exist.
    pub artifact_class: Option<ArtifactClass>,
    /// For a sanitized recording: which transformation produced it.
    pub transform: Option<(&'a str, &'a str)>,
    /// Which token dictionary the frames were encoded against.
    pub dictionary: Option<&'a str>,
    /// Whether every record the writer intended is present and undamaged.
    pub whole: bool,
    /// Whether a critical metadata tag was skipped for want of understanding.
    pub unknown_critical: bool,
}

impl<'a> Comparability<'a> {
    /// Read what a container declares.
    #[must_use]
    pub fn of(recording: &RecordingRef<'a>) -> Self {
        Self {
            input_digest: recording.input_digest(),
            artifact_class: recording.artifact_class(),
            transform: recording.transform(),
            dictionary: recording.dictionary(),
            whole: matches!(recording.integrity(), Integrity::Complete),
            unknown_critical: recording.unknown_critical_tags() > 0,
        }
    }

    /// Declare comparability directly, for a recording assembled in memory.
    #[must_use]
    pub const fn declared(input_digest: &'a [u8], artifact_class: ArtifactClass) -> Self {
        Self {
            input_digest: Some(input_digest),
            artifact_class: Some(artifact_class),
            transform: None,
            dictionary: None,
            whole: true,
            unknown_critical: false,
        }
    }

    /// Name the token dictionary the frames were encoded against.
    #[must_use]
    pub const fn with_dictionary(mut self, dictionary: &'a str) -> Self {
        self.dictionary = Some(dictionary);
        self
    }

    /// Name the transformation that produced a sanitized recording.
    #[must_use]
    pub const fn with_transform(mut self, identity: &'a str, config_digest: &'a str) -> Self {
        self.transform = Some((identity, config_digest));
        self
    }
}

/// Why this pair may not be compared, if it may not.
///
/// Checked before anything else, because a comparison between unlike things
/// produces findings that read exactly like real ones.
#[must_use]
pub fn check(
    left: Option<Comparability<'_>>,
    right: Option<Comparability<'_>>,
) -> Option<Incomparable> {
    let (left, right) = match (left, right) {
        // Neither declares: the caller assembled both sides and is vouching.
        (None, None) => return None,
        (Some(left), Some(right)) => (left, right),
        // One declares and the other does not. Checking half a claim leaves
        // the pair unchecked, so the pair is refused.
        _ => return Some(Incomparable::UndeclaredInput),
    };

    if left.unknown_critical || right.unknown_critical {
        return Some(Incomparable::UnknownCriticalTag);
    }
    if !left.whole || !right.whole {
        return Some(Incomparable::NotWhole);
    }

    match (left.input_digest, right.input_digest) {
        (Some(a), Some(b)) if a == b => {}
        (Some(_), Some(_)) => return Some(Incomparable::DifferentInput),
        // A capture declares no input, so nothing else can have seen the same
        // traffic. It is an input to a comparison, never a side of one.
        _ => return Some(Incomparable::UndeclaredInput),
    }

    if left.artifact_class != right.artifact_class {
        return Some(Incomparable::DifferentArtifactClass);
    }
    if matches!(left.artifact_class, Some(ArtifactClass::Sanitized))
        && left.transform != right.transform
    {
        return Some(Incomparable::DifferentTransform);
    }
    if left.dictionary != right.dictionary {
        return Some(Incomparable::DifferentDictionary);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate alloc;

    fn replayed(digest: &[u8]) -> Comparability<'_> {
        Comparability::declared(digest, ArtifactClass::Replayed)
    }

    #[test]
    fn two_replays_of_one_input_are_comparable() {
        let a = replayed(b"corpus-1");
        let b = replayed(b"corpus-1");
        assert_eq!(check(Some(a), Some(b)), None);
    }

    #[test]
    fn a_caller_that_assembled_both_sides_is_vouching_for_them() {
        // The existing in-memory path. Nothing declared, nothing to check.
        assert_eq!(check(None, None), None);
    }

    #[test]
    fn half_a_checked_claim_leaves_the_pair_unchecked() {
        let declared = replayed(b"corpus-1");
        assert_eq!(
            check(Some(declared), None),
            Some(Incomparable::UndeclaredInput)
        );
        assert_eq!(
            check(None, Some(declared)),
            Some(Incomparable::UndeclaredInput)
        );
    }

    #[test]
    fn two_recordings_of_different_traffic_are_not_a_regression() {
        // The failure the whole declaration exists to prevent: without it these
        // would compare stanza by stanza and report every difference as an
        // engine fault.
        assert_eq!(
            check(Some(replayed(b"corpus-1")), Some(replayed(b"corpus-2"))),
            Some(Incomparable::DifferentInput)
        );
    }

    #[test]
    fn a_capture_is_an_input_to_a_comparison_never_a_side_of_one() {
        // A capture is a session that happened once, so nothing else can have
        // seen the same traffic (D-079).
        let capture = Comparability {
            artifact_class: Some(ArtifactClass::Captured),
            whole: true,
            ..Comparability::default()
        };
        assert_eq!(
            check(Some(capture), Some(capture)),
            Some(Incomparable::UndeclaredInput)
        );
        assert_eq!(
            check(Some(capture), Some(replayed(b"corpus-1"))),
            Some(Incomparable::UndeclaredInput)
        );
    }

    #[test]
    fn a_sanitized_recording_is_not_comparable_to_the_capture_it_came_from() {
        let sanitized = Comparability::declared(b"corpus-1", ArtifactClass::Sanitized)
            .with_transform("pseudonymise", "sha256:cfg");
        let replayed = replayed(b"corpus-1");
        assert_eq!(
            check(Some(sanitized), Some(replayed)),
            Some(Incomparable::DifferentArtifactClass)
        );
    }

    #[test]
    fn two_sanitized_recordings_must_share_a_transform() {
        // Same source, different rewriting: any difference between them is the
        // transform's, not an engine's.
        let one = Comparability::declared(b"corpus-1", ArtifactClass::Sanitized)
            .with_transform("pseudonymise", "sha256:one");
        let two = Comparability::declared(b"corpus-1", ArtifactClass::Sanitized)
            .with_transform("pseudonymise", "sha256:two");
        assert_eq!(
            check(Some(one), Some(two)),
            Some(Incomparable::DifferentTransform)
        );

        let same = Comparability::declared(b"corpus-1", ArtifactClass::Sanitized)
            .with_transform("pseudonymise", "sha256:one");
        assert_eq!(check(Some(one), Some(same)), None);
    }

    #[test]
    fn a_transform_only_matters_when_the_recording_is_sanitized() {
        // A replay carrying a stray transform tag is not thereby incomparable:
        // the field is meaningless outside its class.
        let plain = replayed(b"corpus-1");
        let odd = replayed(b"corpus-1").with_transform("irrelevant", "sha256:x");
        assert_eq!(check(Some(plain), Some(odd)), None);
    }

    #[test]
    fn different_dictionaries_are_not_an_engine_difference() {
        let one = replayed(b"corpus-1").with_dictionary("whatspec@2.3000.1");
        let two = replayed(b"corpus-1").with_dictionary("whatspec@2.3000.9");
        assert_eq!(
            check(Some(one), Some(two)),
            Some(Incomparable::DifferentDictionary)
        );
        let same = replayed(b"corpus-1").with_dictionary("whatspec@2.3000.1");
        assert_eq!(check(Some(one), Some(same)), None);
    }

    #[test]
    fn a_recording_that_is_not_whole_cannot_be_compared() {
        let cut = Comparability {
            whole: false,
            ..replayed(b"corpus-1")
        };
        assert_eq!(
            check(Some(cut), Some(replayed(b"corpus-1"))),
            Some(Incomparable::NotWhole)
        );
    }

    #[test]
    fn a_critical_tag_nobody_understood_outranks_everything_else() {
        // Checked first: if something load-bearing was skipped, no other
        // conclusion about this pair is trustworthy.
        let opaque = Comparability {
            unknown_critical: true,
            whole: false,
            ..replayed(b"corpus-2")
        };
        assert_eq!(
            check(Some(opaque), Some(replayed(b"corpus-1"))),
            Some(Incomparable::UnknownCriticalTag)
        );
    }

    #[test]
    fn a_container_declares_what_it_carries() {
        use wa_wire_recording::{MetaBuilder, RecordingRef, RecordingWriter};

        let meta = MetaBuilder::new()
            .artifact_class(ArtifactClass::Replayed)
            .expect("class")
            .input_digest(b"corpus-1")
            .expect("input")
            .dictionary("whatspec@2.3000.1")
            .expect("dictionary");
        let bytes = RecordingWriter::new(meta).expect("writer").finish();
        let recording = RecordingRef::decode(&bytes).expect("decodes");

        let read = Comparability::of(&recording);
        assert_eq!(read.input_digest, Some(&b"corpus-1"[..]));
        assert_eq!(read.artifact_class, Some(ArtifactClass::Replayed));
        assert_eq!(read.dictionary, Some("whatspec@2.3000.1"));
        assert!(read.whole);
        assert!(!read.unknown_critical);
        assert_eq!(read.transform, None);

        assert_eq!(check(Some(read), Some(read)), None);
    }

    #[test]
    fn a_truncated_container_reports_itself_as_not_whole() {
        use wa_wire_recording::{MetaBuilder, RecordingRef, RecordingWriter};

        let meta = MetaBuilder::new()
            .artifact_class(ArtifactClass::Replayed)
            .expect("class")
            .input_digest(b"corpus-1")
            .expect("input");
        let mut writer = RecordingWriter::new(meta).expect("writer");
        writer.envelope(b"one").expect("envelope");
        let frozen = writer.as_bytes().to_vec();

        let recording = RecordingRef::decode(&frozen).expect("readable");
        let read = Comparability::of(&recording);
        assert!(!read.whole, "no trailer means the writer was interrupted");
        assert_eq!(
            check(Some(read), Some(read)),
            Some(Incomparable::NotWhole),
            "readable, and still not something to draw a verdict from"
        );
    }

    #[test]
    fn comparability_is_debuggable_and_comparable() {
        let one = replayed(b"corpus-1");
        assert_eq!(one, replayed(b"corpus-1"));
        assert_ne!(one, replayed(b"corpus-2"));
        assert!(!alloc::format!("{one:?}").is_empty());
        assert_eq!(Comparability::default().input_digest, None);
    }
}
