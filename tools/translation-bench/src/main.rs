//! What the same translation costs in Rust, on the same bytes.
//!
//! The JS side decodes a 1705-byte libsignal session record in 2.19–2.82 µs
//! (median of five runs of 5000). This runs the equivalent so the comparison is
//! the same record, not the same idea of a record.

use std::time::Instant;

use wacore_libsignal::protocol::SessionRecord;

/// The first session record's bytes, out of the fixture's JSON.
///
/// Hand-scanned rather than parsed with a JSON crate: this binary exists to
/// measure a decoder, and adding a dependency to read its own input would put
/// that dependency's version in the way of the number.
fn extract_session_record(fixture: &str) -> Vec<u8> {
    let sessions = fixture
        .find("\"sessions\"")
        .expect("the fixture has sessions");
    let record = fixture[sessions..]
        .find("\"record\"")
        .expect("a session has a record")
        + sessions;
    let open = fixture[record..].find(':').expect("a value") + record;
    let start = fixture[open..].find('"').expect("a string") + open + 1;
    let end = fixture[start..].find('"').expect("a closing quote") + start;
    base64(&fixture[start..end])
}

/// Standard base64, no padding assumptions beyond `=`.
fn base64(text: &str) -> Vec<u8> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    let mut buffer = 0u32;
    let mut bits = 0u32;
    for byte in text.bytes() {
        if byte == b'=' {
            break;
        }
        let value = ALPHABET
            .iter()
            .position(|candidate| *candidate == byte)
            .unwrap_or_else(|| panic!("not base64: {}", byte as char)) as u32;
        buffer = buffer << 6 | value;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
        }
    }
    out
}

fn median(mut samples: Vec<f64>) -> f64 {
    samples.sort_by(|a, b| a.partial_cmp(b).expect("no NaN"));
    samples[samples.len() / 2]
}

fn time(label: &str, iterations: usize, mut work: impl FnMut()) -> f64 {
    for _ in 0..200 {
        work();
    }
    let mut samples = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let started = Instant::now();
        work();
        samples.push(started.elapsed().as_nanos() as f64 / 1000.0);
    }
    let value = median(samples);
    println!("{label:<44} {value:>8.4} µs");
    value
}

fn main() {
    // The same record the JS side measures, read out of the same fixture, so
    // the two numbers are about one thing. A benchmark that generated its own
    // record would be comparing two sizes and calling it a language.
    let fixture = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../handoff-cycle/fixtures/whatsapp-rust-session.json"),
    )
    .expect("the fixture");
    let bytes = extract_session_record(&fixture);
    let iterations: usize = std::env::args()
        .nth(1)
        .and_then(|n| n.parse().ok())
        .unwrap_or(5000);

    println!("session record: {} bytes, {iterations} iterations\n", bytes.len());

    // The floor: handing over the same bytes as a borrow.
    let passthrough = time("pass-through (borrow, no copy)", iterations, || {
        let view: &[u8] = &bytes[..];
        std::hint::black_box(view);
    });

    time("copy (session bytes)", iterations, || {
        std::hint::black_box(bytes.clone());
    });

    let decode = time("decode session record (proto -> Rust)", iterations, || {
        std::hint::black_box(SessionRecord::deserialize(&bytes).expect("decodes"));
    });

    let record = SessionRecord::deserialize(&bytes).expect("decodes");
    let encode = time("encode session record (Rust -> proto)", iterations, || {
        std::hint::black_box(record.serialize().expect("serializes"));
    });

    println!(
        "\nread-modify-write: {:.3} µs; pass-through is {:.0}x cheaper than decoding",
        decode + encode,
        decode / passthrough.max(f64::MIN_POSITIVE)
    );
}
