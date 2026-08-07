#!/usr/bin/env python3
"""Generate the L1 derivation from whatspec's `incoming` domain.

Committed output, not a build script: a protocol change should arrive as a
reviewable diff (RFC-009). CI runs this and requires the tree to be unchanged.

The generator emits *structure* — which extraction primitive to call, in what
order, into which field. The primitives themselves are hand-written in
`extract.rs`, so a protocol change moves shapes and calls, never rules.

    python3 tools/generate-l1.py
"""

import json
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
CRATE = ROOT / "crates" / "wa-wire-l1"
SPEC = CRATE / "spec" / "incoming.json"
PROVENANCE = CRATE / "spec" / "provenance.json"
TARGET = CRATE / "src" / "generated" / "mod.rs"

# Field methods the generator knows how to emit. Anything else is dropped and
# reported — silently omitting a field would make the derivation quietly
# incomplete, which is the one failure mode a conformance suite cannot catch.
SCALAR_METHODS = {
    "attrString": ("Value<'a>", "extract::attr_string(node, {key})?"),
    "maybeAttrString": ("Option<Value<'a>>", "extract::maybe_attr_string(node, {key})"),
    "attrInt": ("i64", "extract::attr_int(node, {key})?"),
    "maybeAttrInt": ("Option<i64>", "extract::maybe_attr_int(node, {key})?"),
    "attrTime": ("i64", "extract::attr_time(node, {key})?"),
    "maybeAttrTime": ("Option<i64>", "extract::maybe_attr_time(node, {key})?"),
    "attrJidWithType": ("Jid<'a>", "extract::attr_jid(node, {key})?"),
    "attrDeviceJid": ("Jid<'a>", "extract::attr_jid(node, {key})?"),
    "attrUserJid": ("Jid<'a>", "extract::attr_jid(node, {key})?"),
    "contentBytes": ("&'a [u8]", "extract::content_bytes(node)?"),
    "contentUint": ("u64", "extract::content_uint(node)?"),
}

# A JID method whose field is optional degrades to the maybe_ variant.
OPTIONAL_JID = {
    "attrJidWithType": "extract::maybe_attr_jid(node, {key})?",
    "attrDeviceJid": "extract::maybe_attr_jid(node, {key})?",
    "attrUserJid": "extract::maybe_attr_jid(node, {key})?",
}

ENUM_METHODS = {"attrEnum", "attrEnumValues", "maybeAttrEnum", "attrEnumOrNullIfUnknown"}
CHILD_METHODS = {"child", "maybeChild"}
LIST_METHODS = {"forEachChildWithTag", "mapChildrenWithTag"}

RESERVED = {
    "type", "self", "ref", "match", "move", "box", "final", "override", "abstract",
    "as", "async", "await", "become", "do", "fn", "for", "if", "in", "let", "loop",
    "mod", "priv", "pub", "static", "struct", "super", "trait", "true", "false",
    "typeof", "unsafe", "use", "where", "while", "yield", "impl", "enum", "const",
    "continue", "crate", "else", "extern", "return", "break", "macro", "virtual",
}

drops: list[str] = []

# whatspec records every place the bundle reads a field, so the same name can
# appear several times with different methods — `t` is read as a string, an int
# and a timestamp at three different call sites. They describe one field, so the
# most specific reading wins. Readings from different categories (an attribute
# and a child) genuinely disagree; the first is kept and the rest reported.
SPECIFICITY = {
    "attrTime": 60, "maybeAttrTime": 55,
    "attrJidWithType": 50, "attrDeviceJid": 50, "attrUserJid": 50,
    "attrEnum": 45, "attrEnumValues": 45, "attrEnumOrNullIfUnknown": 44,
    "maybeAttrEnum": 43,
    "attrInt": 40, "maybeAttrInt": 35,
    "child": 30, "maybeChild": 25,
    "forEachChildWithTag": 30, "mapChildrenWithTag": 30,
    "contentBytes": 20, "contentUint": 20,
    "attrString": 10, "maybeAttrString": 5,
}


