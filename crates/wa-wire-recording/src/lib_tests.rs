//! Round trips, and the two rules the container exists to enforce.

use super::*;
extern crate alloc;
use alloc::vec::Vec;

use wa_wire_contract::Provenance;

fn meta() -> MetaBuilder {
    MetaBuilder::new()
        .adapter(
            "zapo",
            "0.1.0",
            "1.7",
            1,
            ["l0.inbound.tap", "l0.plaintext"],
        )
        .expect("adapter")
        .artifact_class(ArtifactClass::Synthetic)
        .expect("class")
}

fn written(envelopes: &[&[u8]]) -> Vec<u8> {
    let mut writer = RecordingWriter::new(meta()).expect("writer");
    for envelope in envelopes {
        writer.envelope(envelope).expect("envelope");
    }
    writer.finish()
}

// -- round trips ------------------------------------------------------------

#[test]
fn a_recording_round_trips_with_its_metadata() {
    let bytes = written(&[b"one", b"two", b"three"]);
    let recording = RecordingRef::decode(&bytes).expect("decodes");

    assert_eq!(recording.container_version(), CONTAINER_VERSION);
    assert_eq!(recording.integrity(), Integrity::Complete);
    assert_eq!(
        recording.envelopes().collect::<Vec<_>>(),
        [b"one".as_slice(), b"two", b"three"]
    );
    assert_eq!(recording.envelope_count(), 3);

    let adapter = recording.adapter().expect("adapter");
    assert_eq!(adapter.id, "zapo");
    assert_eq!(adapter.version, "0.1.0");
    assert_eq!(adapter.engine_version, "1.7");
    assert_eq!(adapter.contract_version, 1);
    assert_eq!(adapter.capabilities.len(), 2);
    assert!(adapter.capabilities.contains("l0.plaintext"));
    assert!(!adapter.capabilities.contains("l0.takeover"));
    assert_eq!(
        adapter.capabilities.iter().collect::<Vec<_>>(),
        ["l0.inbound.tap", "l0.plaintext"]
    );
    assert_eq!(recording.artifact_class(), Some(ArtifactClass::Synthetic));
}

#[test]
fn an_empty_recording_is_complete_rather_than_truncated() {
    let bytes = written(&[]);
    let recording = RecordingRef::decode(&bytes).expect("decodes");
    assert_eq!(recording.integrity(), Integrity::Complete);
    assert_eq!(recording.envelope_count(), 0);
    assert_eq!(recording.records().count(), 0);
}

#[test]
fn every_metadata_field_round_trips() {
    let provenance = Provenance::new("2.3000.1", "sha256:abc", "0.1.0");
    let meta = MetaBuilder::new()
        .provenance(&provenance)
        .expect("provenance")
        .dictionary("whatspec@2.3000.1")
        .expect("dictionary")
        .artifact_class(ArtifactClass::Sanitized)
        .expect("class")
        .input_digest(b"\x01\x02\x03\x04")
        .expect("input")
        .transform("pseudonymise-jids", "sha256:cfg")
        .expect("transform")
        .created_at(1_754_000_000_000)
        .expect("created")
        .note("captured against the test account")
        .expect("note");

    let bytes = RecordingWriter::new(meta).expect("writer").finish();
    let recording = RecordingRef::decode(&bytes).expect("decodes");

    assert_eq!(recording.provenance(), Some(provenance));
    assert_eq!(recording.dictionary(), Some("whatspec@2.3000.1"));
    assert_eq!(recording.artifact_class(), Some(ArtifactClass::Sanitized));
    assert_eq!(recording.input_digest(), Some(&b"\x01\x02\x03\x04"[..]));
    assert_eq!(
        recording.transform(),
        Some(("pseudonymise-jids", "sha256:cfg"))
    );
    assert_eq!(recording.created_at(), Some(1_754_000_000_000));
    assert_eq!(recording.note(), Some("captured against the test account"));
    assert_eq!(recording.meta().count(), 7);
}

#[test]
fn a_field_that_was_never_written_reads_as_absent() {
    let bytes = written(&[b"one"]);
    let recording = RecordingRef::decode(&bytes).expect("decodes");
    // A capture declares no input digest, which is what makes it an input to a
    // comparison rather than a result from one.
    assert_eq!(recording.input_digest(), None);
    assert_eq!(recording.provenance(), None);
    assert_eq!(recording.dictionary(), None);
    assert_eq!(recording.transform(), None);
    assert_eq!(recording.created_at(), None);
    assert_eq!(recording.note(), None);
}

