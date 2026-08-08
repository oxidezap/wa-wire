//! Each documented claim, measured.

#![allow(clippy::expect_used, clippy::panic)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::hint::black_box;

use wa_wire_codec::Parser;
use wa_wire_contract::{EnvelopeRef, Flags, NodePath, PlaintextEntry, PlaintextStatus};
use wa_wire_l1::testing::{FIXTURE_TABLE, Fixture, FixtureBuilder};
use wa_wire_recording::{ArtifactClass, MetaBuilder, RecordingRef, RecordingWriter};

thread_local! {
    /// Per-thread, so the test harness running tests in parallel cannot make
    /// one measurement count another's work.
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}

struct Counting;

// `try_with` rather than `with`: an allocation during thread-local teardown
// would otherwise panic inside the allocator, which is not a failure mode
// worth having in a counter.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let _ = ALLOCATIONS.try_with(|n| n.set(n.get().saturating_add(1)));
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let _ = ALLOCATIONS.try_with(|n| n.set(n.get().saturating_add(1)));
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// How many allocations `work` made.
fn allocations(work: impl FnOnce()) -> usize {
    ALLOCATIONS.with(|n| n.set(0));
    work();
    ALLOCATIONS.with(Cell::get)
}

/// The counter has to count, or every assertion below passes vacuously.
#[test]
fn the_counter_sees_an_allocation() {
    assert_eq!(allocations(|| {}), 0);
    assert!(allocations(|| drop(black_box(Vec::<u8>::with_capacity(64)))) > 0);
}

// -- wa-wire-contract --------------------------------------------------------

fn receipt() -> FixtureBuilder {
    Fixture::node("receipt")
        .attr("id", "ABCD1234")
        .jid_attr("from", "5511999998888")
        .attr("type", "read")
}

#[test]
fn decoding_an_envelope_allocates_nothing() {
    // "Decoding never allocates and never copies: an EnvelopeRef borrows from
    // the buffer it was decoded from."
    let fixture = receipt().build();
    let path = [0u8, 0];
    let entries = [PlaintextEntry {
        path: NodePath::from_le_bytes(&path),
        status: PlaintextStatus::Ok,
        payload: b"plaintext",
    }];
    let bytes = wa_wire_contract::EnvelopeBuilder::new(Flags::inbound(), fixture.bytes())
        .with_entries(entries.iter().copied())
        .encode_to_vec()
        .expect("encodes");

    let count = allocations(|| {
        let envelope = EnvelopeRef::decode(&bytes).expect("decodes");
        black_box(envelope.frame());
        black_box(envelope.entry_count());
        for entry in envelope.entries() {
            black_box(entry.payload);
            black_box(entry.path.len());
            for component in entry.path.iter() {
                black_box(component);
            }
        }
        black_box(envelope.entry_at(NodePath::from_le_bytes(&path)));
    });
    assert_eq!(count, 0, "decoding an envelope allocated {count} time(s)");
}

#[test]
fn encoding_into_a_slice_allocates_nothing() {
    // "Encoding writes once into a caller-supplied slice."
    let fixture = receipt().build();
    let builder = wa_wire_contract::EnvelopeBuilder::new(Flags::inbound(), fixture.bytes());
    let mut out = vec![0u8; builder.encoded_len().expect("sizes")];

    let count = allocations(|| {
        black_box(builder.encode_into_slice(&mut out).expect("encodes"));
    });
    assert_eq!(count, 0, "encoding into a slice allocated {count} time(s)");
}

#[test]
fn encoding_to_a_vec_allocates_exactly_once() {
    // "or allocates exactly once with encode_to_vec". Exactly once, not
    // "not much": a sizing pass followed by a growing write would be two, and
    // the whole reason the builder walks its entries twice is to avoid that.
    let fixture = receipt().build();
    let path = [0u8, 0];
    let entries = [PlaintextEntry {
        path: NodePath::from_le_bytes(&path),
        status: PlaintextStatus::Ok,
        payload: b"a payload long enough that a growing buffer would reallocate at least once",
    }];
    let builder = wa_wire_contract::EnvelopeBuilder::new(Flags::inbound(), fixture.bytes())
        .with_entries(entries.iter().copied());

    let count = allocations(|| {
        black_box(builder.encode_to_vec().expect("encodes"));
    });
    assert_eq!(count, 1, "encode_to_vec allocated {count} time(s)");
}

// -- wa-wire-codec -----------------------------------------------------------

