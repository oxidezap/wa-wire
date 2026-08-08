//! Comparison tests.
//!
//! Two engines are simulated by building the same stanzas two ways: one
//! recording encodes a value as a token, the other as bytes. That is the case
//! the comparison exists to get right — same meaning, different encoding — and
//! it is what separates a real finding from noise.

use super::*;
extern crate alloc;
use alloc::vec::Vec;

use wa_wire_adapter::{AdapterInfo, Capability, CapabilitySet, Plaintext, Provenance, RawStanza};
use wa_wire_contract::{Direction, NodePath, PlaintextStatus};
use wa_wire_l1::testing::{FIXTURE_TABLE, Fixture, FixtureBuilder};

use crate::comparability::Comparability;
use crate::divergence::Layer;
use crate::profile::{ComparisonProfile, Incomparable, Verdict};
use crate::report::Tables;

fn info(id: &'static str) -> AdapterInfo<'static> {
    AdapterInfo::new(
        id,
        "0.1.0",
        "1.0",
        CapabilitySet::NONE.with(Capability::L0InboundTap),
    )
}

fn envelope(builder: FixtureBuilder) -> Vec<u8> {
    let fixture = builder.build();
    RawStanza::inbound(fixture.bytes())
        .encode_to_vec()
        .expect("encodes")
}

fn outbound_envelope(builder: FixtureBuilder) -> Vec<u8> {
    let fixture = builder.build();
    RawStanza::outbound(fixture.bytes())
        .encode_to_vec()
        .expect("encodes")
}

/// A receipt every shape in the spec accepts.
fn receipt(id: &str) -> FixtureBuilder {
    Fixture::node("receipt")
        .attr("id", id)
        .jid_attr("from", "5511999998888")
        .attr("type", "read")
}

fn recording<'a>(id: &'static str, envelopes: &'a [&'a [u8]]) -> Recording<'a> {
    Recording::new(info(id), envelopes)
}

// --- agreement -------------------------------------------------------------

#[test]
fn identical_recordings_agree_and_are_identical() {
    let a = envelope(receipt("ABCD"));
    let b = envelope(receipt("ABCD"));
    let (left, right): ([&[u8]; 1], [&[u8]; 1]) = ([&a], [&b]);

    let report = compare(
        &recording("engine-a", &left),
        &recording("engine-b", &right),
        Tables::shared(FIXTURE_TABLE),
    );

    assert!(report.agrees());
    assert!(report.is_identical());
    assert_eq!(report.compared(), 1);
    assert_eq!(report.divergences().count(), 0);
    assert_eq!(report.faults().count(), 0);
}

#[test]
fn empty_recordings_agree() {
    let report = compare(
        &recording("a", &[]),
        &recording("b", &[]),
        Tables::shared(FIXTURE_TABLE),
    );
    assert!(report.agrees());
    assert!(report.is_identical());
    assert_eq!(report.compared(), 0);
}

#[test]
fn replaying_a_recording_against_itself_agrees() {
    // The property everything else rests on: derivation is pure. If this ever
    // failed, comparing two engines would mean nothing.
    let one = envelope(receipt("A"));
    let two = envelope(receipt("B"));
    let envelopes: [&[u8]; 2] = [&one, &two];

    let report = replay(&recording("engine", &envelopes), FIXTURE_TABLE);
    assert!(report.agrees());
    assert!(report.is_identical());
    assert_eq!(report.compared(), 2);
}

// --- the case the whole comparison exists for ------------------------------

