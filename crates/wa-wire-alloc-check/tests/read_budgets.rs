//! How long the read paths take, so a regression has a number to fail against.
//!
//! The upgrade gate has five criteria and four of them are measured: stanzas
//! not lost, frames still parsing, the same L1, plaintext coverage held. The
//! fifth is performance, and it had nothing behind it.
//!
//! # These are debug-build numbers
//!
//! `cargo test` builds without optimisation and so does CI, so the ceilings
//! are set against what a debug build costs. A release build is far faster and
//! would pass them trivially, which is fine: the point is to catch a change of
//! kind, and a change of kind shows up in either build.
//!
//! # Why a criterion needs a floor, not a graph
//!
//! A benchmark whose only output is a number tells the next person nothing:
//! they see 900 ns and cannot say whether that is fine. So each of these
//! carries a **budget**, and the assertion is against the budget rather than
//! against last week's run. The budgets are deliberately loose, several times
//! the measured cost, because the thing worth catching is a change of *kind* —
//! a copy where there was a borrow, a parse where there was none — and not the
//! few percent a different machine produces.
//!
//! Running under a counting allocator, which is why this lives here: the
//! numbers are what a caller pays in this crate's own conditions, and holding
//! both measurements in one place keeps them from disagreeing about the
//! fixture.
//!
//! In `tests/` rather than `benches/` on purpose: a criterion that only runs
//! when somebody remembers is not a criterion. The ceilings are set well above
//! the measured cost so a loaded CI runner does not fail a build that is fine.
//!
//! ```sh
//! cargo test -p wa-wire-alloc-check --test read_budgets -- --nocapture
//! ```

use std::hint::black_box;
use std::time::{Duration, Instant};

use wa_wire_codec::Parser;
use wa_wire_contract::{EnvelopeBuilder, EnvelopeRef, Flags};
use wa_wire_l1::content::derive_content;
use wa_wire_l1::testing::{FIXTURE_TABLE, Fixture};
use wa_wire_recording::{ArtifactClass, MetaBuilder, RecordingRef, RecordingWriter};

/// Iterations per measurement. Enough that a single scheduling hiccup does not
/// decide the answer, short enough that the whole file runs in under a second.
const ROUNDS: u32 = 20_000;

/// One measured path and what it is allowed to cost.
struct Budget {
    name: &'static str,
    /// Per operation. Generous on purpose: this catches a borrow becoming a
    /// copy, not a slower laptop.
    ceiling: Duration,
}

fn measure(budget: &Budget, mut work: impl FnMut()) -> Duration {
    // One untimed pass, so a cold cache is not charged to the first budget.
    work();

    let start = Instant::now();
    for _ in 0..ROUNDS {
        work();
    }
    let each = start.elapsed() / ROUNDS;

    println!(
        "{:<28} {:>8.0} ns/op   ceiling {:>6} ns",
        budget.name,
        each.as_nanos(),
        budget.ceiling.as_nanos()
    );
    assert!(
        each <= budget.ceiling,
        "{} took {} ns/op, past its {} ns budget. Either something started \
         copying, or the budget is wrong and moving it is a decision worth \
         writing down.",
        budget.name,
        each.as_nanos(),
        budget.ceiling.as_nanos()
    );
    each
}

fn receipt() -> Vec<u8> {
    Fixture::node("receipt")
        .attr("id", "ABCD1234")
        .jid_attr("from", "5511999998888")
        .attr("type", "read")
        .packed_attr("t", "1754000000")
        .build()
        .bytes()
        .to_vec()
}

/// `deviceSentMessage { message { conversation: "hello there" } }`.
fn payload() -> Vec<u8> {
    fn field(number: u32, value: &[u8]) -> Vec<u8> {
        // A varint tag, written out because the numbers here go past 31.
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
    let conversation = field(1, b"hello there");
    let inner = field(2, &conversation);
    field(31, &inner)
}

#[test]
fn the_read_paths_stay_inside_their_budgets() {
    let frame = receipt();
    let envelope = EnvelopeBuilder::new(Flags::inbound(), &frame)
        .encode_to_vec()
        .expect("encodes");
    let plaintext = payload();

    let meta = MetaBuilder::new()
        .adapter("engine", "0.1.0", "1.0", 1, ["l0.inbound.tap"])
        .expect("adapter")
        .artifact_class(ArtifactClass::Replayed)
        .expect("class");
    let mut writer = RecordingWriter::new(meta).expect("writer");
    for _ in 0..32 {
        writer.envelope(&envelope).expect("envelope");
    }
    let recording = writer.finish();

    let parser = Parser::new(FIXTURE_TABLE);

    measure(
        &Budget {
            name: "envelope decode",
            // Measured at ~313 ns in a debug build.
            ceiling: Duration::from_nanos(1_500),
        },
        || {
            let decoded = EnvelopeRef::decode(&envelope).expect("decodes");
            black_box(decoded.frame());
        },
    );

    measure(
        &Budget {
            name: "frame parse",
            // Measured at ~1_331 ns in a debug build.
            ceiling: Duration::from_micros(6),
        },
        || {
            let node = parser.parse(&frame).expect("parses");
            black_box(node.tag());
        },
    );

    measure(
        &Budget {
            name: "stanza derive",
            // Measured at ~11_222 ns in a debug build.
            ceiling: Duration::from_micros(45),
        },
        || {
            let node = parser.parse(&frame).expect("parses");
            black_box(wa_wire_l1::derive(&node).ok());
        },
    );

    measure(
        &Budget {
            name: "payload derive",
            // Measured at ~536 ns in a debug build.
            ceiling: Duration::from_nanos(2_500),
        },
        || {
            black_box(derive_content(&plaintext).expect("reads"));
        },
    );

    measure(
        &Budget {
            name: "recording walk (32)",
            // Measured at ~88_653 ns in a debug build.
            ceiling: Duration::from_micros(350),
        },
        || {
            let read = RecordingRef::decode(&recording).expect("reads");
            black_box(read.envelopes().count());
        },
    );
}

#[test]
fn a_budget_that_is_exceeded_fails_rather_than_prints() {
    // The assertion is the point. A benchmark that only printed would let a
    // regression through with a number nobody read.
    let outcome = std::panic::catch_unwind(|| {
        measure(
            &Budget {
                name: "deliberately impossible",
                ceiling: Duration::ZERO,
            },
            || {
                black_box((0..64).sum::<u64>());
            },
        );
    });
    assert!(outcome.is_err(), "a blown budget has to fail the run");
}