#[test]
fn parsing_a_frame_and_walking_it_allocates_nothing() {
    let fixture = Fixture::node("message")
        .attr("id", "ABCD1234")
        .jid_attr("from", "5511999998888")
        .child(Fixture::node("enc").attr("type", "msg").attr("v", "2"))
        .child(Fixture::node("device-identity"))
        .build();
    let parser = Parser::new(FIXTURE_TABLE);
    // The walk stack is the test's own, so it is allocated before the
    // measurement rather than counted against the parser.
    let mut stack = Vec::with_capacity(16);

    let count = allocations(|| {
        let node = parser.parse(fixture.bytes()).expect("parses");
        stack.push(node);
        while let Some(current) = stack.pop() {
            black_box(current.tag());
            for (key, value) in current.attrs() {
                black_box(key);
                black_box(value);
            }
            for child in current.children() {
                black_box(child.tag());
            }
        }
    });
    assert_eq!(
        count, 0,
        "parsing and walking a frame allocated {count} time(s)"
    );
}

/// What `derive` costs, measured rather than claimed.
///
/// No doc promises the derivation is allocation-free, so this records what it
/// actually does instead of asserting a claim nobody made. The two facts worth
/// pinning: the common shape costs nothing, and the shapes that do allocate
/// allocate once per optional child rather than per field.
#[test]
fn deriving_the_common_shape_allocates_nothing() {
    let fixture = receipt().build();
    let parser = Parser::new(FIXTURE_TABLE);
    let node = parser.parse(fixture.bytes()).expect("parses");

    let count = allocations(|| {
        black_box(wa_wire_l1::derive(&node).expect("derives"));
    });
    assert_eq!(count, 0, "deriving a receipt allocated {count} time(s)");
}

#[test]
fn deriving_an_optional_child_costs_one_box_and_no_more() {
    // The generated shapes hold an optional child as a `Box`, so this one is
    // not free. It is bounded: one per optional child present, not one per
    // field read out of it.
    let fixture = Fixture::node("message")
        .attr("id", "ABCD1234")
        .jid_attr("from", "5511999998888")
        .jid_attr("recipient", "5511999998888")
        .attr("t", "1754000000")
        .child(Fixture::node("plaintext"))
        .build();
    let parser = Parser::new(FIXTURE_TABLE);
    let node = parser.parse(fixture.bytes()).expect("parses");

    let count = allocations(|| {
        black_box(wa_wire_l1::derive(&node).ok());
    });
    assert!(
        count <= 1,
        "deriving a message with one optional child allocated {count} time(s)"
    );
}

#[test]
fn comparing_a_packed_value_allocates_nothing() {
    // "No allocation: the comparison walks the parts." A packed digit run and
    // a JID have no text in the buffer to borrow, so a comparison that built
    // one would allocate on every attribute of every stanza.
    let fixture = Fixture::node("receipt")
        .attr("id", "ABCD1234")
        .jid_attr("from", "5511999998888")
        .packed_attr("t", "1754000000")
        .build();
    let parser = Parser::new(FIXTURE_TABLE);
    let node = parser.parse(fixture.bytes()).expect("parses");

    let count = allocations(|| {
        for (_, value) in node.attrs() {
            black_box(value.eq_str("5511999998888@s.whatsapp.net"));
            black_box(value.eq_str("1754000000"));
            black_box(value.eq_str("nothing like it"));
        }
    });
    assert_eq!(count, 0, "comparing values allocated {count} time(s)");
}

// -- wa-wire-recording -------------------------------------------------------

#[test]
fn reading_a_recording_allocates_nothing() {
    // "Reading never allocates."
    let fixture = receipt().build();
    let envelope = wa_wire_contract::EnvelopeBuilder::new(Flags::inbound(), fixture.bytes())
        .encode_to_vec()
        .expect("encodes");
    let meta = MetaBuilder::new()
        .adapter(
            "engine",
            "0.1.0",
            "1.0",
            1,
            ["l0.inbound.tap", "l0.plaintext"],
        )
        .expect("adapter")
        .artifact_class(ArtifactClass::Replayed)
        .expect("class")
        .input_digest(b"monday")
        .expect("input")
        .note("a fixture")
        .expect("note");
    let mut writer = RecordingWriter::new(meta).expect("writer");
    writer.envelope(&envelope).expect("envelope");
    writer.mark(42, "stream:error").expect("mark");
    let bytes = writer.finish();

    let count = allocations(|| {
        let recording = RecordingRef::decode(&bytes).expect("decodes");
        black_box(recording.integrity());
        for entry in recording.meta() {
            black_box(entry.value);
        }
        let adapter = recording.adapter().expect("adapter");
        for capability in adapter.capabilities.iter() {
            black_box(capability);
        }
        black_box(recording.artifact_class());
        black_box(recording.input_digest());
        black_box(recording.note());
        for record in recording.records() {
            black_box(record.payload);
            black_box(record.as_mark());
        }
        black_box(recording.envelope_count());
    });
    assert_eq!(count, 0, "reading a recording allocated {count} time(s)");
}

