# translation-bench

What one store translation costs in Rust, on the same record the JavaScript
measurement uses.

The JS side prices `zapo`'s codecs on the same fixtures:
`../handoff-cycle/measure-translation.mjs` for the session record,
`../handoff-cycle/measure-domains.mjs` for sender keys, app-state and the
device. This runs
`SessionRecord::deserialize` and `serialize` on the same bytes, read out of the
same fixture — a benchmark that generated its own record would be comparing two
sizes and calling it a language.

```console
cargo run --release -- 5000
```

Measured on this machine, medians of five runs of 5000 iterations. The first run
after a build reports roughly double, so a single run is not a number.

| Session record, 1705 B | Rust | JS |
| --- | ---: | ---: |
| pass-through | 0.011 – 0.014 µs | 0.041 – 0.047 µs |
| copy | 0.026 – 0.031 µs | 0.286 – 0.315 µs |
| decode | 1.36 – 1.55 µs | 2.19 – 2.82 µs |
| encode | 0.33 – 0.37 µs | 1.25 – 1.55 µs |
| read-modify-write | **1.7 – 1.9 µs** | **3.4 – 4.4 µs** |

| Sender key record, 83 B | Rust | JS |
| --- | ---: | ---: |
| decode | 0.121 – 0.137 µs | 0.245 µs |
| encode | 0.079 – 0.087 µs | 0.406 µs |
| read-modify-write | **0.20 – 0.22 µs** | **0.65 µs** |

| Device key pair | Rust | JS |
| --- | ---: | ---: |
| split, borrowing | 0.015 µs | 0.04 – 0.10 µs |
| split, copying | 0.021 µs | 0.05 – 0.07 µs |

In JavaScript the borrow and the copy are inside each other'''s noise at that
scale; only Rust separates them.

The session record is the only expensive domain, and it is expensive because it
is twenty times larger than anything else — the cost tracks the record, not the
domain.

`whatsapp-rust`'s app-state `HashState` codec is `pub(crate)`, so only the
JavaScript side of that domain could be measured from outside. That is a finding
rather than a gap: a translating store outside the engine would have to carry a
second implementation of the engine's own encoder, which is where the
`timestamp ?? 0` defect lived.

A third path matters as much as either: Rust compiled to wasm and called from
JavaScript costs 2.9–3.0 µs for the same round trip, of which **0.095 µs is the
boundary crossing**. The crossing is three percent of it, which is the opposite
of what a per-access design is usually warned about.

What that adds up to, and why it does not settle the question, is in RFC-006's
Option E amendment. The short version: pass-through is a hundred times cheaper
than decoding, so the canonical format decides far more than the language does.

Its own workspace, and `wacore-libsignal` by path outside this repository — the
same arrangement the `whatsapp-rust` adapter uses, and for the same reason.
