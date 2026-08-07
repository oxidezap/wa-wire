#!/usr/bin/env python3
"""Regenerate the bundled token dictionaries.

Committed output, not a build script: a protocol change should arrive as a
reviewable diff (RFC-009). CI runs this and requires the tree to be unchanged,
which is what rules out drift.

    python3 tools/generate-tokens.py
"""

import hashlib
import json
import pathlib
import subprocess

ROOT = pathlib.Path(__file__).resolve().parent.parent
SOURCE = ROOT / "crates" / "wa-wire-codec" / "tokens.json"
TARGET = ROOT / "crates" / "wa-wire-codec" / "src" / "tokens" / "mod.rs"


def rust_str(value: str) -> str:
    escaped = value.replace("\\", "\\\\").replace('"', '\\"')
    return f'"{escaped}"'


def main() -> None:
    raw = SOURCE.read_bytes()
    digest = hashlib.sha256(raw).hexdigest()
    tables = json.loads(raw)

    single = tables["single_byte"]
    dictionaries = tables["double_byte"]

    lines = [
        "//! Bundled WhatsApp binary-protocol token dictionaries.",
        "//!",
        "//! GENERATED FILE — do not edit by hand. Run `tools/generate-tokens.py`.",
        "//!",
        "//! Committed rather than produced by a build script so that a protocol change",
        "//! arrives as a reviewable diff, per RFC-009. CI regenerates and requires no",
        "//! change, which rules out drift.",
        "",
        "use crate::token::TokenTable;",
        "",
        "/// SHA-256 of the source table this module was generated from.",
        f'pub const SOURCE_DIGEST: &str = "sha256:{digest}";',
        "",
        "/// Tokens addressable by a single byte, indexed from 1.",
        f"pub static SINGLE_BYTE: [&str; {len(single)}] = [",
        *(f"    {rust_str(token)}," for token in single),
        "];",
        "",
    ]

    for index, dictionary in enumerate(dictionaries):
        lines += [
            f"/// Dictionary {index}, addressed by the `DICTIONARY_{index}` tag plus an index byte.",
            f"pub static DICTIONARY_{index}: [&str; {len(dictionary)}] = [",
            *(f"    {rust_str(token)}," for token in dictionary),
            "];",
            "",
        ]

    lines += [
        "/// The double-byte dictionaries, in tag order.",
        f"pub static DICTIONARIES: [&[&str]; {len(dictionaries)}] = [",
        *(f"    &DICTIONARY_{i}," for i in range(len(dictionaries))),
        "];",
        "",
        "/// The bundled table, ready to pass to the parser.",
        "pub static TABLE: TokenTable<'static> = TokenTable::new(&SINGLE_BYTE, &DICTIONARIES);",
        "",
    ]

    TARGET.write_text("\n".join(lines))

    # Formatted here rather than left to the author: CI regenerates and requires
    # no diff, so the generator's output has to be byte-identical to what
    # `cargo fmt` would leave behind.
    subprocess.run(["rustfmt", "--edition", "2024", str(TARGET)], check=True)

    print(f"{TARGET.relative_to(ROOT)}: {len(single)} single-byte, "
          f"{len(dictionaries)} dictionaries, sha256:{digest[:16]}…")


if __name__ == "__main__":
    main()
