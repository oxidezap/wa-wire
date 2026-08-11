# translation-bench

What one store translation costs in Rust, on the same record the JavaScript
measurement uses.

The JS side (`../handoff-cycle/measure-translation.mjs`) prices `zapo`'s codec on
the committed fixture's 1705-byte libsignal session record. This runs
`SessionRecord::deserialize` and `serialize` on the same bytes, read out of the
same fixture — a benchmark that generated its own record would be comparing two
sizes and calling it a language.

```console
cargo run --release -- 5000
```

Measured on this machine, medians of 5000 iterations:

| | Rust | JS |
| --- | ---: | ---: |
| pass-through | 0.014 µs | 0.041 – 0.047 µs |
| copy | 0.031 µs | 0.286 – 0.315 µs |
| decode | 1.39 – 1.43 µs | 2.19 – 2.82 µs |
| encode | 0.39 µs | 1.25 – 1.55 µs |
| read-modify-write | **1.78 – 1.82 µs** | **3.4 – 4.4 µs** |

A third path matters as much as either: Rust compiled to wasm and called from
JavaScript costs 2.9–3.0 µs for the same round trip, of which **0.095 µs is the
boundary crossing**. The crossing is three percent of it, which is the opposite
of what a per-access design is usually warned about.

What that adds up to, and why it does not settle the question, is in RFC-006's
Option E amendment. The short version: pass-through is a hundred times cheaper
than decoding, so the canonical format decides far more than the language does.

Its own workspace, and `wacore-libsignal` by path outside this repository — the
same arrangement the `whatsapp-rust` adapter uses, and for the same reason.