#[test]
fn different_encodings_of_one_stanza_are_not_a_fault() {
    // Two engines forwarding different bytes for one stanza is expected: the
    // format has more than one way to say a thing. What matters is that they
    // derive to the same event, and they do.
    let terse = envelope(receipt("ABCD"));
    let padded = envelope(receipt("ABCD").attr("padding", "ignored-by-the-shape"));
    let (left, right): ([&[u8]; 1], [&[u8]; 1]) = ([&terse], [&padded]);

    let report = compare(
        &recording("engine-a", &left),
        &recording("engine-b", &right),
        Tables::shared(FIXTURE_TABLE),
    );

    assert!(report.agrees(), "the events mean the same thing");
    assert!(!report.is_identical(), "but the bytes differ");

    let frame_diffs: Vec<_> = report
        .divergences()
        .filter(|d| d.layer() == Layer::L0)
        .collect();
    assert_eq!(frame_diffs.len(), 1);
    assert!(!ComparisonProfile::Interop.is_failure(frame_diffs[0]));
    assert_eq!(frame_diffs[0].index(), Some(0));
}

// --- real findings ---------------------------------------------------------

#[test]
fn a_different_derived_event_is_a_fault() {
    let read = envelope(receipt("ABCD"));
    let delivery = envelope(
        Fixture::node("receipt")
            .attr("id", "ABCD")
            .jid_attr("from", "5511999998888")
            .attr("type", "delivery"),
    );
    let (left, right): ([&[u8]; 1], [&[u8]; 1]) = ([&read], [&delivery]);

    let report = compare(
        &recording("engine-a", &left),
        &recording("engine-b", &right),
        Tables::shared(FIXTURE_TABLE),
    );

    assert!(!report.agrees(), "one of the two is wrong");
    let faults: Vec<_> = report.faults().collect();
    assert_eq!(faults.len(), 1);
    assert_eq!(faults[0].layer(), Layer::L1);
    assert!(matches!(faults[0], Divergence::Derivation { index: 0, .. }));
}

#[test]
fn deriving_where_the_other_failed_is_a_fault() {
    let good = envelope(receipt("ABCD"));
    // Missing `from`, which every receipt shape requires.
    let bad = envelope(Fixture::node("receipt").attr("id", "ABCD"));
    let (left, right): ([&[u8]; 1], [&[u8]; 1]) = ([&good], [&bad]);

    let report = compare(
        &recording("engine-a", &left),
        &recording("engine-b", &right),
        Tables::shared(FIXTURE_TABLE),
    );

    assert!(!report.agrees());
    let faults: Vec<_> = report.faults().collect();
    assert_eq!(faults.len(), 1);
    match faults[0] {
        Divergence::DerivationOutcome { left, right, .. } => {
            assert!(left.is_none(), "one derived");
            assert!(right.is_some(), "the other did not");
        }
        other => panic!("unexpected divergence: {other}"),
    }
}

#[test]
fn failing_the_same_way_is_agreement_not_a_finding() {
    // Neither engine models `presence`. Being consistently silent about a
    // stanza is not a divergence — it is two engines agreeing.
    let a = envelope(Fixture::node("presence").attr("type", "available"));
    let b = envelope(Fixture::node("presence").attr("type", "available"));
    let (left, right): ([&[u8]; 1], [&[u8]; 1]) = ([&a], [&b]);

    let report = compare(
        &recording("engine-a", &left),
        &recording("engine-b", &right),
        Tables::shared(FIXTURE_TABLE),
    );

    assert!(report.agrees());
    assert_eq!(report.faults().count(), 0);
}

#[test]
fn different_lengths_are_reported_once_and_stop_the_comparison() {
    // After a missing stanza every later index compares unrelated things, so
    // one report beats a hundred that all say the same thing.
    let one = envelope(receipt("A"));
    let two = envelope(receipt("B"));
    let three = envelope(receipt("C"));
    let (left, right): ([&[u8]; 3], [&[u8]; 1]) = ([&one, &two, &three], [&one]);

    let report = compare(
        &recording("engine-a", &left),
        &recording("engine-b", &right),
        Tables::shared(FIXTURE_TABLE),
    );

    let divergences: Vec<_> = report.divergences().collect();
    assert_eq!(divergences.len(), 1);
    assert_eq!(*divergences[0], Divergence::Length { left: 3, right: 1 });
    assert_eq!(report.compared(), 0, "nothing was compared past it");
    assert!(
        !report.agrees(),
        "one engine lost a stanza; the gate must not pass"
    );
}