// -- wa-wire-proto -----------------------------------------------------------

/// `deviceSentMessage { message { conversation: "hello there" } }`.
///
/// Built rather than hand-counted: the outer tag needs two bytes and each
/// length depends on the one inside it, which is exactly the arithmetic a
/// person gets wrong. Built outside every measurement, so none of it counts.
fn device_sent_conversation() -> Vec<u8> {
    fn bytes_field(number: u32, value: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut tag = u64::from(number) << 3 | 2;
        loop {
            let byte = u8::try_from(tag & 0x7F).unwrap_or(0);
            tag >>= 7;
            if tag == 0 {
                out.push(byte);
                break;
            }
            out.push(byte | 0x80);
        }
        out.push(u8::try_from(value.len()).expect("short fixture"));
        out.extend_from_slice(value);
        out
    }

    let conversation = bytes_field(1, b"hello there");
    let inner = bytes_field(2, &conversation);
    bytes_field(31, &inner)
}

#[test]
fn reading_a_payload_allocates_nothing() {
    // The newest reader, held to the same rule as the other two: it borrows
    // from the payload rather than copying out of it.
    let payload = device_sent_conversation();

    let count = allocations(|| {
        let mut reader = wa_wire_proto::Reader::new(&payload);
        while let Some(field) = reader.next() {
            let field = field.expect("reads");
            black_box(field.number);
            black_box(field.value.as_u64());
            black_box(field.value.as_str());
            if let Some(mut nested) = field.value.as_message() {
                while let Some(inner) = nested.next() {
                    black_box(inner.expect("reads").value.as_str());
                }
            }
        }
    });
    assert_eq!(count, 0, "reading a payload allocated {count} time(s)");
}

#[test]
fn deriving_message_content_allocates_nothing() {
    // The text is borrowed straight out of the payload, so a consumer that
    // only looks at it pays nothing at all.
    let payload = device_sent_conversation();

    let count = allocations(|| {
        let content = wa_wire_l1::content::derive_content(&payload).expect("reads");
        black_box(content.kind);
        black_box(content.text);
        black_box(content.wrappers);
    });
    assert_eq!(count, 0, "deriving content allocated {count} time(s)");

    // And it really did read through the wrapper, or the measurement above
    // would be measuring a reader that gave up early.
    let content = wa_wire_l1::content::derive_content(&payload).expect("reads");
    assert_eq!(content.text, Some("hello there"));
    assert_eq!(content.wrappers, 1);
}

// -- wa-wire-l1 --------------------------------------------------------------

#[test]
fn rendering_a_value_into_a_buffer_allocates_nothing() {
    // "comparing and rendering them is allocation-free". Rendering *into* a
    // caller's buffer, which is the claim: collecting to a String obviously
    // allocates the String.
    use std::fmt::Write as _;

    let fixture = Fixture::node("receipt")
        .jid_attr("from", "5511999998888")
        .packed_attr("t", "1754000000")
        .build();
    let parser = Parser::new(FIXTURE_TABLE);
    let node = parser.parse(fixture.bytes()).expect("parses");
    let mut out = String::with_capacity(256);

    let count = allocations(|| {
        out.clear();
        for (_, value) in node.attrs() {
            write!(out, "{value}").expect("renders");
        }
    });
    assert_eq!(count, 0, "rendering values allocated {count} time(s)");
}

#[test]
fn repeated_children_are_iterators_rather_than_collections() {
    // "Repeated children are iterators, not collections, so a caller that
    // wants the first does not pay for the rest." Measured by giving it many
    // and taking one.
    let mut builder = Fixture::node("receipt")
        .attr("id", "ABCD1234")
        .jid_attr("from", "5511999998888")
        .attr("type", "read");
    for _ in 0..64 {
        builder = builder.child(Fixture::node("item").attr("id", "X"));
    }
    let fixture = builder.build();
    let parser = Parser::new(FIXTURE_TABLE);
    let node = parser.parse(fixture.bytes()).expect("parses");

    let count = allocations(|| {
        black_box(node.children().next());
    });
    assert_eq!(
        count, 0,
        "taking the first of 64 children allocated {count} time(s)"
    );
}
