#!/usr/bin/env python3
"""Generate the payload derivation's field numbers from whatspec's proto.

`generate-l1.py` does this for the stanza half, out of whatspec's `incoming`
domain. This does it for the payload half, out of `WAProto.proto`, which
whatspec extracts from the WA Web bundle's `internalSpec` modules and pins by
SHA-256 in its manifest.

The split is the same one the stanza generator makes. The *numbers* come from
the spec, because they are the protocol's and a build that hand-wrote them
would keep compiling after a renumbering and lie in silence. The *rules* stay
hand-written in `content.rs`: which variants are worth naming, what unwrapping
means, which field of a variant is its text. None of that is in the `.proto`,
because it is knowledge about the protocol rather than about the schema.

    python3 tools/generate-content.py
"""

from __future__ import annotations

import hashlib
import json
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
CRATE = ROOT / "crates" / "wa-wire-l1"
PROTO = CRATE / "spec" / "WAProto.proto"
PROVENANCE = CRATE / "spec" / "provenance.json"
TARGET = CRATE / "src" / "generated" / "content_fields.rs"

# The variants worth naming, by their field name in `waE2E.Message`. Names
# rather than numbers, so this file states an editorial choice and the spec
# states the encoding.
NAMED_VARIANTS = {
    "conversation": "CONVERSATION",
    "imageMessage": "IMAGE",
    "contactMessage": "CONTACT",
    "locationMessage": "LOCATION",
    "extendedTextMessage": "EXTENDED_TEXT",
    "documentMessage": "DOCUMENT",
    "audioMessage": "AUDIO",
    "videoMessage": "VIDEO",
    "call": "CALL",
    "protocolMessage": "PROTOCOL",
    "stickerMessage": "STICKER",
    "reactionMessage": "REACTION",
}

# Where a variant keeps its text: the variant's field name, then the field name
# inside the message type it points at.
TEXT_FIELDS = {
    "extendedTextMessage": "text",
    "reactionMessage": "text",
    "imageMessage": "caption",
    "videoMessage": "caption",
    "documentMessage": "caption",
}

# The envelopes that hold another `Message`. Found by *type* rather than by a
# list of names: a list is a second place to add one and therefore a place to
# forget one, which is exactly the bug this generator exists to make impossible.
WRAPPER_TYPES = {"FutureProofMessage", "DeviceSentMessage"}

FIELD = re.compile(
    r"^\s*(?:optional|required|repeated)?\s*([\w.]+)\s+(\w+)\s*=\s*(\d+)\s*;"
)


def messages(source: str) -> dict[str, list[tuple[str, str, int]]]:
    """Every message, by its qualified name, as (type, name, number) triples.

    whatspec nests most of `waE2E` inside `Message`, so a scan that only saw
    top-level blocks would find `Message` and nothing it points at. The stack
    tracks depth; fields are attributed to the innermost block that is a
    message, and blocks that are not (enums, oneofs) are entered and ignored.

    Names are qualified because the spec reuses them: two different
    `ExtendedTextMessage` types exist under different parents, and they do not
    agree on what their fields hold.

    A hand-rolled scan rather than a protobuf parser: the file is generated and
    regular, and a dependency here would be the only one in the toolchain.
    """
    out: dict[str, list[tuple[str, str, int]]] = {}
    # (simple name, qualified path) for a message block, or None for a block
    # that holds no fields of its own (enum, oneof).
    stack: list[tuple[str, str] | None] = []

    for line in source.splitlines():
        stripped = line.strip()
        if stripped.startswith("//"):
            continue

        opening = re.match(r"^message (\w+) \{", stripped)
        if opening:
            simple = opening.group(1)
            parents = [entry[0] for entry in stack if entry is not None]
            path = ".".join([*parents, simple])
            out.setdefault(path, [])
            # The spec writes empty messages closed on their own line
            # (`message Signal {}`). Pushing one would leak a level and
            # misqualify every name after it.
            if not stripped.endswith("{}"):
                stack.append((simple, path))
            continue
        if re.match(r"^(enum|oneof)\s+\w+\s*\{", stripped):
            stack.append(None)
            continue
        if stripped.startswith("}"):
            if stack:
                stack.pop()
            continue

        current = stack[-1] if stack else None
        if current is None:
            continue
        match = FIELD.match(line)
        if match:
            out[current[1]].append(
                (match.group(1), match.group(2), int(match.group(3)))
            )
    return out


def fields_of(defs: dict[str, list[tuple[str, str, int]]], parent: str, type_: str) -> list[tuple[str, str, int]]:
    """The fields of `type_` as named from inside `parent`.

    Protobuf resolves an unqualified type against the enclosing scope first,
    and so does this: the `ExtendedTextMessage` a `Message` field points at is
    the one nested in `Message`, not a same-named type elsewhere.
    """
    simple = type_.rsplit(".", 1)[-1]
    for candidate in (f"{parent}.{simple}", simple, type_):
        if candidate in defs:
            return defs[candidate]
    return []


def require(condition: bool, message: str) -> None:
    if not condition:
        sys.exit(f"generate-content: {message}")