def category(method: str) -> str:
    if method in CHILD_METHODS or method in LIST_METHODS:
        return "child"
    if method.startswith("content"):
        return "content"
    return "attr"


# The optional form of each reading. Whatspec observes call sites, and one site
# requiring a field does not make the field required on the wire — the next
# stanza may simply not carry it. So optionality is taken conservatively: if any
# reading says a field can be absent, it can be absent.
OPTIONAL_FORM = {
    "attrString": "maybeAttrString",
    "attrInt": "maybeAttrInt",
    "attrTime": "maybeAttrTime",
    "attrEnum": "maybeAttrEnum",
    "attrEnumValues": "maybeAttrEnum",
    "child": "maybeChild",
}


def normalise(field: dict) -> dict:
    """`required` is the authority, not the method name.

    Whatspec records both, and they disagree: a call site can use the
    always-present reader (`child`, `attrString`) on a field it also marks
    optional. Trusting the method there makes the generated shape reject
    stanzas that are perfectly valid.
    """
    out = dict(field)
    if not out.get("required"):
        out["method"] = OPTIONAL_FORM.get(out["method"], out["method"])
    return out


def dedupe(fields: list[dict], owner: str) -> list[dict]:
    """One field per name: the most specific reading, at the weakest
    obligation any reading recorded."""
    chosen: dict[str, dict] = {}
    order: list[str] = []
    for raw in fields:
        field = normalise(raw)
        name = field["name"]
        if name not in chosen:
            chosen[name] = field
            order.append(name)
            continue
        kept = chosen[name]
        if category(kept["method"]) != category(field["method"]):
            drops.append(
                f"{owner}.{name}: {field['method']} conflicts with kept {kept['method']}"
            )
            continue
        optional = not (kept.get("required") and field.get("required"))
        if SPECIFICITY.get(field["method"], 0) > SPECIFICITY.get(kept["method"], 0):
            merged = dict(field)
            merged["children"] = field.get("children") or kept.get("children") or []
            merged.setdefault("tag", kept.get("tag"))
            chosen[name] = merged
        if optional:
            entry = chosen[name]
            entry["required"] = False
            entry["method"] = OPTIONAL_FORM.get(entry["method"], entry["method"])
    return [chosen[name] for name in order]


def snake(name: str) -> str:
    out = re.sub(r"[^0-9a-zA-Z]+", "_", name)
    out = re.sub(r"(?<=[a-z0-9])([A-Z])", r"_\1", out)
    out = re.sub(r"__+", "_", out).strip("_").lower()
    if not out:
        out = "field"
    if out[0].isdigit():
        out = f"n{out}"
    return f"r#{out}" if out in RESERVED else out


def pascal(name: str) -> str:
    parts = re.split(r"[^0-9a-zA-Z]+", name)
    out = "".join(p[:1].upper() + p[1:] for p in parts if p)
    if not out:
        out = "Item"
    if out[0].isdigit():
        out = f"N{out}"
    return out


def rust_str(value: str) -> str:
    return '"' + value.replace("\\", "\\\\").replace('"', '\\"') + '"'


