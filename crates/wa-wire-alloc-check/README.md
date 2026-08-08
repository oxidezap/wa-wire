# wa-wire-alloc-check

Counts allocations and times the read paths, so the crates that make claims
about both have to prove them.

```console
cargo test -p wa-wire-alloc-check
```

## Why it exists

Five places said some version of the same thing:

| Where | Claim |
| --- | --- |
| root `README.md` | "none of them allocates while reading" |
| `wa-wire-contract` | "Decoding never allocates and never copies" |
| `wa-wire-recording` | "Reading never allocates" |
| `wa-wire-codec` | "No allocation: the comparison walks the parts" |
| `wa-wire-l1` | "comparing and rendering them is allocation-free" |

Nothing checked any of them. A claim repeated in five files and verified in none
is a claim that stops being true quietly, and the cost of it stopping is paid
per stanza by a process that runs for months.

## Why it is a crate of its own

Counting allocations means installing a `GlobalAlloc`, which cannot be written
without `unsafe`. Every other crate here sets `unsafe_code = "forbid"`, which an
`allow` cannot lift — and weakening that for a whole package to fit a test in
would trade a real guarantee for a measurement.

So the exception lives here, in the one crate that exists for the measurement
and that nothing depends on. It is also the only crate besides the gate that is
not `no_std`: installing a global allocator is a `std` thing to do.

## Time budgets, not benchmarks

Each read path carries a **ceiling**, and the assertion is against the ceiling
rather than against last week's number. A benchmark whose only output is a
number tells the next reader nothing: they see 900 ns and cannot say whether it
is fine.

| Path | Measured | Ceiling |
| --- | --- | --- |
| envelope decode | 210 ns | 1.5 µs |
| frame parse | 1.2 µs | 6 µs |
| stanza derive | 11.6 µs | 45 µs |
| payload derive | 521 ns | 2.5 µs |
| walk a 32-record recording | 87 µs | 350 µs |

Debug build, since `cargo test` and CI both build without optimisation.

Ceilings sit several times above the measurements on purpose. What is worth
catching is a borrow becoming a copy, not a loaded CI runner — a budget tight
enough to fail on scheduling noise gets disabled, and a disabled budget checks
nothing.

`stanza derive` at nine times the parse it contains is not a defect: the
generated derivation tries shapes richest-first until one matches, so a receipt
walks several that do not.

## It lives in `tests/`, not `benches/`

A criterion that runs only when somebody remembers is not a criterion. A test
asserts that a blown budget fails the run, so the mechanism cannot rot into
printing a number nobody reads.