def check_balanced(source: str) -> None:
    """The scan must end where it started.

    A block this reader opened and never closed would silently misqualify every
    name after it, which is the failure mode that produced wrong numbers rather
    than an error.
    """
    depth = 0
    for line in source.splitlines():
        stripped = line.strip()
        if stripped.startswith("//"):
            continue
        depth += stripped.count("{") - stripped.count("}")
    require(depth == 0, f"the proto's braces do not balance ({depth} left open)")


def main() -> None:
    source = PROTO.read_text(encoding="utf-8")
    digest = "sha256:" + hashlib.sha256(PROTO.read_bytes()).hexdigest()
    recorded = json.loads(PROVENANCE.read_text())["protoDigest"]
    require(
        digest == recorded,
        f"the vendored proto hashes to {digest}, provenance says {recorded}",
    )

    check_balanced(source)
    defs = messages(source)
    require(
        "Message" in defs,
        "no top-level `Message`, which means the scan lost its place",
    )
    message = {name: (type_, number) for type_, name, number in defs["Message"]}

    lines = [
        "//! Field numbers in `waE2E.Message`, from whatspec's `WAProto.proto`.",
        "//!",
        "//! GENERATED FILE — do not edit by hand. Run `tools/generate-content.py`.",
        "//!",
        "//! Committed rather than produced by a build script so that a protocol",
        "//! change arrives as a reviewable diff, per RFC-009. CI regenerates and",
        "//! requires no change, which rules out drift.",
        "//!",
        "//! The rules that read these live in [`crate::content`] and are written by",
        "//! hand: which variant is worth naming, what unwrapping means, and which",
        "//! field of a variant carries its text are not in the schema.",
        "",
        "/// SHA-256 of the proto this module was generated from.",
        f'pub const PROTO_DIGEST: &str = "{digest}";',
        "",
    ]

    # The named variants.
    for field_name, constant in NAMED_VARIANTS.items():
        require(field_name in message, f"`Message.{field_name}` is gone from the spec")
        _, number = message[field_name]
        lines.append(f"/// `Message.{field_name}`.")
        lines.append(f"pub const {constant}: u32 = {number};")
    lines.append("")

    # Every envelope, by type.
    wrappers = sorted(
        (number, name, type_)
        for type_, name, number in defs["Message"]
        if type_ in WRAPPER_TYPES
    )
    require(len(wrappers) >= 2, "no wrappers found; the scan is wrong")
    lines.append("/// Every field of `Message` whose type holds another `Message`.")
    lines.append("///")
    lines.append("/// Collected by type rather than by name, so a wrapper added upstream")
    lines.append("/// arrives here without anyone remembering to list it.")
    lines.append(f"pub const WRAPPERS: [u32; {len(wrappers)}] = [")
    for number, name, type_ in wrappers:
        lines.append(f"    {number}, // {name}: {type_}")
    lines.append("];")
    lines.append("")

    # The one wrapper that is not a `FutureProofMessage`, named because the
    # field it keeps its message in is a different number.
    device_sent = [
        number
        for type_, name, number in defs["Message"]
        if type_.rsplit(".", 1)[-1] == "DeviceSentMessage"
    ]
    require(len(device_sent) == 1, "`Message.deviceSentMessage` is not a single field")
    lines.append("/// The first `FutureProofMessage` wrapper, for a test that needs one.")
    view_once = [n for ty, na, n in defs["Message"] if na == "viewOnceMessage"]
    require(len(view_once) == 1, "`Message.viewOnceMessage` is not a single field")
    lines.append(f"pub const VIEW_ONCE: u32 = {view_once[0]};")
    lines.append("")
    lines.append("/// `Message.deviceSentMessage`, the one wrapper that is not a")
    lines.append("/// `FutureProofMessage` and keeps its message elsewhere.")
    lines.append(f"pub const DEVICE_SENT: u32 = {device_sent[0]};")
    lines.append("")

    # Where each wrapper keeps its inner message, and where text lives.
    for name, type_ in (("DEVICE_SENT_INNER", "DeviceSentMessage"), ("FUTURE_PROOF_INNER", "FutureProofMessage")):
        inner = [n for _, f, n in fields_of(defs, "Message", type_) if f == "message"]
        require(len(inner) == 1, f"`{type_}.message` is not a single field")
        lines.append(f"/// `{type_}.message`.")
        lines.append(f"pub const {name}: u32 = {inner[0]};")
    lines.append("")

    lines.append("/// Where each variant that speaks keeps its text.")
    for variant, text_field in TEXT_FIELDS.items():
        type_, _ = message[variant]
        inner = [n for _, f, n in fields_of(defs, "Message", type_) if f == text_field]
        require(
            len(inner) == 1,
            f"`{type_}.{text_field}` is not a single field",
        )
        constant = f"{NAMED_VARIANTS[variant]}_TEXT"
        lines.append(f"/// `{type_}.{text_field}`.")
        lines.append(f"pub const {constant}: u32 = {inner[0]};")

    TARGET.write_text("\n".join(lines) + "\n", encoding="utf-8")
    # Formatted here, as `generate-l1.py` does: CI runs `cargo fmt --check` and
    # regenerates in separate jobs, so a generator whose output rustfmt would
    # rewrite fails one of them whatever the numbers say.
    subprocess.run(["rustfmt", "--edition", "2024", str(TARGET)], check=True)
    print(f"{TARGET.relative_to(ROOT)}: {len(NAMED_VARIANTS)} variants, {len(wrappers)} wrappers")


if __name__ == "__main__":
    main()