class Emitter:
    def __init__(self) -> None:
        self.structs: list[str] = []
        self.enums: dict[str, list[str]] = {}

    def enum_for(self, owner: str, field: dict) -> str | None:
        """Emit an enum type for a field, deduplicating identical value sets."""
        keys = field.get("enumKeys")
        if keys is None:
            ref = field.get("enumRef") or {}
            keys = [v["value"] for v in ref.get("variants", [])]
            name = pascal(ref.get("name") or f"{owner}{pascal(field['name'])}")
        else:
            name = f"{owner}{pascal(field['name'])}"
        keys = [k for k in keys if isinstance(k, str)]
        if not keys:
            return None
        existing = self.enums.get(name)
        if existing is not None and existing != keys:
            name = f"{owner}{pascal(field['name'])}"
            existing = self.enums.get(name)
        if existing is None:
            self.enums[name] = keys
        return name

    def emit_struct(self, name: str, fields: list[dict]) -> None:
        """Emit `name` and everything it nests, then record it."""
        decls: list[str] = []
        inits: list[str] = []
        accessors: list[str] = []

        for field in dedupe(self.flatten(fields, name), name):
            emitted = self.emit_field(name, field, decls, inits, accessors)
            if not emitted:
                drops.append(f"{name}.{field['name']}: {field['method'] or 'mixin'}")

        body = "\n".join(decls) if decls else ""
        lifetime = "<'a>" if ("'a" in body or not decls) else "<'a>"
        struct = [
            f"/// Derived from whatspec's `{name}` shape.",
            "#[derive(Debug, Clone, PartialEq)]",
            "#[non_exhaustive]",
            f"pub struct {name}{lifetime} {{",
        ]
        if decls:
            struct += decls
        struct += [
            "    /// The node this was derived from, for fields the shape does",
            "    /// not model yet.",
            "    pub node: NodeRef<'a>,",
            "}",
            "",
            f"impl<'a> {name}<'a> {{",
            "    /// Derive from a node already known to match this shape.",
            "    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {",
            "        Ok(Self {",
        ]
        struct += [f"            {line}" for line in inits]
        struct += ["            node: *node,", "        })", "    }", "}"]
        if accessors:
            struct += ["", f"impl<'a> {name}<'a> {{"] + accessors + ["}"]
        self.structs.append("\n".join(struct))

    def flatten(self, fields: list[dict], owner: str) -> list[dict]:
        """Inline `sameNode` mixins — they parse the same node, so they are the
        same struct's fields."""
        out: list[dict] = []
        for field in fields:
            if not field.get("method") and field.get("sameNode"):
                out.extend(self.flatten(field.get("children", []), owner))
            else:
                out.append(field)
        return out

    def emit_field(
        self, owner: str, field: dict, decls: list[str], inits: list[str],
        accessors: list[str],
    ) -> bool:
        method = field.get("method") or ""
        name = snake(field["name"])
        key = rust_str(field["name"])
        doc = f"    /// `{field['name']}`, via `{method or 'mixin'}`."

        if method in SCALAR_METHODS:
            ty, call = SCALAR_METHODS[method]
            if not field.get("required") and method in OPTIONAL_JID:
                ty, call = f"Option<{ty}>", OPTIONAL_JID[method]
            decls += [doc, f"    pub {name}: {ty},"]
            inits.append(f"{name}: {call.format(key=key)},")
            return True

        if method in ENUM_METHODS:
            enum_name = self.enum_for(owner, field)
            if enum_name is None:
                return False
            if method == "attrEnumOrNullIfUnknown":
                ty = f"Option<{enum_name}>"
                call = f"extract::attr_enum_or_none(node, {key}, {enum_name}::from_wire)"
            elif method == "maybeAttrEnum" or not field.get("required"):
                ty = f"Option<{enum_name}>"
                call = f"extract::maybe_attr_enum(node, {key}, {enum_name}::from_wire)?"
            else:
                ty = enum_name
                call = f"extract::attr_enum(node, {key}, {enum_name}::from_wire)?"
            decls += [doc, f"    pub {name}: {ty},"]
            inits.append(f"{name}: {call},")
            return True

        if method in CHILD_METHODS:
            child_name = f"{owner}{pascal(field['name'])}"
            self.emit_struct(child_name, field.get("children", []))
            tag = rust_str(field.get("tag") or field["name"])
            if method == "child":
                decls += [doc, f"    pub {name}: alloc::boxed::Box<{child_name}<'a>>,"]
                inits.append(
                    f"{name}: alloc::boxed::Box::new({child_name}::derive("
                    f"&extract::child(node, {tag})?)?),"
                )
            else:
                decls += [doc, f"    pub {name}: Option<alloc::boxed::Box<{child_name}<'a>>>,"]
                inits.append(
                    f"{name}: match extract::maybe_child(node, {tag}) {{ "
                    f"Some(child) => Some(alloc::boxed::Box::new("
                    f"{child_name}::derive(&child)?)), None => None }},"
                )
            return True

        if method in LIST_METHODS:
            item_name = f"{owner}{pascal(field['name'])}"
            self.emit_struct(item_name, field.get("children", []))
            tag = rust_str(field.get("tag") or field["name"])
            accessors += [
                f"    /// Each `<{field.get('tag') or field['name']}>` child, derived lazily.",
                "    ///",
                "    /// An iterator rather than a collection: nothing is allocated, and a",
                "    /// caller that wants only the first does not pay for the rest.",
                f"    pub fn {name}(&self) -> impl Iterator<Item = Result<{item_name}<'a>, DeriveError>> + use<'a> {{",
                f"        extract::children_with_tag(&self.node, {tag})",
                f"            .map(|child| {item_name}::derive(&child))",
                "    }",
            ]
            return True

        return False


