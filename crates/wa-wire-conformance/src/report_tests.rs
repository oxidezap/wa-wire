//! Comparison tests.
//!
//! Two engines are simulated by building the same stanzas two ways: one
//! recording encodes a value as a token, the other as bytes. That is the case
//! the comparison exists to get right — same meaning, different encoding — and
//! it is what separates a real finding from noise.

use super::*;
extern crate alloc;
use alloc::vec::Vec;

use wa_wire_adapter::{AdapterInfo, Capability, CapabilitySet, Provenance, RawStanza};
use wa_wire_l1::testing::{FIXTURE_TABLE, Fixture, FixtureBuilder};

use crate::divergence::Layer;

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
        FIXTURE_TABLE,
    );

    assert!(report.agrees());
    assert!(report.is_identical());
    assert_eq!(report.compared(), 1);
    assert_eq!(report.divergences().count(), 0);
    assert_eq!(report.faults().count(), 0);
}

#[test]
fn empty_recordings_agree() {
    let report = compare(&recording("a", &[]), &recording("b", &[]), FIXTURE_TABLE);
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
        FIXTURE_TABLE,
    );

    assert!(report.agrees(), "the events mean the same thing");
    assert!(!report.is_identical(), "but the bytes differ");

    let frame_diffs: Vec<_> = report
        .divergences()
        .filter(|d| d.layer() == Layer::L0)
        .collect();
    assert_eq!(frame_diffs.len(), 1);
    assert!(!frame_diffs[0].is_fault());
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
        FIXTURE_TABLE,
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
        FIXTURE_TABLE,
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
        FIXTURE_TABLE,
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
        FIXTURE_TABLE,
    );

    let divergences: Vec<_> = report.divergences().collect();
    assert_eq!(divergences.len(), 1);
    assert_eq!(*divergences[0], Divergence::Length { left: 3, right: 1 });
    assert_eq!(report.compared(), 0, "nothing was compared past it");
    assert!(report.agrees(), "a length difference is not itself a fault");
}

#[test]
fn a_malformed_envelope_names_which_recording_it_came_from() {
    let good = envelope(receipt("A"));
    let (left, right): ([&[u8]; 1], [&[u8]; 1]) = ([&good], [b"not-an-envelope"]);

    let report = compare(
        &recording("engine-a", &left),
        &recording("broken-engine", &right),
        FIXTURE_TABLE,
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
        FIXTURE_TABLE,
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

    let report = compare(&older, &newer, FIXTURE_TABLE);
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
        FIXTURE_TABLE,
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
        FIXTURE_TABLE,
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