// --- comparability ---------------------------------------------------------

#[test]
fn two_recordings_of_different_traffic_report_incomparable_not_disagreement() {
    // The failure the container exists to prevent. These two recordings differ
    // on every stanza, and every one of those differences would read exactly
    // like an engine fault. What is actually true is that nobody established
    // they were of the same traffic.
    use wa_wire_recording::ArtifactClass;

    let a = envelope(receipt("A"));
    let b = envelope(receipt("B"));
    let (left, right): ([&[u8]; 1], [&[u8]; 1]) = ([&a], [&b]);

    let older = recording("engine-a", &left)
        .with_comparability(Comparability::declared(b"monday", ArtifactClass::Replayed));
    let newer = recording("engine-b", &right)
        .with_comparability(Comparability::declared(b"tuesday", ArtifactClass::Replayed));

    let report = compare(&older, &newer, Tables::shared(FIXTURE_TABLE));

    assert_eq!(
        report.evaluate(ComparisonProfile::Regression),
        Verdict::Incomparable(Incomparable::DifferentInput)
    );
    assert_eq!(
        report.incomparable(),
        Some(Incomparable::DifferentInput),
        "and it says which of the reasons applied"
    );
    assert!(
        report.divergences().count() > 0,
        "the findings are still collected, so a human can see why"
    );
    assert!(
        !report.agrees(),
        "incomparable is not agreement under any profile"
    );
}

#[test]
fn declaring_the_same_input_lets_the_comparison_proceed() {
    use wa_wire_recording::ArtifactClass;

    let a = envelope(receipt("A"));
    let b = envelope(receipt("A"));
    let (left, right): ([&[u8]; 1], [&[u8]; 1]) = ([&a], [&b]);

    let baseline = recording("engine-a", &left)
        .with_comparability(Comparability::declared(b"monday", ArtifactClass::Replayed));
    let candidate = recording("engine-b", &right)
        .with_comparability(Comparability::declared(b"monday", ArtifactClass::Replayed));

    let report = compare(&baseline, &candidate, Tables::shared(FIXTURE_TABLE));
    assert_eq!(
        report.evaluate(ComparisonProfile::Regression),
        Verdict::Pass
    );
    assert_eq!(report.evaluate(ComparisonProfile::Interop), Verdict::Pass);
}

#[test]
fn an_in_memory_recording_and_a_declared_one_are_not_mixed() {
    use wa_wire_recording::ArtifactClass;

    let a = envelope(receipt("A"));
    let (left, right): ([&[u8]; 1], [&[u8]; 1]) = ([&a], [&a]);

    let vouched = recording("engine-a", &left);
    let declared = recording("engine-b", &right)
        .with_comparability(Comparability::declared(b"monday", ArtifactClass::Replayed));
    assert_eq!(vouched.comparability(), None);

    let report = compare(&vouched, &declared, Tables::shared(FIXTURE_TABLE));
    assert_eq!(
        report.evaluate(ComparisonProfile::Interop),
        Verdict::Incomparable(Incomparable::UndeclaredInput),
        "half a checked claim leaves the pair unchecked"
    );
}

// --- the envelope around the frame ----------------------------------------