def fixture_value(field: dict, full: bool = False) -> str | None:
    """A plausible wire value for one required field."""
    method = field["method"]
    name = rust_str(field["name"])
    if method in {"attrJidWithType", "attrDeviceJid", "attrUserJid"}:
        return f".jid_attr({name}, \"u\")"
    if method in {"attrInt", "attrTime"}:
        return f".attr({name}, \"1\")"
    if method in {"attrEnum", "attrEnumValues"}:
        keys = field.get("enumKeys") or [
            v["value"] for v in (field.get("enumRef") or {}).get("variants", [])
        ]
        keys = [k for k in keys if isinstance(k, str)]
        return f".attr({name}, {rust_str(keys[0])})" if keys else None
    if method == "attrString":
        return f'.attr({name}, "x")'
    if method == "contentBytes":
        return '.bytes(b"x")'
    if method == "contentUint":
        return ".bytes(&[1])"
    if method in {"maybeAttrString"}:
        return f'.attr({name}, "x")'
    if method in {"maybeAttrInt", "maybeAttrTime"}:
        return f'.attr({name}, "1")'
    if method in {"maybeAttrEnum", "attrEnumOrNullIfUnknown"}:
        keys = field.get("enumKeys") or [
            v["value"] for v in (field.get("enumRef") or {}).get("variants", [])
        ]
        keys = [k for k in keys if isinstance(k, str)]
        return f".attr({name}, {rust_str(keys[0])})" if keys else None
    if method in CHILD_METHODS or method in LIST_METHODS:
        tag = field.get("tag") or field["name"]
        return f".child({fixture_for(tag, field.get('children', []), full)})"
    return None


def fixture_for(tag: str, fields: list[dict], full: bool = False) -> str:
    """A builder expression for a stanza carrying this shape's fields.

    `full` includes the optional ones, which is what exercises the `Some` side
    of every optional field — the half a required-only fixture never reaches.
    """
    parts = [f"Fixture::node({rust_str(tag)})"]
    seen: set[str] = set()
    for field in dedupe_quiet(fields):
        if not full and not field.get("required"):
            continue
        # A stanza carries one node per tag here, and one attribute per name.
        key = field.get("tag") or field["name"]
        if key in seen:
            continue
        value = fixture_value(field, full)
        if value:
            seen.add(key)
            parts.append(value)
    return "".join(parts)


def dedupe_quiet(fields: list[dict]) -> list[dict]:
    """`dedupe` without recording drops — fixtures reuse the choice, not the
    reporting."""
    before = len(drops)
    out = dedupe(flatten_same_node(fields), "fixture")
    del drops[before:]
    return out


def flatten_same_node(fields: list[dict]) -> list[dict]:
    out: list[dict] = []
    for field in fields:
        if not field.get("method") and field.get("sameNode"):
            out.extend(flatten_same_node(field.get("children", [])))
        else:
            out.append(field)
    return out


