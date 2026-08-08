//! The consumer's dependency graph is the argument, so it is checked.
//!
//! "Swap the engine and the consumer does not change" is easy to claim and easy
//! to erode: one `whatsapp-rust = ...` added for convenience and the crate is
//! coupled forever, with nothing failing to say so. This test is what says so.

#![allow(clippy::expect_used, clippy::panic)]

use std::path::Path;

/// Crates a consumer must never need, and why each one would be a mistake.
const FORBIDDEN: &[(&str, &str)] = &[
    (
        "whatsapp-rust",
        "an engine — the whole point is not naming one",
    ),
    ("zapo", "an engine"),
    ("baileys", "an engine"),
    ("tokio", "a runtime; the boundary is bytes in, values out"),
    ("async-std", "a runtime"),
    ("futures", "async at all: nothing here waits for anything"),
    (
        "reqwest",
        "a transport; a consumer is handed envelopes, it does not fetch them",
    ),
    ("tungstenite", "a transport"),
];

#[test]
fn the_consumer_depends_on_no_engine_runtime_or_transport() {
    let manifest =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
            .expect("this crate has a manifest");

    // Everything before `[dev-dependencies]`: test-only tooling is not part of
    // what a consumer would link.
    let shipped = manifest
        .split("[dev-dependencies]")
        .next()
        .expect("split yields at least one part");

    let found: Vec<_> = FORBIDDEN
        .iter()
        .filter(|(name, _)| shipped.contains(*name))
        .map(|(name, why)| format!("  {name}: {why}"))
        .collect();

    assert!(
        found.is_empty(),
        "the example consumer grew a dependency it should not have:\n{}\n\n\
         If a consumer genuinely needs this, the boundary is missing something \
         — add it there instead.",
        found.join("\n")
    );
}

#[test]
fn it_depends_on_the_boundary_and_nothing_else() {
    let manifest =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
            .expect("this crate has a manifest");
    let shipped = manifest
        .split("[dev-dependencies]")
        .next()
        .expect("split yields at least one part");

    let deps: Vec<&str> = shipped
        .split("[dependencies]")
        .nth(1)
        .expect("the manifest declares dependencies")
        .lines()
        .filter_map(|line| line.split_once(" = ").map(|(name, _)| name.trim()))
        .filter(|name| !name.is_empty() && !name.starts_with('#'))
        .collect();

    assert_eq!(
        deps,
        ["wa-wire-codec", "wa-wire-contract", "wa-wire-l1"],
        "the consumer's dependencies are the boundary itself; a new one is a \
         claim that the boundary is insufficient"
    );
}
