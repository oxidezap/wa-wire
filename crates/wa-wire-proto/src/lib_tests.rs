//! The wire format, one shape at a time.
//!
//! Payloads are built by hand rather than by an encoder, so a failure here is
//! a fault in this reader and never in whatever produced the fixture.

use super::*;
extern crate alloc;
use alloc::string::ToString;
use alloc::vec::Vec;

/// `(number << 3) | wire`, for the small field numbers these fixtures use.
fn tag(number: u32, wire: u8) -> u8 {
    u8::try_from((number << 3) | u32::from(wire)).expect("fixture field numbers stay under 32")
}

fn assert_error<E: core::error::Error>(_: &E) {}

fn fields(payload: &[u8]) -> Result<Vec<Field<'_>>, Error> {
    let mut reader = Reader::new(payload);
    let mut out = Vec::new();
    while let Some(field) = reader.next() {
        out.push(field?);
    }
    Ok(out)
}

// -- the five wire types -----------------------------------------------------

#[test]
fn a_varint_reads_as_every_shape_it_can_carry() {
    let payload = [tag(1, 0), 0x96, 0x01];
    let value = fields(&payload).expect("reads")[0].value;

    assert_eq!(value, Value::Varint(150));
    assert_eq!(value.as_u64(), Some(150));
    assert_eq!(value.as_u32(), Some(150));
    assert_eq!(value.as_bool(), Some(true));
    assert_eq!(value.as_bytes(), None);
    assert_eq!(value.as_str(), None);
}

#[test]
fn a_zero_varint_is_false_and_a_nonzero_one_is_true() {
    assert_eq!(
        fields(&[tag(1, 0), 0])
            .expect("reads")
            .first()
            .map(|f| f.value.as_bool()),
        Some(Some(false))
    );
    assert_eq!(
        fields(&[tag(1, 0), 7])
            .expect("reads")
            .first()
            .map(|f| f.value.as_bool()),
        Some(Some(true))
    );
}

#[test]
fn zig_zag_decodes_both_directions() {
    // The encoding exists so small negatives stay small on the wire; a decoder
    // that got the sign wrong would read -1 as a huge positive.
    for (raw, want) in [
        (0u64, 0i64),
        (1, -1),
        (2, 1),
        (3, -2),
        (4_294_967_294, 2_147_483_647),
    ] {
        assert_eq!(Value::Varint(raw).as_sint64(), Some(want), "raw {raw}");
    }
}

#[test]
fn a_length_delimited_field_reads_as_bytes_a_string_or_a_message() {
    let payload = [tag(1, 2), 0x02, b'h', b'i'];
    let value = fields(&payload).expect("reads")[0].value;

    assert_eq!(value, Value::Bytes(b"hi"));
    assert_eq!(value.as_bytes(), Some(&b"hi"[..]));
    assert_eq!(value.as_str(), Some("hi"));
    assert!(value.as_message().is_some());
    assert_eq!(value.as_u64(), None);
}

#[test]
fn a_string_field_that_is_not_utf8_reads_as_none_rather_than_lossily() {
    // A payload disagreeing with its own schema. Reading it lossily would put
    // replacement characters into a record whose purpose is being compared.
    let payload = [tag(1, 2), 0x02, 0xFF, 0xFE];
    let value = fields(&payload).expect("reads")[0].value;
    assert_eq!(value.as_str(), None);
    assert_eq!(value.as_bytes(), Some(&[0xFF, 0xFE][..]));
}

#[test]
fn the_fixed_widths_read_little_endian() {
    let payload = [
        tag(1, 5),
        0x01,
        0x00,
        0x00,
        0x00,
        tag(2, 1),
        2,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
    ];
    let read = fields(&payload).expect("reads");
    assert_eq!(read[0].value, Value::Fixed32(1));
    assert_eq!(read[1].value, Value::Fixed64(2));
    assert_eq!(read[1].value.as_u64(), Some(2));
}

#[test]
fn an_embedded_message_is_walked_with_the_same_reader() {
    // inner: field 1 = "hi"
    let inner = [tag(1, 2), 0x02, b'h', b'i'];
    let mut payload = alloc::vec![tag(6, 2), u8::try_from(inner.len()).expect("short")];
    payload.extend_from_slice(&inner);

    let outer = fields(&payload).expect("reads");
    assert_eq!(outer[0].number, 6);
    let nested = outer[0].value.as_message().expect("a message");
    assert_eq!(
        nested.find_last(1).expect("reads").and_then(Value::as_str),
        Some("hi")
    );
}

// -- groups ------------------------------------------------------------------

