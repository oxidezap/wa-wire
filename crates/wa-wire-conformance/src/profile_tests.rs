//! The matrix: the same finding, judged twice.

use super::*;
extern crate alloc;
use alloc::string::ToString;
use alloc::vec::Vec;

use wa_wire_contract::{Direction, NodePath, PlaintextStatus};
use wa_wire_l1::DeriveError;

use crate::divergence::Divergence;

const INTEROP: ComparisonProfile = ComparisonProfile::Interop;
const REGRESSION: ComparisonProfile = ComparisonProfile::Regression;

fn path_bytes() -> Vec<u8> {
    alloc::vec![0, 0]
}

fn path(bytes: &[u8]) -> NodePath<'_> {
    NodePath::from_le_bytes(bytes)
}

// -- always a failure -------------------------------------------------------

#[test]
fn what_no_profile_tolerates() {
    // One engine lost, invented or mislabelled a stanza; one emitted something
    // unusable; or the derivation gave two answers for one input. None of
    // these can be explained by two implementations differing legitimately.
    let bytes = path_bytes();
    let always: [Divergence<'_>; 7] = [
        Divergence::Length { left: 3, right: 2 },
        Divergence::Direction {
            index: 0,
            left: Direction::Inbound,
            right: Direction::Outbound,
        },
        Divergence::Plaintext {
            index: 0,
            path: path(&bytes),
            left_len: 4,
            right_len: 5,
        },
        Divergence::MalformedEnvelope {
            adapter: "x",
            index: 0,
        },
        Divergence::UnparsableFrame {
            adapter: "x",
            index: 0,
        },
        Divergence::Derivation {
            index: 0,
            tag: None,
        },
        Divergence::DerivationOutcome {
            index: 0,
            left: None,
            right: Some(DeriveError::UnknownStanza),
        },
    ];

    for divergence in &always {
        assert!(INTEROP.is_failure(divergence), "{divergence}");
        assert!(REGRESSION.is_failure(divergence), "{divergence}");
        assert!(!INTEROP.is_improvement(divergence));
        assert!(!REGRESSION.is_improvement(divergence));
    }
}

// -- the same evidence, opposite verdicts -----------------------------------

#[test]
fn differing_frame_bytes_are_valid_between_engines_and_a_regression_between_builds() {
    // The finding that motivated splitting the profile: two encodings of one
    // stanza are both valid, but one encoder producing different bytes than it
    // did yesterday is the definition of a regression.
    let divergence = Divergence::Frame {
        index: 0,
        left_len: 10,
        right_len: 12,
    };
    assert!(!INTEROP.is_failure(&divergence));
    assert!(REGRESSION.is_failure(&divergence));
}

#[test]
fn coverage_lost_by_the_candidate_is_a_regression() {
    let divergence = Divergence::PlaintextCoverage {
        index: 0,
        only_left: 2,
        only_right: 0,
    };
    assert!(
        !INTEROP.is_failure(&divergence),
        "between engines, coverage is a property of the adapter"
    );
    assert!(REGRESSION.is_failure(&divergence));
    assert!(!REGRESSION.is_improvement(&divergence));
}

#[test]
fn coverage_gained_by_the_candidate_passes_and_is_reported() {
    // Direction is the whole point. Treating any coverage difference as a
    // failure would fail a build for observing more than the last one.
    let divergence = Divergence::PlaintextCoverage {
        index: 0,
        only_left: 0,
        only_right: 3,
    };
    assert!(!REGRESSION.is_failure(&divergence));
    assert!(REGRESSION.is_improvement(&divergence));
    assert!(
        !INTEROP.is_improvement(&divergence),
        "between engines neither side is the reference"
    );
}

#[test]
fn coverage_traded_in_both_directions_is_a_regression() {
    // Gaining elsewhere does not pay for what was lost here.
    let divergence = Divergence::PlaintextCoverage {
        index: 0,
        only_left: 1,
        only_right: 1,
    };
    assert!(REGRESSION.is_failure(&divergence));
    assert!(!REGRESSION.is_improvement(&divergence));
}