#[test]
fn marks_sit_between_stanzas_without_becoming_stanzas() {
    let mut writer = RecordingWriter::new(meta()).expect("writer");
    writer.envelope(b"before").expect("envelope");
    writer.mark(1_500, "stream:error").expect("mark");
    writer.envelope(b"after").expect("envelope");
    let bytes = writer.finish();

    let recording = RecordingRef::decode(&bytes).expect("decodes");
    assert_eq!(
        recording.envelopes().collect::<Vec<_>>(),
        [b"before".as_slice(), b"after"],
        "a mark is an annotation about traffic, not traffic"
    );
    assert_eq!(recording.records().count(), 3);

    let mark = recording
        .records()
        .find_map(|record| record.as_mark())
        .expect("a mark");
    assert_eq!(mark.delta_us, 1_500);
    assert_eq!(mark.label, "stream:error");
}

// -- truncation -------------------------------------------------------------

#[test]
fn a_recording_with_no_trailer_is_readable() {
    // The artifact a crash recorder exists to produce. If this were a parse
    // error, the format would fail its most important use while passing every
    // test written against well-formed files.
    let mut writer = RecordingWriter::new(meta()).expect("writer");
    writer.envelope(b"one").expect("envelope");
    writer.envelope(b"two").expect("envelope");
    let frozen = writer.as_bytes().to_vec();

    let recording = RecordingRef::decode(&frozen).expect("a frozen buffer still decodes");
    assert_eq!(
        recording.integrity(),
        Integrity::Truncated {
            found: 2,
            dangling: 0
        }
    );
    assert!(!recording.integrity().is_complete());
    assert_eq!(
        recording.envelopes().collect::<Vec<_>>(),
        [b"one".as_slice(), b"two"],
        "everything written before the cut is still there"
    );
}

#[test]
fn a_record_cut_in_half_is_dropped_and_the_rest_survives() {
    let bytes = written(&[b"one", b"two"]);
    // Somewhere inside the second record, past the first.
    let cut = bytes.len() - 6;
    let recording = RecordingRef::decode(&bytes[..cut]).expect("decodes");

    match recording.integrity() {
        Integrity::Truncated { found, dangling } => {
            assert_eq!(found, 2, "both whole records were read");
            assert!(dangling > 0, "the partial trailer is reported, not read");
        }
        other => panic!("expected truncation, got {other:?}"),
    }
    assert_eq!(recording.envelope_count(), 2);
}

#[test]
fn truncation_at_every_offset_past_the_header_is_readable() {
    // Not "does not panic" — the reader must produce a usable answer at every
    // cut, because a ring buffer can be frozen at any of them.
    let bytes = written(&[b"one", b"two", b"three"]);
    let meta_end = {
        let recording = RecordingRef::decode(&bytes).expect("decodes");
        HEADER_LEN + recording.meta().map(|e| e.value.len() + 6).sum::<usize>()
    };

    for cut in meta_end..bytes.len() {
        let recording = RecordingRef::decode(&bytes[..cut])
            .unwrap_or_else(|error| panic!("cut {cut}: {error}"));
        // Every envelope it reports must be one that was written whole.
        for envelope in recording.envelopes() {
            assert!(
                [b"one".as_slice(), b"two", b"three"].contains(&envelope),
                "cut {cut} produced a partial envelope"
            );
        }
        if cut < bytes.len() {
            assert!(!recording.integrity().is_complete(), "cut {cut}");
        }
    }
    assert!(
        RecordingRef::decode(&bytes)
            .expect("whole")
            .integrity()
            .is_complete()
    );
}

#[test]
fn a_header_cut_short_is_an_error_rather_than_truncation() {
    // A buffer too small to hold a header never was a recording, which is a
    // different thing from a recording that stopped early.
    let bytes = written(&[b"one"]);
    for cut in 0..HEADER_LEN {
        let error = RecordingRef::decode(&bytes[..cut]).expect_err("must not decode");
        assert!(
            matches!(
                error,
                ReadError::HeaderTooShort { .. } | ReadError::NotARecording
            ),
            "cut {cut} gave {error:?}"
        );
    }
}

// -- damage -----------------------------------------------------------------

/// Where `needle` sits in `haystack`, for corrupting one byte of it.
fn offset_of(haystack: &[u8], needle: &[u8]) -> usize {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
        .expect("the fixture must contain it")
}

#[test]
fn a_flipped_byte_in_a_record_is_caught_by_the_checksum() {
    let mut bytes = written(&[b"one", b"two"]);
    let at = offset_of(&bytes, b"two");
    bytes[at] ^= 0xFF;

    let recording = RecordingRef::decode(&bytes).expect("still decodes");
    match recording.integrity() {
        Integrity::Damaged {
            claimed,
            found,
            checksum_ok,
        } => {
            assert!(!checksum_ok, "the checksum must catch it");
            assert_eq!((claimed, found), (2, 2), "the framing is intact");
        }
        other => panic!("expected damage, got {other:?}"),
    }
    assert_eq!(
        recording.envelope_count(),
        2,
        "damaged is not unreadable: a caller may still look at what is there"
    );
}