#[test]
fn a_group_yields_everything_between_its_ends() {
    // Deprecated in the format and unused by this protocol, so it is read
    // rather than refused: stopping here would stop on a payload that could
    // have been handed over whole.
    let payload = [
        tag(3, 3), // start group 3
        tag(1, 0),
        42,        // inside
        tag(3, 4), // end group 3
        tag(9, 0),
        7, // after
    ];
    let read = fields(&payload).expect("reads");
    assert_eq!(read.len(), 2);
    assert_eq!(read[0].number, 3);
    assert_eq!(read[0].value, Value::Group(&[tag(1, 0), 42]));
    assert_eq!(read[1].number, 9, "reading resumes past the group");

    let inside = read[0].value.as_message().expect("walkable");
    assert_eq!(
        inside.find_last(1).expect("reads").and_then(Value::as_u64),
        Some(42)
    );
}

#[test]
fn nested_groups_close_in_order() {
    let payload = [
        tag(3, 3), // open 3
        tag(4, 3), // open 4
        tag(4, 4), // close 4
        tag(3, 4), // close 3
    ];
    let read = fields(&payload).expect("reads");
    assert_eq!(read.len(), 1);
    assert_eq!(read[0].value, Value::Group(&[tag(4, 3), tag(4, 4)]));
}

#[test]
fn an_unclosed_group_is_reported() {
    let payload = [tag(3, 3), tag(1, 0), 1];
    assert_eq!(
        fields(&payload),
        Err(Error::UnterminatedGroup { number: 3 })
    );
}

#[test]
fn a_group_end_without_a_start_is_reported() {
    assert_eq!(
        fields(&[tag(3, 4)]),
        Err(Error::UnexpectedGroupEnd { number: 3 })
    );
}

#[test]
fn a_group_closed_by_the_wrong_number_is_reported() {
    assert_eq!(
        fields(&[tag(3, 3), tag(4, 4)]),
        Err(Error::UnexpectedGroupEnd { number: 4 })
    );
}

#[test]
fn a_group_is_walked_past_whatever_it_holds() {
    // The skip inside a group has to handle every wire type, or a group
    // carrying anything but a varint would swallow the rest of the payload.
    let payload = [
        tag(3, 3), // open
        tag(1, 0),
        1, // varint
        tag(2, 1),
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0, // fixed64
        tag(4, 2),
        0x02,
        b'h',
        b'i', // bytes
        tag(5, 5),
        0,
        0,
        0,
        0,         // fixed32
        tag(3, 4), // close
        tag(9, 0),
        7, // after
    ];
    let read = fields(&payload).expect("reads");
    assert_eq!(read.len(), 2);
    assert_eq!(read[1].number, 9, "reading resumed past the group");

    // And everything inside is still reachable.
    let inside = read[0].value.as_message().expect("walkable");
    assert_eq!(
        inside.find_last(4).expect("reads").and_then(Value::as_str),
        Some("hi")
    );
}

#[test]
fn a_group_holding_something_malformed_reports_rather_than_swallowing_it() {
    for (payload, want) in [
        (
            [tag(3, 3), tag(1, 6), tag(3, 4)].as_slice(),
            Error::UnknownWireType(6),
        ),
        ([tag(3, 3), 0x00, 0x01].as_slice(), Error::ZeroFieldNumber),
    ] {
        assert_eq!(fields(payload), Err(want), "{payload:?}");
    }
}

#[test]
fn a_length_no_usize_can_hold_is_reported_rather_than_truncated() {
    // Reached here rather than through the reader: the smallest payload that
    // overflows a 64-bit `usize` is larger than one.
    assert_eq!(payload_len(4, 10), Ok(4));
    assert_eq!(payload_len(u64::from(u32::MAX), 10), Ok(4_294_967_295));
    if usize::BITS < 64 {
        assert!(payload_len(u64::MAX, 10).is_err());
    }
}

// -- malformed input ---------------------------------------------------------

#[test]
fn every_way_a_payload_can_be_malformed_is_reported() {
    let cases: [(&[u8], Error); 6] = [
        (&[tag(1, 0), 0x80], Error::MalformedVarint),
        (
            &[
                0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80,
            ],
            Error::MalformedVarint,
        ),
        (
            &[tag(1, 2), 0x09, b'a'],
            Error::UnexpectedEnd {
                needed: 9,
                available: 1,
            },
        ),
        (&[tag(1, 6)], Error::UnknownWireType(6)),
        (&[tag(1, 7)], Error::UnknownWireType(7)),
        (&[0x00, 0x01], Error::ZeroFieldNumber),
    ];
    for (payload, want) in cases {
        assert_eq!(fields(payload), Err(want), "{payload:?}");
    }
}

#[test]
fn a_field_cut_short_at_every_offset_is_reported_and_never_panics() {
    let whole: [u8; 12] = [
        tag(1, 2),
        0x02,
        b'h',
        b'i',
        tag(2, 0),
        0x96,
        0x01,
        tag(3, 5),
        1,
        0,
        0,
        0,
    ];
    for cut in 0..whole.len() {
        // Either it reads the whole prefix or it reports; nothing else.
        if let Err(error) = fields(&whole[..cut]) {
            assert!(!error.to_string().is_empty(), "cut {cut}");
        }
    }
    assert!(fields(&whole).is_ok());
}