fn plain_envelope(builder: FixtureBuilder, plaintexts: &[Plaintext<'_>]) -> Vec<u8> {
    let fixture = builder.build();
    RawStanza::inbound(fixture.bytes())
        .with_plaintexts(plaintexts)
        .encode_to_vec()
        .expect("encodes")
}

fn at(components: &[u16]) -> Vec<u8> {
    components.iter().flat_map(|c| c.to_le_bytes()).collect()
}

#[test]
fn the_same_frame_decrypted_two_ways_is_a_fault() {
    // The whole point of L0-plain. A frame-only comparison sees nothing here:
    // the bytes on the wire are identical and only the plaintexts differ.
    let p = at(&[0]);
    let path = NodePath::from_le_bytes(&p);
    let a = plain_envelope(receipt("A"), &[Plaintext::ok(path, b"hello")]);
    let b = plain_envelope(receipt("A"), &[Plaintext::ok(path, b"world")]);
    let (left, right): ([&[u8]; 1], [&[u8]; 1]) = ([&a], [&b]);

    let report = compare(
        &recording("engine-a", &left),
        &recording("engine-b", &right),
        Tables::shared(FIXTURE_TABLE),
    );

    assert!(!report.agrees(), "one of them decrypted wrongly");
    let fault = report.faults().next().expect("a fault");
    assert!(
        matches!(fault, Divergence::Plaintext { index: 0, .. }),
        "{fault:?}"
    );
}

#[test]
fn observing_less_plaintext_is_reported_without_failing_the_run() {
    // An adapter that cannot resolve a payload to its node says so rather than
    // guessing, and that is a limit on the adapter, not a fault in an engine.
    let p = at(&[0]);
    let path = NodePath::from_le_bytes(&p);
    let a = plain_envelope(receipt("A"), &[Plaintext::ok(path, b"hello")]);
    let b = plain_envelope(receipt("A"), &[Plaintext::unobserved(path)]);
    let (left, right): ([&[u8]; 1], [&[u8]; 1]) = ([&a], [&b]);

    let report = compare(
        &recording("engine-a", &left),
        &recording("engine-b", &right),
        Tables::shared(FIXTURE_TABLE),
    );

    assert!(report.agrees(), "coverage is not correctness");
    assert!(!report.is_identical());
    assert_eq!(
        *report.divergences().next().expect("a divergence"),
        Divergence::PlaintextCoverage {
            index: 0,
            only_left: 1,
            only_right: 0,
        }
    );
}

#[test]
fn plaintext_at_a_different_node_is_not_treated_as_the_same_payload() {
    // Same bytes, different node. Matching by ordinal would call this agreement.
    let (p0, p1) = (at(&[0]), at(&[1]));
    let a = plain_envelope(
        receipt("A"),
        &[Plaintext::ok(NodePath::from_le_bytes(&p0), b"same")],
    );
    let b = plain_envelope(
        receipt("A"),
        &[Plaintext::ok(NodePath::from_le_bytes(&p1), b"same")],
    );
    let (left, right): ([&[u8]; 1], [&[u8]; 1]) = ([&a], [&b]);

    let report = compare(
        &recording("engine-a", &left),
        &recording("engine-b", &right),
        Tables::shared(FIXTURE_TABLE),
    );

    assert_eq!(
        *report.divergences().next().expect("a divergence"),
        Divergence::PlaintextCoverage {
            index: 0,
            only_left: 1,
            only_right: 1,
        }
    );
}

#[test]
fn matching_plaintext_leaves_the_recordings_identical() {
    let p = at(&[0]);
    let path = NodePath::from_le_bytes(&p);
    let a = plain_envelope(receipt("A"), &[Plaintext::ok(path, b"hello")]);
    let b = plain_envelope(receipt("A"), &[Plaintext::ok(path, b"hello")]);
    let (left, right): ([&[u8]; 1], [&[u8]; 1]) = ([&a], [&b]);

    let report = compare(
        &recording("engine-a", &left),
        &recording("engine-b", &right),
        Tables::shared(FIXTURE_TABLE),
    );

    assert!(report.is_identical());
}

#[test]
fn disagreeing_about_direction_is_a_fault() {
    // Direction is a property of the stanza. A consumer reading one of these
    // recordings would be wrong about what the other saw.
    let fixture = receipt("A").build();
    let a = RawStanza::inbound(fixture.bytes())
        .encode_to_vec()
        .expect("encodes");
    let b = RawStanza::outbound(fixture.bytes())
        .encode_to_vec()
        .expect("encodes");
    let (left, right): ([&[u8]; 1], [&[u8]; 1]) = ([&a], [&b]);

    let report = compare(
        &recording("engine-a", &left),
        &recording("engine-b", &right),
        Tables::shared(FIXTURE_TABLE),
    );

    // Comparison is per direction now, so the two never meet at an index and
    // there is nothing to call a direction mismatch. What is wrong here is
    // narrower and worse: `engine-b` recorded a stanza travelling a way its own
    // manifest does not claim to observe, so nothing downstream can tell
    // whether the record is real.
    assert!(!report.agrees());
    assert_eq!(
        *report.faults().next().expect("a fault"),
        Divergence::UndeclaredDirection {
            adapter: "engine-b",
            count: 1,
            direction: Direction::Outbound,
        }
    );
}

/// An adapter that declares the outbound half may record it, and a difference
/// there is the engines' rather than the observers'.
#[test]
fn two_observers_of_the_outbound_half_are_compared_on_it() {
    let watching = AdapterInfo::new(
        "watching",
        "0.1.0",
        "1.0",
        CapabilitySet::NONE
            .with(Capability::L0InboundTap)
            .with(Capability::L0OutboundObserved),
    );
    let a = outbound_envelope(receipt("AAAA"));
    let b = outbound_envelope(receipt("BBBB"));
    let (la, rb): ([&[u8]; 1], [&[u8]; 1]) = ([&a], [&b]);

    let report = compare(
        &Recording::new(watching, &la),
        &Recording::new(watching, &rb),
        Tables::shared(FIXTURE_TABLE),
    );
    assert!(
        report
            .divergences()
            .any(|d| matches!(d, Divergence::Frame { .. })),
        "two different sends are a difference, not coverage"
    );
    assert!(
        !report
            .divergences()
            .any(|d| matches!(d, Divergence::DirectionCoverage { .. })),
        "both sides watch the outbound half, so nothing is uncovered"
    );
}

/// One observer of the outbound half and one without is coverage, not a fault.
///
/// Only one engine can report what it sent. Comparing a recording that has the
/// outbound half against one that cannot have it would otherwise read as the
/// second engine losing stanzas.
#[test]
fn an_observer_that_cannot_see_the_outbound_half_is_not_at_fault() {
    let watching = AdapterInfo::new(
        "watching",
        "0.1.0",
        "1.0",
        CapabilitySet::NONE
            .with(Capability::L0InboundTap)
            .with(Capability::L0OutboundObserved),
    );
    let inbound = envelope(receipt("AAAA"));
    let outbound = outbound_envelope(receipt("BBBB"));
    let (la, rb): ([&[u8]; 2], [&[u8]; 1]) = ([&inbound, &outbound], [&inbound]);

    let report = compare(
        &Recording::new(watching, &la),
        &recording("blind", &rb),
        Tables::shared(FIXTURE_TABLE),
    );
    assert!(
        report
            .divergences()
            .any(|d| matches!(d, Divergence::DirectionCoverage { .. })),
        "the difference is what each could see"
    );
    assert!(
        report.evaluate(ComparisonProfile::Interop) == Verdict::Pass,
        "an observer's reach is not the engine's fault: {:?}",
        report.divergences().collect::<Vec<_>>()
    );
}

#[test]
fn a_re_encoded_frame_is_recorded_but_does_not_fail_two_engines() {
    // Recorded rather than judged: between two engines this is how they differ
    // by design, and between two builds of one adapter it is the newer one
    // having stopped reaching its engine's own buffer. Suppressing it at the
    // source would have left the second case undetectable.
    let fixture = receipt("A").build();
    let a = RawStanza::inbound(fixture.bytes())
        .encode_to_vec()
        .expect("encodes");
    let b = RawStanza::inbound(fixture.bytes())
        .re_encoded()
        .encode_to_vec()
        .expect("encodes");
    let (left, right): ([&[u8]; 1], [&[u8]; 1]) = ([&a], [&b]);

    let report = compare(
        &recording("engine-a", &left),
        &recording("engine-b", &right),
        Tables::shared(FIXTURE_TABLE),
    );

    assert!(report.agrees(), "two adapters may legitimately differ here");
    assert_eq!(
        *report.divergences().next().expect("recorded"),
        Divergence::FrameOrigin {
            index: 0,
            degraded: true,
        }
    );
    assert_eq!(
        report.evaluate(ComparisonProfile::Regression),
        Verdict::Fail,
        "the same adapter reaching less far than it did is a regression"
    );
}

#[test]
fn two_engines_failing_to_decrypt_for_different_reasons_is_recorded() {
    // Neither carries a payload, so nothing about the traffic differs. What
    // differs is how much each could say about the failure — nothing between
    // engines, diagnostic ground between builds.
    let p = at(&[0]);
    let path = NodePath::from_le_bytes(&p);
    let a = plain_envelope(receipt("A"), &[Plaintext::failed(path)]);
    let b = plain_envelope(receipt("A"), &[Plaintext::unobserved(path)]);
    let (left, right): ([&[u8]; 1], [&[u8]; 1]) = ([&a], [&b]);

    let report = compare(
        &recording("engine-a", &left),
        &recording("engine-b", &right),
        Tables::shared(FIXTURE_TABLE),
    );

    assert!(report.agrees(), "between engines this says nothing");
    assert_eq!(
        *report.divergences().next().expect("recorded"),
        Divergence::PlaintextStatus {
            index: 0,
            path,
            left: PlaintextStatus::DecryptFailed,
            right: PlaintextStatus::Unobserved,
        }
    );
    assert_eq!(
        report.evaluate(ComparisonProfile::Regression),
        Verdict::Fail,
        "between builds, an adapter that stopped knowing why has regressed"
    );
    assert_eq!(report.failures(ComparisonProfile::Regression).count(), 1);
    assert_eq!(report.failures(ComparisonProfile::Interop).count(), 0);
}

#[test]
fn a_candidate_that_observes_more_passes_and_is_named_as_an_improvement() {
    // A gate that cannot say what improved can only ever deliver bad news.
    let p = at(&[0]);
    let path = NodePath::from_le_bytes(&p);
    let baseline_bytes = plain_envelope(receipt("A"), &[Plaintext::unobserved(path)]);
    let candidate_bytes = plain_envelope(receipt("A"), &[Plaintext::ok(path, b"decrypted")]);
    let (left, right): ([&[u8]; 1], [&[u8]; 1]) = ([&baseline_bytes], [&candidate_bytes]);

    let report = compare(
        &recording("baseline", &left),
        &recording("candidate", &right),
        Tables::shared(FIXTURE_TABLE),
    );

    assert_eq!(
        report.evaluate(ComparisonProfile::Regression),
        Verdict::Pass
    );
    let improvements: Vec<_> = report.improvements(ComparisonProfile::Regression).collect();
    assert_eq!(improvements.len(), 1);
    assert!(matches!(
        improvements[0],
        Divergence::PlaintextCoverage {
            only_left: 0,
            only_right: 1,
            ..
        }
    ));
    assert_eq!(
        report.improvements(ComparisonProfile::Interop).count(),
        0,
        "between engines neither side is the reference"
    );
}

#[test]
fn each_side_is_parsed_with_its_own_dictionary() {
    // The dictionary travels with the WhatsApp client version, and an upgrade
    // gate compares exactly the builds where it may have moved. A single table
    // for both sides would attribute a dictionary change to an engine.
    let a = envelope(receipt("A"));
    let (left, right): ([&[u8]; 1], [&[u8]; 1]) = ([&a], [&a]);

    let tables = Tables {
        left: FIXTURE_TABLE,
        right: FIXTURE_TABLE,
    };
    let report = compare(
        &recording("engine-a", &left),
        &recording("engine-b", &right),
        tables,
    );
    assert!(report.is_identical());

    // And the shorthand builds the same thing.
    let shared = Tables::shared(FIXTURE_TABLE);
    assert_eq!(
        compare(
            &recording("engine-a", &left),
            &recording("engine-b", &right),
            shared,
        )
        .divergences()
        .count(),
        0
    );
    assert!(!alloc::format!("{shared:?}").is_empty());
}

#[test]
fn a_malformed_envelope_names_which_recording_it_came_from() {
    let good = envelope(receipt("A"));
    let (left, right): ([&[u8]; 1], [&[u8]; 1]) = ([&good], [b"not-an-envelope"]);

    let report = compare(
        &recording("engine-a", &left),
        &recording("broken-engine", &right),
        Tables::shared(FIXTURE_TABLE),
    );

    assert!(!report.agrees());
    let faults: Vec<_> = report.faults().collect();
    assert_eq!(faults.len(), 1);
    assert_eq!(
        *faults[0],
        Divergence::MalformedEnvelope {
            adapter: "broken-engine",
            index: 0
        }
    );
}

#[test]
fn an_unparsable_frame_names_its_recording() {
    let good = envelope(receipt("A"));
    // A well-formed envelope carrying bytes that are not a stanza.
    let bad = RawStanza::inbound(b"\xff\xff\xff")
        .encode_to_vec()
        .expect("encodes");
    let (left, right): ([&[u8]; 1], [&[u8]; 1]) = ([&good], [&bad]);

    let report = compare(
        &recording("engine-a", &left),
        &recording("engine-b", &right),
        Tables::shared(FIXTURE_TABLE),
    );

    assert!(!report.agrees());
    assert!(report.faults().any(|d| matches!(
        d,
        Divergence::UnparsableFrame {
            adapter: "engine-b",
            ..
        }
    )));
}

// --- provenance ------------------------------------------------------------

#[test]
fn different_spec_builds_are_flagged_before_anything_else() {
    // An L1 difference between builds generated from different specs may be
    // the specs rather than the engines, and reading the report without
    // knowing that would send someone hunting the wrong bug.
    let a = envelope(receipt("A"));
    let (left, right): ([&[u8]; 1], [&[u8]; 1]) = ([&a], [&a]);

    let older = Recording::new(
        info("engine-a").with_provenance(Provenance::new("2.3000.1", "sha256:old", "0.1.0")),
        &left,
    );
    let newer = Recording::new(
        info("engine-b").with_provenance(Provenance::new("2.3000.9", "sha256:new", "0.1.0")),
        &right,
    );

    let report = compare(&older, &newer, Tables::shared(FIXTURE_TABLE));
    let first = report.divergences().next().expect("reported");
    assert_eq!(
        *first,
        Divergence::Provenance {
            left: "sha256:old",
            right: "sha256:new"
        }
    );
    assert!(
        report.agrees(),
        "a spec mismatch is context, not a fault in either engine"
    );
}

#[test]
fn matching_spec_builds_are_not_flagged() {
    let a = envelope(receipt("A"));
    let (left, right): ([&[u8]; 1], [&[u8]; 1]) = ([&a], [&a]);
    let spec = Provenance::new("2.3000.1", "sha256:same", "0.1.0");

    let report = compare(
        &Recording::new(info("engine-a").with_provenance(spec), &left),
        &Recording::new(info("engine-b").with_provenance(spec), &right),
        Tables::shared(FIXTURE_TABLE),
    );
    assert!(report.is_identical());
}

// --- deriving a whole recording --------------------------------------------

#[test]
fn deriving_a_recording_keeps_positions_aligned() {
    let good = envelope(receipt("A"));
    let unmodelled = envelope(Fixture::node("presence"));
    let envelopes: [&[u8]; 4] = [&good, b"not-an-envelope", &unmodelled, &good];

    let derived: Vec<_> = derive_all(&recording("engine", &envelopes), FIXTURE_TABLE).collect();

    assert_eq!(derived.len(), 4, "nothing is skipped");
    assert!(derived[0].as_ref().expect("decoded").is_ok());
    assert!(derived[1].is_none(), "malformed envelope holds its place");
    assert_eq!(
        derived[2].as_ref().expect("decoded").as_ref().err(),
        Some(&wa_wire_l1::DeriveError::UnknownStanza)
    );
    assert!(derived[3].as_ref().expect("decoded").is_ok());
}

// --- report surface --------------------------------------------------------

#[test]
fn a_default_report_agrees_about_nothing() {
    let report = Report::default();
    assert!(report.agrees());
    assert!(report.is_identical());
    assert_eq!(report.compared(), 0);
    assert_eq!(report.divergences().count(), 0);
    assert!(!alloc::format!("{report:?}").is_empty());
}

#[test]
fn faults_are_a_subset_of_divergences() {
    let read = envelope(receipt("ABCD"));
    let other = envelope(receipt("ABCD").attr("padding", "x"));
    let delivery = envelope(
        Fixture::node("receipt")
            .attr("id", "Z")
            .jid_attr("from", "u")
            .attr("type", "delivery"),
    );
    let read_two = envelope(receipt("Z"));
    let (left, right): ([&[u8]; 2], [&[u8]; 2]) = ([&read, &read_two], [&other, &delivery]);

    let report = compare(
        &recording("a", &left),
        &recording("b", &right),
        Tables::shared(FIXTURE_TABLE),
    );

    let all = report.divergences().count();
    let faults = report.faults().count();
    assert!(faults >= 1);
    assert!(
        all > faults,
        "a frame difference is reported but is no fault"
    );
    assert!(!report.agrees());
    assert_eq!(report.compared(), 2);
}

// --- direction ---------------------------------------------------------------

/// An outbound stanza is derived with the outbound grammar.
///
/// Two receipts a client sends, differing in a field the shape models. Derived
/// as outbound they are two derivations of one shape and must differ; derived
/// as inbound they would be two *different* inbound shapes, which is a reading
/// of a stanza nobody received.
#[test]
fn an_outbound_stanza_is_derived_with_the_outbound_grammar() {
    let a = outbound_envelope(
        Fixture::node("ack")
            .attr("id", "AAAA1111")
            .attr("class", "receipt")
            .attr("to", "5511999998888"),
    );
    let b = outbound_envelope(
        Fixture::node("ack")
            .attr("id", "AAAA1111")
            .attr("class", "receipt")
            .attr("to", "5511999997777"),
    );
    let (la, rb): ([&[u8]; 1], [&[u8]; 1]) = ([&a], [&b]);
    let report = compare(
        &recording("left", &la),
        &recording("right", &rb),
        Tables::shared(FIXTURE_TABLE),
    );
    let found: Vec<_> = report.divergences().collect();
    assert!(
        found.iter().any(|d| matches!(d.layer(), Layer::L1)),
        "a modelled field differing is an L1 finding: {found:?}"
    );
}

/// The outbound grammar knows tags the inbound one does not.
///
/// `<iq>` never arrives as a shape the `incoming` domain models, and a client
/// sends them constantly. Deriving one is only possible on the outbound side,
/// which is the clearest evidence the two grammars are not interchangeable.
#[test]
fn an_outbound_iq_derives_where_an_inbound_reading_has_nothing() {
    let iq = outbound_envelope(
        Fixture::node("iq")
            .attr("xmlns", "abt")
            .attr("type", "get")
            .attr("to", "s.whatsapp.net")
            .attr("id", "1")
            .child(Fixture::node("props").attr("protocol", "1")),
    );
    let (la, rb): ([&[u8]; 1], [&[u8]; 1]) = ([&iq], [&iq]);
    let report = compare(
        &recording("left", &la),
        &recording("right", &rb),
        Tables::shared(FIXTURE_TABLE),
    );
    let found: Vec<_> = report.divergences().collect();
    assert!(
        !found.iter().any(|d| matches!(d.layer(), Layer::L1)),
        "one stanza derived twice agrees with itself: {found:?}"
    );

    // And the same bytes read as inbound derive to nothing at all.
    let node = wa_wire_codec::Parser::new(FIXTURE_TABLE)
        .parse(
            wa_wire_contract::EnvelopeRef::decode(&iq)
                .expect("decodes")
                .frame(),
        )
        .expect("parses");
    assert!(wa_wire_l1::derive(&node).is_err());
    assert!(wa_wire_l1::derive_outgoing(&node).is_ok());
}