#[test]
fn a_flipped_byte_in_the_metadata_is_caught_too() {
    // The checksum covers the header and the metadata, not only the records.
    let mut bytes = written(&[b"one"]);
    let at = offset_of(&bytes, b"zapo");
    bytes[at] = b'Z';

    let recording = RecordingRef::decode(&bytes).expect("still decodes");
    assert!(matches!(
        recording.integrity(),
        Integrity::Damaged {
            checksum_ok: false,
            ..
        }
    ));
    assert_eq!(recording.adapter().map(|a| a.id), Some("Zapo"));
}

#[test]
fn a_trailer_that_miscounts_is_damage_even_though_the_checksum_holds() {
    // The checksum covers everything *before* the trailer, so it cannot cover
    // the count that the trailer carries. The count is its own witness: damage
    // to the body is caught by the checksum, damage to the count by this
    // comparison, and damage to the checksum field by the checksum. Every
    // field has exactly one detector, which is why neither check is redundant.
    let mut bytes = written(&[b"one", b"two"]);
    let count_at = bytes.len() - 8;
    bytes[count_at] = 9;

    let recording = RecordingRef::decode(&bytes).expect("decodes");
    assert_eq!(
        recording.integrity(),
        Integrity::Damaged {
            claimed: 9,
            found: 2,
            checksum_ok: true,
        }
    );
}

#[test]
fn a_flipped_byte_in_the_checksum_itself_is_caught() {
    let mut bytes = written(&[b"one"]);
    let at = bytes.len() - 1;
    bytes[at] ^= 0xFF;

    let recording = RecordingRef::decode(&bytes).expect("decodes");
    assert!(matches!(
        recording.integrity(),
        Integrity::Damaged {
            checksum_ok: false,
            ..
        }
    ));
}

// -- the critical bit -------------------------------------------------------

#[test]
fn an_unknown_ancillary_tag_is_skipped_and_costs_nothing() {
    let meta = meta()
        .raw(Tag(0x0042), b"from a later writer")
        .expect("raw");
    let bytes = RecordingWriter::new(meta).expect("writer").finish();

    let recording = RecordingRef::decode(&bytes).expect("decodes");
    assert_eq!(recording.unknown_critical_tags(), 0);
    assert_eq!(recording.adapter().map(|a| a.id), Some("zapo"));
    assert_eq!(
        recording
            .meta()
            .find(|entry| entry.tag == Tag(0x0042))
            .map(|entry| entry.value),
        Some(&b"from a later writer"[..]),
        "preserved rather than dropped"
    );
}

#[test]
fn an_unknown_critical_tag_is_counted_so_comparison_can_refuse() {
    // The point of the bit. Skipping a field that decides comparability would
    // let a reader produce a confident wrong verdict.
    let meta = meta()
        .raw(Tag(meta::CRITICAL_BIT | 0x0042), b"load-bearing")
        .expect("raw");
    let bytes = RecordingWriter::new(meta).expect("writer").finish();

    let recording = RecordingRef::decode(&bytes).expect("still readable");
    assert_eq!(recording.unknown_critical_tags(), 1);
    assert_eq!(
        recording.envelope_count(),
        0,
        "inspection still works; comparison is what must refuse"
    );
}

#[test]
fn an_unknown_record_kind_is_skipped_and_counted() {
    let mut writer = RecordingWriter::new(meta()).expect("writer");
    writer.envelope(b"one").expect("envelope");
    writer
        .record(Kind(0x7E), b"from a later writer")
        .expect("kind");
    writer.envelope(b"two").expect("envelope");
    let bytes = writer.finish();

    let recording = RecordingRef::decode(&bytes).expect("decodes");
    assert_eq!(recording.integrity(), Integrity::Complete);
    assert_eq!(recording.skipped_records(), 1);
    assert_eq!(
        recording.envelopes().collect::<Vec<_>>(),
        [b"one".as_slice(), b"two"],
        "walking past it must not lose what follows"
    );
}

// -- refusals ---------------------------------------------------------------

#[test]
fn a_buffer_that_is_not_a_recording_is_refused() {
    let mut bytes = written(&[b"one"]);
    bytes[0] = b'X';
    assert_eq!(RecordingRef::decode(&bytes), Err(ReadError::NotARecording));
}