#[test]
fn a_reader_stops_after_a_failure_rather_than_spinning() {
    // A caller looping over `next` on a bad byte would otherwise never end.
    let payload = [tag(1, 6)];
    let mut reader = Reader::new(&payload);
    assert_eq!(reader.next(), Some(Err(Error::UnknownWireType(6))));
    assert!(reader.is_failed());
    assert_eq!(reader.next(), None);
    assert_eq!(reader.next(), None);
}

#[test]
fn an_empty_payload_yields_nothing() {
    let mut reader = Reader::new(&[]);
    assert_eq!(reader.next(), None);
    assert!(!reader.is_failed());
    assert_eq!(Reader::new(&[]).find_last(1), Ok(None));
}

// -- lookups -----------------------------------------------------------------

#[test]
fn a_repeated_scalar_resolves_to_the_last_one_written() {
    // What the format says, and what an encoder that appends relies on.
    let payload = [tag(1, 0), 1, tag(1, 0), 2, tag(1, 0), 3];
    assert_eq!(
        Reader::new(&payload)
            .find_last(1)
            .expect("reads")
            .and_then(Value::as_u64),
        Some(3)
    );
}

#[test]
fn a_lookup_reports_a_payload_that_goes_bad_before_it_finds_anything() {
    let payload = [tag(1, 0), 1, tag(2, 6)];
    assert_eq!(
        Reader::new(&payload).find_last(9),
        Err(Error::UnknownWireType(6))
    );
}

#[test]
fn the_reader_reports_what_it_has_not_read() {
    let payload = [tag(1, 0), 1, tag(2, 0), 2];
    let mut reader = Reader::new(&payload);
    assert_eq!(reader.remaining().len(), 4);
    reader.next();
    assert_eq!(reader.remaining().len(), 2);
    reader.next();
    assert!(reader.remaining().is_empty());
}

#[test]
fn errors_render_and_are_comparable() {
    let all = [
        Error::MalformedVarint,
        Error::UnexpectedEnd {
            needed: 4,
            available: 1,
        },
        Error::UnknownWireType(6),
        Error::ZeroFieldNumber,
        Error::UnterminatedGroup { number: 3 },
        Error::UnexpectedGroupEnd { number: 4 },
    ];
    for (i, a) in all.iter().enumerate() {
        assert!(!a.to_string().is_empty());
        for b in all.iter().skip(i + 1) {
            assert_ne!(a, b);
            assert_ne!(a.to_string(), b.to_string());
        }
    }
    assert_error(&Error::MalformedVarint);
    assert!(!alloc::format!("{:?}", Value::Varint(1)).is_empty());
}

/// The tenth byte of a varint carries one bit, and no more.
///
/// Nine groups of seven reach bit 62, so the tenth contributes bit 63 alone.
/// Anything else shifts out of the word: nine `0x80`s and a `0x02` were read as
/// *zero*, which is a malformed varint accepted as a value — worse than one
/// refused, because nothing downstream can tell.
#[test]
fn a_tenth_varint_byte_past_one_bit_is_refused() {
    let mut wire = alloc::vec![0x08u8];
    wire.extend_from_slice(&[0x80; 9]);
    wire.push(0x02);
    let mut reader = Reader::new(&wire);
    assert!(matches!(reader.next(), Some(Err(Error::MalformedVarint))));

    // Bit 63 itself is legal: every bit on is `u64::MAX`.
    let mut largest = alloc::vec![0x08u8];
    largest.extend_from_slice(&[0xFF; 9]);
    largest.push(0x01);
    let field = Reader::new(&largest)
        .next()
        .expect("a field")
        .expect("well formed");
    assert_eq!(field.value.as_u64(), Some(u64::MAX));
}

/// An end-group naming the wrong field is refused at every depth.
///
/// A depth counter alone only checks the outermost close, so
/// `group 1 { group 2 { end 3 } end 1 }` balanced and the mismatched `end 3`
/// passed unremarked.
#[test]
fn a_nested_group_must_be_closed_by_its_own_number() {
    let wrong = [
        0x0Bu8, // start group, field 1
        0x13,   // start group, field 2
        0x1C,   // end group, field 3 — closes nothing that is open
        0x0C,   // end group, field 1
    ];
    let mut reader = Reader::new(&wrong);
    assert!(matches!(
        reader.next(),
        Some(Err(Error::UnexpectedGroupEnd { number: 3 }))
    ));

    // Closed by its own number, the same shape parses.
    let correct = [0x0Bu8, 0x13, 0x14, 0x0C];
    let field = Reader::new(&correct)
        .next()
        .expect("a field")
        .expect("well formed");
    assert_eq!(field.number, 1);
}