def main() -> None:
    spec = json.loads(SPEC.read_text())
    provenance = json.loads(PROVENANCE.read_text())
    entries = spec["incoming"]

    emitter = Emitter()
    variants: list[tuple[str, str, str]] = []  # (variant, struct, tag)

    for entry in entries:
        parser = entry["shape"].get("parserName") or entry["tag"]
        name = pascal(parser)
        emitter.emit_struct(name, entry["shape"]["fields"])
        variants.append((name, name, entry["tag"]))

    # Assertions beyond the tag are what tell two shapes of the same tag apart.
    # `attr` ones are checkable from the stanza alone; `reference` ones pair a
    # response with the request that caused it, which no single stanza can
    # answer, so they are reported rather than silently treated as satisfied.
    guards: dict[str, list[tuple[str, str]]] = {}
    for entry in entries:
        parser = pascal(entry["shape"].get("parserName") or entry["tag"])
        for assertion in entry["shape"].get("assertions", []):
            kind = assertion.get("kind")
            if kind == "tag":
                continue
            if kind == "attr" and "value" in assertion:
                guards.setdefault(parser, []).append(
                    (assertion["name"], assertion["value"])
                )
            else:
                drops.append(
                    f"{parser}: {kind} assertion on `{assertion.get('name')}` "
                    "needs request context"
                )

    lines = [
        "//! The L1 derivation, generated from whatspec's `incoming` domain.",
        "//!",
        "//! GENERATED FILE — do not edit by hand. Run `tools/generate-l1.py`.",
        "//!",
        "//! Committed rather than produced by a build script so that a protocol change",
        "//! arrives as a reviewable diff, per RFC-009. CI regenerates and requires no",
        "//! change, which rules out drift.",
        "//!",
        "//! Every field here says which extraction primitive produced it. The primitives",
        "//! live in `extract.rs` and are written by hand; this file only chooses among",
        "//! them, which is what keeps a protocol change from becoming a logic change.",
        "",
        "// One arm per shape, even where several share a tag: which shapes a tag has is",
        "// part of what this file records, and collapsing the arms would erase it.",
        "#![allow(clippy::match_same_arms)]",
        "",
        "extern crate alloc;",
        "",
        "use wa_wire_codec::{Jid, NodeRef, Value};",
        "",
        "use crate::error::DeriveError;",
        "use crate::extract;",
        "use crate::provenance::Provenance;",
        "",
        "/// Which whatspec build this derivation came from.",
        "pub const PROVENANCE: Provenance<'static> = Provenance {",
        f"    whatsapp_version: {rust_str(provenance['whatsappVersion'])},",
        f"    schema_version: {rust_str(provenance['schemaVersion'])},",
        f"    generator_version: {rust_str(provenance['generatorVersion'])},",
        f"    incoming_digest: {rust_str(provenance['incomingDigest'])},",
        "};",
        "",
    ]

    for name, keys in sorted(emitter.enums.items()):
        lines += [
            f"/// Wire values for `{name}`.",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]",
            "#[non_exhaustive]",
            f"pub enum {name} {{",
        ]
        for key in keys:
            lines += [f"    /// `{key}`", f"    {pascal(key)},"]
        lines += ["}", "", f"impl {name} {{"]
        lines += [
            "    /// Resolve a wire value, or `None` if this build does not know it.",
            "    #[must_use]",
            "    pub fn from_wire(value: Value<'_>) -> Option<Self> {",
        ]
        for key in keys:
            lines.append(
                f"        if value.eq_str({rust_str(key)}) {{ return Some(Self::{pascal(key)}); }}"
            )
        lines += ["        None", "    }", ""]
        lines += [
            "    /// The wire value this variant carries.",
            "    #[must_use]",
            "    pub const fn as_wire(self) -> &'static str {",
            "        match self {",
        ]
        for key in keys:
            lines.append(f"            Self::{pascal(key)} => {rust_str(key)},")
        lines += ["        }", "    }", "}", ""]

    lines += emitter.structs
    lines.append("")

    # The event union and its dispatcher.
    lines += [
        "/// One derived stanza.",
        "///",
        "/// A tag with several shapes tries each in order and takes the first that",
        "/// derives cleanly, which is how whatspec models a tag whose meaning depends",
        "/// on the fields present.",
        "#[derive(Debug, Clone, PartialEq)]",
        "#[non_exhaustive]",
        "pub enum Event<'a> {",
    ]
    for variant, struct, tag in variants:
        lines += [
            f"    /// A `<{tag}>` matching the `{struct}` shape.",
            f"    {variant}({struct}<'a>),",
        ]
    lines += ["}", "", "impl<'a> Event<'a> {"]
    lines += [
        "    /// The stanza tag this event was derived from.",
        "    #[must_use]",
        "    pub const fn tag(&self) -> &'static str {",
        "        match self {",
    ]
    for variant, _struct, tag in variants:
        lines.append(f"            Self::{variant}(_) => {rust_str(tag)},")
    lines += ["        }", "    }", "", ]
    lines += [
        "    /// The node this event was derived from.",
        "    #[must_use]",
        "    pub const fn node(&self) -> &NodeRef<'a> {",
        "        match self {",
    ]
    for variant, _struct, _tag in variants:
        lines.append(f"            Self::{variant}(inner) => &inner.node,")
    lines += ["        }", "    }", "}", ""]

    # Field counts decide the order two shapes of one tag are tried in. Without
    # it the most permissive shape wins every time — a call receipt would claim
    # every message receipt, because its required fields are a subset. Richest
    # first means the most informative reading that still fits is the one taken.
    field_counts = {
        pascal(e["shape"].get("parserName") or e["tag"]): len(e["shape"]["fields"])
        for e in entries
    }

    by_tag: dict[str, list[tuple[str, str]]] = {}
    for variant, struct, tag in variants:
        by_tag.setdefault(tag, []).append((variant, struct))
    for shapes in by_tag.values():
        shapes.sort(
            key=lambda s: (len(guards.get(s[1], [])), field_counts.get(s[1], 0)),
            reverse=True,
        )

    lines += [
        "/// Derive an event from a parsed stanza.",
        "///",
        "/// Pure: the same node yields the same event, with no key material and no",
        "/// accumulated state. That is what lets this run host-side, once, instead of",
        "/// being reimplemented per engine.",
        "pub fn derive<'a>(node: &NodeRef<'a>) -> Result<Event<'a>, DeriveError> {",
    ]
    for tag, shapes in sorted(by_tag.items()):
        lines.append(f"    if node.tag().eq_str({rust_str(tag)}) {{")
        for variant, struct in shapes:
            checks = guards.get(struct, [])
            guard = "".join(
                f"node.attr_eq({rust_str(k)}, {rust_str(v)}) && " for k, v in checks
            )
            if guard:
                lines.append(f"        // guarded by {', '.join(f'{k}={v}' for k, v in checks)}")
            lines.append(
                f"        if {guard}let Ok(inner) = {struct}::derive(node) {{ "
                f"return Ok(Event::{variant}(inner)); }}"
            )
        lines.append(f"        return Err(DeriveError::NoMatchingShape {{ tag: {rust_str(tag)} }});")
        lines.append("    }")
    lines += [
        "    Err(DeriveError::UnknownStanza)",
        "}",
        "",
        "/// Tags this build can derive.",
        "pub const KNOWN_TAGS: [&str; " + str(len(by_tag)) + "] = [",
    ]
    lines += [f"    {rust_str(tag)}," for tag in sorted(by_tag)]
    lines += ["];", ""]

    # What the generator could not express, named rather than omitted.
    lines += [
        "/// Fields the generator could not express, named rather than dropped in",
        "/// silence.",
        "///",
        "/// A derivation that quietly omitted a field would look complete and be",
        "/// wrong, and no conformance run could tell — every engine would agree on",
        "/// the same missing field.",
        "pub const UNMODELLED_FIELDS: [&str; " + str(len(drops)) + "] = [",
    ]
    lines += [f"    {rust_str(d)}," for d in sorted(set(drops))[: len(drops)]]
    lines += ["];", ""]

    # Generated tests for generated code. Writing them by hand would mean 16
    # fixtures kept in step with 16 shapes by memory; deriving both from the
    # same source is the only way they cannot drift. Each one asserts that the
    # shape matches a stanza built from its own required fields — which is
    # exactly the claim the generator makes and cannot check itself.
    lines += [
        "#[cfg(test)]",
        "mod generated_tests {",
        "    use super::*;",
        "    use crate::testing::{Fixture, parse};",
        "",
    ]
    for entry in entries:
        name = pascal(entry["shape"].get("parserName") or entry["tag"])
        builder = fixture_for(entry["tag"], entry["shape"]["fields"])
        lines += [
            f"    /// `{name}` derives from a stanza carrying its required fields.",
            "    #[test]",
            f"    fn {snake(name)}_derives_from_its_required_fields() {{",
            f"        let stanza = {builder}.build();",
            "        let node = parse(&stanza);",
            f"        let derived = {name}::derive(&node);",
            f'        assert!(derived.is_ok(), "{name}: {{:?}}", derived.err());',
            "        // Derivation is pure, so a second run must agree.",
            f"        assert_eq!(derived, {name}::derive(&node));",
            "    }",
            "",
            f"    /// `{name}` derives when every field it models is present.",
            "    ///",
            "    /// The required-only fixture never reaches the `Some` side of an",
            "    /// optional field; this one does.",
            "    #[test]",
            f"    fn {snake(name)}_derives_with_every_field_present() {{",
            f"        let stanza = {fixture_for(entry['tag'], entry['shape']['fields'], True)}.build();",
            "        let node = parse(&stanza);",
            f"        let derived = {name}::derive(&node);",
            f'        assert!(derived.is_ok(), "{name}: {{:?}}", derived.err());',
            "    }",
            "",
        ]
    for enum_name, keys in sorted(emitter.enums.items()):
        lines += [
            f"    /// Every `{enum_name}` value round-trips through the wire form.",
            "    #[test]",
            f"    fn {snake(enum_name)}_round_trips() {{",
            "    #[allow(clippy::single_element_loop)]",
            f"        for variant in [{', '.join(f'{enum_name}::{pascal(k)}' for k in keys)}] {{",
            "            let wire = variant.as_wire();",
            "            assert!(!wire.is_empty());",
            "            let stanza = Fixture::node(\"n\").attr(\"v\", wire).build();",
            "            let node = parse(&stanza);",
            "            let value = node.attr(\"v\").expect(\"the attribute\");",
            f"            assert_eq!({enum_name}::from_wire(value), Some(variant));",
            "        }",
            f'        let stanza = Fixture::node("n").attr("v", "not-a-variant").build();',
            "        let node = parse(&stanza);",
            f'        assert_eq!({enum_name}::from_wire(node.attr("v").expect("attr")), None);',
            "    }",
            "",
        ]
    lines += [
        "    /// Every tag dispatches, and an unmodelled one is reported as such.",
        "    #[test]",
        "    fn dispatch_covers_every_known_tag() {",
        "        for tag in KNOWN_TAGS {",
        "            let stanza = Fixture::node(tag).build();",
        "            // A bare stanza matches no shape, but the tag is recognised —",
        "            // which is the distinction `derive` exists to make.",
        "            assert_ne!(",
        "                derive(&parse(&stanza)),",
        "                Err(DeriveError::UnknownStanza),",
        "                \"{tag} is in KNOWN_TAGS but does not dispatch\"",
        "            );",
        "        }",
        "    }",
        "}",
        "",
    ]

    TARGET.parent.mkdir(parents=True, exist_ok=True)
    TARGET.write_text("\n".join(lines))
    subprocess.run(["rustfmt", "--edition", "2024", str(TARGET)], check=True)

    print(f"{TARGET.relative_to(ROOT)}: {len(variants)} shapes, "
          f"{len(emitter.enums)} enums, {len(by_tag)} tags, {len(set(drops))} unmodelled")
    if drops:
        for drop in sorted(set(drops)):
            print(f"  unmodelled: {drop}", file=sys.stderr)


if __name__ == "__main__":
    main()