#[test]
fn a_newer_container_version_is_refused_rather_than_guessed_at() {
    // An older reader that guessed at a newer layout would produce a confident
    // wrong answer, which is the failure this format exists to prevent.
    let mut bytes = written(&[b"one"]);
    bytes[4] = 99;
    assert_eq!(
        RecordingRef::decode(&bytes),
        Err(ReadError::UnsupportedVersion(99))
    );
}

#[test]
fn a_metadata_block_that_runs_past_the_buffer_is_refused() {
    let mut bytes = written(&[b"one"]);
    bytes[6..10].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(matches!(
        RecordingRef::decode(&bytes),
        Err(ReadError::MetaOutOfBounds { .. })
    ));
}

#[test]
fn a_metadata_entry_that_runs_past_its_block_is_refused() {
    // The block declares its own length, so an entry overflowing it is a
    // header fault rather than an interrupted write.
    let meta_len = 6u32;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&MAGIC);
    bytes.extend_from_slice(&CONTAINER_VERSION.to_le_bytes());
    bytes.extend_from_slice(&meta_len.to_le_bytes());
    bytes.extend_from_slice(&Tag::NOTE.0.to_le_bytes());
    bytes.extend_from_slice(&99u32.to_le_bytes());

    assert_eq!(
        RecordingRef::decode(&bytes),
        Err(ReadError::MalformedMeta { tag: Tag::NOTE.0 })
    );
}

#[test]
fn a_repeated_tag_is_refused_at_the_writer() {
    // A reader takes the first, so a duplicate is a value the writer believes
    // it set and the reader will never see.
    let error = MetaBuilder::new()
        .note("first")
        .expect("first")
        .note("second")
        .expect_err("must refuse");
    assert_eq!(error, WriteError::DuplicateTag(Tag::NOTE.0));
}

#[test]
fn an_adapter_that_declares_nothing_says_so() {
    // A capability set can legitimately be empty — an adapter mid-bring-up
    // declares nothing rather than declaring falsely.
    let meta = MetaBuilder::default()
        .adapter("bare", "0.0.1", "0", 1, [])
        .expect("adapter");
    let bytes = RecordingWriter::new(meta).expect("writer").finish();

    let recording = RecordingRef::decode(&bytes).expect("decodes");
    let capabilities = recording.adapter().expect("adapter").capabilities;
    assert!(capabilities.is_empty());
    assert_eq!(capabilities.len(), 0);
    assert_eq!(capabilities.iter().count(), 0);
    assert!(!capabilities.contains("l0.inbound.tap"));
}

#[test]
fn the_writer_reports_what_it_holds() {
    let mut writer = RecordingWriter::new(meta()).expect("writer");
    assert!(writer.is_empty());
    writer.envelope(b"one").expect("envelope");
    assert_eq!(writer.len(), 1);
    assert!(!writer.is_empty());
    assert!(!writer.as_bytes().is_empty());
}

/// A recording that says it ends and does not is not complete.
///
/// The checksum cannot catch this. It covers the bytes up to the trailer,
/// which is everything the trailer knew about — so records appended afterwards
/// leave the count right, the checksum right, and the file wrong. A gate
/// reading one would compare the prefix and pass on traffic it never saw.
#[test]
fn records_appended_after_the_trailer_are_reported() {
    let mut writer = RecordingWriter::new(meta()).expect("writer");
    writer.envelope(b"first").expect("envelope");
    let mut bytes = writer.finish();

    let whole = RecordingRef::decode(&bytes).expect("decodes");
    assert_eq!(whole.integrity(), Integrity::Complete);

    // A second writer's output, appended — the shape a naive `cat` produces.
    let mut second = RecordingWriter::new(meta()).expect("writer");
    second.envelope(b"appended").expect("envelope");
    let extra = second.finish();
    bytes.extend_from_slice(&extra);

    let joined = RecordingRef::decode(&bytes).expect("still decodes");
    assert_eq!(
        joined.integrity(),
        Integrity::TrailingBytes {
            found: 1,
            trailing: extra.len(),
        },
        "the first trailer accounted for one record and the file holds more"
    );
    // What was appended is not read: guessing what it is would invent the
    // thing the trailer exists to state.
    assert_eq!(joined.envelope_count(), 1);
}

/// Even a single stray byte after the trailer is reported.
#[test]
fn one_byte_after_the_trailer_is_enough() {
    let mut writer = RecordingWriter::new(meta()).expect("writer");
    writer.envelope(b"only").expect("envelope");
    let mut bytes = writer.finish();
    bytes.push(0);

    assert!(matches!(
        RecordingRef::decode(&bytes).expect("decodes").integrity(),
        Integrity::TrailingBytes { trailing: 1, .. }
    ));
}