#[test]
fn a_frame_origin_that_degraded_is_a_regression_and_the_reverse_is_not() {
    let degraded = Divergence::FrameOrigin {
        index: 0,
        degraded: true,
    };
    let recovered = Divergence::FrameOrigin {
        index: 0,
        degraded: false,
    };

    assert!(
        !INTEROP.is_failure(&degraded) && !INTEROP.is_failure(&recovered),
        "two adapters differ here by design"
    );
    assert!(
        REGRESSION.is_failure(&degraded),
        "the candidate stopped reaching its engine's own buffer"
    );
    assert!(!REGRESSION.is_failure(&recovered));
    assert!(REGRESSION.is_improvement(&recovered));
}

#[test]
fn a_changed_failure_reason_matters_only_between_builds() {
    // Neither side has a payload, so no traffic differs. What differs is how
    // much each can say about why.
    let bytes = path_bytes();
    let divergence = Divergence::PlaintextStatus {
        index: 0,
        path: path(&bytes),
        left: PlaintextStatus::DecryptFailed,
        right: PlaintextStatus::Unobserved,
    };
    assert!(!INTEROP.is_failure(&divergence));
    assert!(REGRESSION.is_failure(&divergence));
}

#[test]
fn provenance_never_fails_because_it_is_not_a_verdict_about_the_engines() {
    let divergence = Divergence::Provenance {
        left: "sha256:a",
        right: "sha256:b",
    };
    assert!(!INTEROP.is_failure(&divergence));
    assert!(!REGRESSION.is_failure(&divergence));
}

// -- verdicts ---------------------------------------------------------------

#[test]
fn incomparable_is_not_a_pass() {
    // The reason the verdict is three-valued. Folding "these were unlike
    // things" into agreement reports a green result from a comparison that
    // never happened.
    let verdict = Verdict::Incomparable(Incomparable::DifferentInput);
    assert!(!verdict.is_pass());
    assert_eq!(verdict.incomparable(), Some(Incomparable::DifferentInput));

    assert!(Verdict::Pass.is_pass());
    assert_eq!(Verdict::Pass.incomparable(), None);
    assert!(!Verdict::Fail.is_pass());
    assert_eq!(Verdict::Fail.incomparable(), None);
}

#[test]
fn every_verdict_and_reason_renders() {
    assert_eq!(Verdict::Pass.to_string(), "pass");
    assert_eq!(Verdict::Fail.to_string(), "fail");
    let text = Verdict::Incomparable(Incomparable::NotWhole).to_string();
    assert!(
        text.contains("incomparable") && text.contains("truncated"),
        "{text}"
    );

    let reasons = [
        Incomparable::UndeclaredInput,
        Incomparable::DifferentInput,
        Incomparable::DifferentArtifactClass,
        Incomparable::DifferentTransform,
        Incomparable::DifferentDictionary,
        Incomparable::UnresolvableDictionary,
        Incomparable::UnknownCriticalTag,
        Incomparable::NotWhole,
    ];
    for (i, a) in reasons.iter().enumerate() {
        assert!(!a.name().is_empty());
        assert_eq!(a.to_string(), a.name());
        for b in reasons.iter().skip(i.saturating_add(1)) {
            assert_ne!(a.name(), b.name(), "reasons must be distinguishable");
        }
    }
}

#[test]
fn profiles_name_themselves_and_interop_is_the_default() {
    assert_eq!(ComparisonProfile::default(), INTEROP);
    assert_eq!(INTEROP.to_string(), "interop");
    assert_eq!(REGRESSION.to_string(), "regression");
    assert_ne!(INTEROP, REGRESSION);
    assert!(!alloc::format!("{INTEROP:?}").is_empty());
}
