#!/usr/bin/env python3
"""Generate the outgoing derivation from whatspec's `stanza` and `iq` domains.

`generate-l1.py` covers what arrives, out of the `incoming` domain. This covers
what leaves, and needs two sources because whatspec describes outbound traffic
in two places:

- `stanza/index.json` — every non-`<iq>` stanza the client builds: acks,
  receipts, messages, presence. `direction` is `outgoing` for all of them.
- `iq/index.json` — the `<iq>` request builders, whose `request` half says which
  namespace, type and children each one carries. The rest of that file is
  response parsers, which are not this generator's business.

# Why the outbound side needs its own derivation at all

The `incoming` domain records how WA Web *parses* what the server sends. An
outbound stanza wears the same tags and means the opposite: an `<ack>` inbound
is the server acknowledging our send, outbound it is us acknowledging a
delivery. Feeding one to the inbound derivation does not fail — it produces a
confident wrong answer, which two engines can agree on.

# What these domains describe

Builders, not parsers. An attribute carries a `kind` saying how the *sender*
produces it (`const`, `dynamic`, `generated_id`, a JID flavour), where the
`incoming` domain names the accessor a reader calls. Read backwards, a builder
is still a shape: a `const` is a value that must be there, and everything else
is a field whose presence the `required` flag decides.

That asymmetry is the reason this is a separate generator rather than a flag on
the other one. The two domains answer different questions and only look alike.

    python3 tools/generate-outgoing.py
"""

from __future__ import annotations

import hashlib
import json
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
CRATE = ROOT / "crates" / "wa-wire-l1"
OUTGOING = CRATE / "spec" / "outgoing.json"
IQ = CRATE / "spec" / "iq.json"
PROVENANCE = CRATE / "spec" / "provenance.json"
TARGET = CRATE / "src" / "generated" / "outgoing.rs"

# How a builder's attribute reads back. The `kind` says how the sender produces
# the value; a reader only cares what shape arrives.
#
# `dynamic` and `generated_id` are computed at build time — a timestamp, a
# message id — and are plain strings on the wire. Reading them as anything more
# specific would be inventing a type the spec does not state.
KIND_READERS = {
    "string": ("Value<'a>", "extract::attr_string(node, {key})?", "maybe_attr_string"),
    "optional": ("Value<'a>", "extract::attr_string(node, {key})?", "maybe_attr_string"),
    "dynamic": ("Value<'a>", "extract::attr_string(node, {key})?", "maybe_attr_string"),
    "generated_id": ("Value<'a>", "extract::attr_string(node, {key})?", "maybe_attr_string"),
    "integer": ("i64", "extract::attr_int(node, {key})?", "maybe_attr_int"),
    # The flavours stay distinct. Two builders differ in nothing but this —
    # an `<ack class="notification">` to a device is an identity change, to a
    # user it is a device notification — so collapsing them into "a JID" makes
    # one shape out of two and lets either claim the other's stanza.
    "user_jid": ("Jid<'a>", "extract::attr_user_jid(node, {key})?", "maybe_attr_jid"),
    "group_jid": ("Jid<'a>", "extract::attr_group_jid(node, {key})?", "maybe_attr_jid"),
    "device_jid": ("Jid<'a>", "extract::attr_device_jid(node, {key})?", "maybe_attr_jid"),
}

OPTIONAL_TYPES = {
    "Value<'a>": ("Option<Value<'a>>", "extract::maybe_attr_string(node, {key})"),
    "i64": ("Option<i64>", "extract::maybe_attr_int(node, {key})?"),
    "Jid<'a>": ("Option<Jid<'a>>", "extract::maybe_attr_jid(node, {key})?"),
}

RESERVED = {
    "type", "self", "ref", "match", "move", "box", "final", "override", "abstract",
    "as", "async", "await", "become", "do", "fn", "for", "if", "in", "let", "loop",
    "mod", "priv", "pub", "static", "struct", "super", "trait", "true", "false",
    "typeof", "unsafe", "use", "where", "while", "yield", "impl", "enum", "const",
    "continue", "crate", "else", "extern", "return", "break", "macro", "virtual",
}

# What could not be expressed, named rather than dropped in silence — the same
# rule the other two generators follow, and for the same reason: a derivation
# that quietly omitted a field would look complete and be wrong.
drops: list[str] = []


def require(condition: bool, message: str) -> None:
    if not condition:
        sys.exit(f"generate-outgoing: {message}")


def snake(name: str) -> str:
    out: list[str] = []
    for index, char in enumerate(name):
        if char in "-.:/ ":
            out.append("_")
        elif char.isupper():
            if index and name[index - 1] not in "-._ ":
                out.append("_")
            out.append(char.lower())
        else:
            out.append(char)
    text = "".join(out).replace("__", "_").strip("_") or "field"
    if text in RESERVED or text[0].isdigit():
        text = f"r#{text}" if text in RESERVED else f"n{text}"
    return text


def pascal(name: str) -> str:
    parts = [p for p in name.replace("-", "_").replace(".", "_").replace(":", "_").split("_") if p]
    return "".join(p[:1].upper() + p[1:] for p in parts) or "Shape"


def rust_str(value: str) -> str:
    escaped = value.replace("\\", "\\\\").replace('"', '\\"')
    return f'"{escaped}"'


def consts_of(attrs: list[dict]) -> list[tuple[str, str]]:
    """The attributes a builder always writes with the same value.

    These are the discriminators: `class="receipt"` is not data a reader wants
    back, it is how this shape is told from the next one.
    """
    return [
        (a["name"], a["value"])
        for a in attrs
        if a.get("kind") == "const" and a.get("value") is not None
    ]


def readable(attrs: list[dict]) -> list[dict]:
    """Attributes that carry data, in spec order. `const` ones are guards."""
    return [a for a in attrs if a.get("kind") != "const"]


class Emitter:
    def __init__(self) -> None:
        self.structs: list[str] = []
        self.emitted: set[str] = set()

    def emit_struct(
        self, name: str, attrs: list[dict], children: list[dict], owner: str,
        tag_hint: str = "", is_child: bool = False,
    ) -> None:
        if name in self.emitted:
            return
        self.emitted.add(name)

        decls: list[str] = []
        inits: list[str] = []
        comparisons: list[str] = []
        seen: set[str] = set()

        # A `const` on a CHILD is enforced here; one on the stanza itself is
        # not.
        #
        # Dispatch can only guard on the stanza's own attributes, so a pin on a
        # child is invisible to it: two `abt get` requests differ only in what
        # their `<props>` child pins, and a shape that ignored it accepted the
        # other one's stanza and claimed it first.
        #
        # At the top level the same check is dead code — dispatch has already
        # tested every one of these before calling `derive`, and there is no
        # other caller. Emitting it anyway would leave five hundred branches
        # that only a bug could reach, which D-030 says to remove rather than
        # to cover.
        checks = (
            [
                f"if !node.attr_eq({rust_str(key)}, {rust_str(value)}) {{ "
                f"return Err(DeriveError::NoMatchingShape {{ tag: {rust_str(tag_hint)} }}); }}"
                for key, value in consts_of(attrs)
            ]
            if is_child
            else []
        )

        for attr in readable(attrs):
            field = snake(attr["name"])
            if field in seen:
                continue
            seen.add(field)
            kind = attr.get("kind") or ""
            if kind not in KIND_READERS:
                drops.append(f"{owner}.{attr['name']}: {kind or 'no kind'}")
                continue
            ty, call, _ = KIND_READERS[kind]
            key = rust_str(attr["name"])
            if not attr.get("required"):
                ty, call = OPTIONAL_TYPES[ty]
            decls += [
                f"    /// `{attr['name']}`, a `{kind}` attribute.",
                f"    pub {field}: {ty},",
            ]
            inits.append(f"{field}: {call.format(key=key)},")
            comparisons.append(compare_of(field, ty))

        for child in children:
            field = snake(child["tag"])
            if field in seen:
                continue
            seen.add(field)
            child_name = f"{name}{pascal(child['tag'])}"
            self.emit_struct(
                child_name, child.get("attrs", []), child.get("children", []), owner,
                child["tag"], is_child=True,
            )
            tag = rust_str(child["tag"])
            decls += [
                f"    /// The `<{child['tag']}>` child.",
                f"    pub {field}: alloc::boxed::Box<{child_name}<'a>>,",
            ]
            inits.append(
                f"{field}: alloc::boxed::Box::new({child_name}::derive("
                f"&extract::child(node, {tag})?)?),"
            )
            comparisons.append(f"self.{field}.semantic_eq(&other.{field})")

        lines = [
            f"/// Derived from whatspec's `{owner}` builder.",
            "#[derive(Debug, Clone, PartialEq)]",
            "#[non_exhaustive]",
            f"pub struct {name}<'a> {{",
            *decls,
            "    /// The node this was derived from, for fields the shape does",
            "    /// not model yet.",
            "    pub node: NodeRef<'a>,",
            "}",
            "",
            f"impl<'a> {name}<'a> {{",
            "    /// Derive from a node already known to match this shape.",
            "    ///",
            "    /// # Errors",
            "    ///",
            "    /// When a field the builder always writes is absent.",
            "    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {",
            *[f"        {check}" for check in checks],
            "        Ok(Self {",
            *[f"            {line}" for line in inits],
            "            node: *node,",
            "        })",
            "    }",
            "}",
            "",
            f"impl {name}<'_> {{",
            "    /// Whether two derivations mean the same thing.",
            "    ///",
            "    /// The originating node is excluded: two engines may encode one",
            "    /// stanza differently and both be right.",
            "    #[must_use]",
            f"    pub fn semantic_eq(&self, {'other' if comparisons else '_other'}: &{name}<'_>) -> bool {{",
        ]
        if len(comparisons) == 1:
            lines.append(f"        {comparisons[0]}")
        elif comparisons:
            lines.append("        " + "\n            && ".join(f"({c})" for c in comparisons))
        else:
            lines.append("        true")
        lines += ["    }", "}"]
        self.structs.append("\n".join(lines))


def compare_of(field: str, ty: str) -> str:
    """How one field is compared.

    `Value` and `Jid` are `Copy` and compare by value, not by reference: their
    text may exist nowhere in the buffer — a packed digit run, a JID in parts —
    so `semantic_eq` walks the parts rather than comparing bytes.
    """
    if ty in {"Option<Value<'a>>", "Option<Jid<'a>>"}:
        return (
            f"match (self.{field}, other.{field}) {{ "
            f"(Some(a), Some(b)) => a.semantic_eq(b), "
            f"(None, None) => true, _ => false }}"
        )
    if ty in {"Value<'a>", "Jid<'a>"}:
        return f"self.{field}.semantic_eq(other.{field})"
    return f"self.{field} == other.{field}"


FIXTURE_VALUES = {
    "string": '.attr({key}, "x")',
    "optional": '.attr({key}, "x")',
    "dynamic": '.attr({key}, "x")',
    "generated_id": '.attr({key}, "x")',
    "integer": '.attr({key}, "1")',
    "user_jid": '.jid_attr({key}, "u")',
    "group_jid": '.group_jid_attr({key}, "u")',
    "device_jid": '.device_jid_attr({key}, "u", 1)',
}


def fixture_for(shape: dict, full: bool = False, break_pin: bool = False) -> str:
    """A builder expression for a stanza this shape's builder would produce.

    `full` writes the optional attributes too, which is the only thing that
    reaches the `Some` side of an optional field and the comparison arm that
    reads it.
    """
    return f"Fixture::node({rust_str(shape['stanzaType'])})" + fixture_body(
        shape["attrs"], shape["children"], full, break_pin
    )


def fixture_body(
    attrs: list[dict], children: list[dict], full: bool = False, break_pin: bool = False
) -> str:
    parts: list[str] = []
    seen: set[str] = set()
    # Guards first: a `const` pins an exact value, and the generic one a
    # non-const attribute of the same name would write contradicts it.
    for key, value in consts_of(attrs):
        seen.add(key)
        # `break_pin` contradicts the first pin and leaves the rest, so the
        # stanza differs from this shape in exactly one thing.
        if break_pin:
            break_pin = False
            parts.append(f".attr({rust_str(key)}, {rust_str(value + '-not')})")
        else:
            parts.append(f".attr({rust_str(key)}, {rust_str(value)})")
    for attr in readable(attrs):
        if attr["name"] in seen or (not full and not attr.get("required")):
            continue
        template = FIXTURE_VALUES.get(attr.get("kind") or "")
        if template:
            seen.add(attr["name"])
            parts.append(template.format(key=rust_str(attr["name"])))
    for child in children:
        parts.append(
            f".child(Fixture::node({rust_str(child['tag'])})"
            + fixture_body(child.get("attrs", []), child.get("children", []), full)
            + ")"
        )
    return "".join(parts)


# Which kinds a fixture built for one kind satisfies. A `string` and a
# `dynamic` are the same thing on the wire — the difference is whether the
# builder was handed the value or computed it — so a shape demanding one
# accepts a stanza built for the other. The JID flavours do not interchange,
# which is what makes them useful discriminators.
SATISFIES = {
    "string": {"string", "dynamic", "generated_id", "optional"},
    "dynamic": {"string", "dynamic", "generated_id", "optional"},
    "generated_id": {"string", "dynamic", "generated_id", "optional"},
    "optional": {"string", "dynamic", "generated_id", "optional"},
    "integer": {"integer"},
    "user_jid": {"user_jid"},
    "device_jid": {"device_jid"},
    "group_jid": {"group_jid"},
}


def written(attrs: list[dict], children: list[dict]) -> tuple[dict, dict]:
    """What a shape's fixture puts on the node: attributes, then children."""
    out: dict[str, tuple[str, str | None]] = {}
    for key, value in consts_of(attrs):
        out[key] = ("const", value)
    for attr in readable(attrs):
        if attr.get("required") and attr["name"] not in out:
            out[attr["name"]] = (attr.get("kind") or "", None)
    kids = {c["tag"]: c for c in children}
    return out, kids


def accepts(shape_attrs: list[dict], shape_children: list[dict], other: dict) -> bool:
    """Whether a shape's `derive` succeeds on another shape's fixture."""
    their_attrs, their_children = other
    for key, value in consts_of(shape_attrs):
        if their_attrs.get(key) != ("const", value):
            return False
    for attr in readable(shape_attrs):
        if not attr.get("required"):
            continue
        theirs = their_attrs.get(attr["name"])
        if theirs is None:
            return False
        kind, _ = theirs
        if kind == "const":
            # A pinned value is a string on the wire; only a string-ish demand
            # is satisfied by it.
            if attr.get("kind") not in SATISFIES["string"]:
                return False
            continue
        if kind not in SATISFIES.get(attr.get("kind") or "", set()):
            return False
    for child in shape_children:
        theirs = their_children.get(child["tag"])
        if theirs is None:
            return False
        if not accepts(
            child.get("attrs", []),
            child.get("children", []),
            written(theirs.get("attrs", []), theirs.get("children", [])),
        ):
            return False
    return True


def indistinguishable(a: dict, b: dict) -> bool:
    """Whether two shapes produce stanzas nothing can tell apart.

    Mutual, not one-way: a shape strictly subsumed by another is unreachable
    but still a different shape, and merging it would throw away fields the
    other does not model. Two that each accept the other's stanzas are one
    shape described twice.
    """
    return accepts(
        a["attrs"], a["children"], written(b["attrs"], b["children"])
    ) and accepts(b["attrs"], b["children"], written(a["attrs"], a["children"]))


def merge_indistinguishable(
    named: dict[str, dict],
) -> tuple[dict[str, dict], list[tuple[str, str]]]:
    """Fold shapes that are one stanza described by two builders.

    whatspec records a module per builder, and two modules can build the same
    stanza while differing in something invisible to a reader: whether a value
    is handed in or computed, or whether one of them bothers to model an
    optional attribute. Keeping both would be two types no stanza can choose
    between, and the earlier would silently claim every stanza meant for the
    later.

    The richer one survives, since it models fields the other does not, and it
    keeps its own name. Folded, not dropped: what merged is reported.

    Computed from the spec on every run, so it un-merges by itself the day
    whatspec records something that separates them.
    """
    kept: dict[str, dict] = {}
    merged: list[tuple[str, str]] = []
    for name, shape in named.items():
        for other_name, other in kept.items():
            if other["stanzaType"] != shape["stanzaType"]:
                continue
            if not indistinguishable(shape, other):
                continue
            # The one modelling more fields survives.
            if len(readable(shape["attrs"])) > len(readable(other["attrs"])):
                kept[other_name] = shape
                merged.append((other_name, name))
            else:
                merged.append((name, other_name))
            break
        else:
            kept[name] = shape
    return kept, merged


def unreachable_shapes(by_tag: dict[str, list[tuple[str, dict]]]) -> dict[str, str]:
    """Shapes an earlier shape in dispatch order always claims first.

    Not a defect to fix by reordering: these differ in nothing a reader can
    see. `HandleGrowthNotification` and `HandleBotProfileNotification` are one
    `<ack class="notification">` with `type` computed one way or the other, and
    which way is the builder's business. A generator that emitted a test for
    each would emit one that can never pass, so it says so instead.
    """
    out: dict[str, str] = {}
    for entries in by_tag.values():
        for index, (name, shape) in enumerate(entries):
            mine = written(shape["attrs"], shape["children"])
            for earlier_name, earlier in entries[:index]:
                if accepts(earlier["attrs"], earlier["children"], mine):
                    out[name] = earlier_name
                    break
    return out


def stanza_shapes(spec: dict) -> list[dict]:
    """Outgoing non-`<iq>` stanzas, deduplicated by the shape they describe.

    Several modules build the same stanza — five signatures appear twice — and
    each would otherwise become a distinct type that no stanza could ever be
    told from its twin.

    Fragments are skipped: a mixin folded into a builder is not a stanza anyone
    sends on its own, and whatspec marks it.
    """
    out: dict[tuple, dict] = {}
    for stanza in spec["stanzas"]:
        if stanza.get("fragment") or stanza.get("direction") != "outgoing":
            continue
        signature = (
            stanza["stanzaType"],
            tuple(sorted((a["name"], a.get("kind"), a.get("value"), a.get("required"))
                         for a in stanza["attrs"])),
            tuple(sorted(c["tag"] for c in stanza["children"])),
        )
        out.setdefault(signature, stanza)
    return list(out.values())


def child_signature(children: list[dict]) -> tuple:
    """What a child tree looks like on the wire, for telling two builders apart."""
    return tuple(
        sorted(
            (
                c["tag"],
                tuple(sorted((a["name"], a.get("kind"), a.get("value"), a.get("required"))
                             for a in c.get("attrs", []))),
                child_signature(c.get("children", [])),
            )
            for c in children
        )
    )


def iq_shapes(spec: dict) -> list[dict]:
    """`<iq>` requests, as stanza-shaped records.

    The `request` half names the namespace and type separately from the
    attributes, because a builder sets them from its own arguments. On the wire
    they are `xmlns` and `type`, so that is where they go — and `to`, when the
    spec pins a target.

    Deduplicated by the stanza produced, not by the module producing it.
    `PrivacyGetContactBlacklistRequest` and `QueryPrivacySettingsJob` are two
    modules emitting the same `<iq xmlns="privacy" type="get"><privacy/></iq>`;
    keeping both would be two types no reader could ever tell apart, and the
    first would silently claim every stanza meant for the second.
    """
    seen: dict[tuple, str] = {}
    out: list[dict] = []
    for stanza in spec["stanzas"]:
        request = stanza.get("request")
        if not request:
            continue
        attrs = [
            {"name": "xmlns", "kind": "const", "value": request["namespace"], "required": True},
            {"name": "type", "kind": "const", "value": request["iqType"], "required": True},
        ]
        if request.get("target"):
            attrs.append(
                {"name": "to", "kind": "const", "value": request["target"], "required": True}
            )
        signature = (
            request["namespace"],
            request["iqType"],
            request.get("target"),
            child_signature(request.get("children", [])),
        )
        if signature in seen:
            continue
        seen[signature] = stanza["moduleName"]
        out.append(
            {
                "stanzaType": "iq",
                "moduleName": stanza["moduleName"],
                "attrs": attrs,
                "children": request.get("children", []),
                "name": stanza.get("exportedFunction") or stanza["moduleName"],
            }
        )
    return out


def name_of(shape: dict) -> str:
    """A type name for a shape, from the module that builds it.

    The module name is the only thing unique per shape — two builders of one
    stanza type differ in nothing else a reader can see. Stripped of the
    `WASmaxOut`/`WAWeb` prefixes the bundle puts on everything, which carry no
    information once every name has them.
    """
    raw = shape.get("moduleName") or shape.get("name") or "Shape"
    for prefix in ("WASmaxOut", "WASmaxIn", "WASmax", "WAWeb", "WA"):
        if raw.startswith(prefix):
            raw = raw[len(prefix) :]
            break
    return pascal(raw)


# How much a kind narrows what it accepts. A `dynamic` attribute is any string,
# so a shape declaring one accepts everything a `device_jid` shape does —
# `HandleDigestKey` and `HandleIdentityChange` differ in nothing else, and
# ordering by count alone let the looser one claim the stricter one's stanzas.
NARROWNESS = {
    "device_jid": 4,
    "group_jid": 4,
    "user_jid": 3,
    "integer": 3,
    "string": 1,
    "generated_id": 1,
    "dynamic": 0,
    "optional": 0,
}


def demands(attrs: list[dict], children: list[dict]) -> tuple[int, int, int, int]:
    """What a shape insists on, counted over the whole tree.

    Over the whole tree because that is where the discrimination often is:
    `SetReadReceiptJob` is `SetPrivacyJob` with `category/@name` pinned to
    `readreceipts`, two levels down, and an ordering that looked only at the
    top saw two shapes with one child each and picked the wrong one.
    """
    consts = len(consts_of(attrs))
    required = [a for a in readable(attrs) if a.get("required")]
    narrowness = sum(NARROWNESS.get(a.get("kind") or "", 0) for a in required)
    count = len(required)
    kids = len(children)
    for child in children:
        c, k, n, r = demands(child.get("attrs", []), child.get("children", []))
        consts += c
        kids += k
        narrowness += n
        count += r
    return consts, kids, narrowness, count


def specificity(shape: dict) -> tuple:
    """Richest first, for the same reason `incoming` orders its shapes (D-041).

    A shape whose required attributes are a subset of another's — or are the
    same attributes at looser types — accepts every stanza that one does.
    Trying the loose one first would claim them all and the strict one would
    never derive.
    """
    consts, kids, narrowness, required = demands(shape["attrs"], shape["children"])
    return (-consts, -kids, -required, -narrowness, -len(shape["attrs"]))


def main() -> None:
    outgoing_bytes = OUTGOING.read_bytes()
    iq_bytes = IQ.read_bytes()
    digests = {
        "outgoingDigest": "sha256:" + hashlib.sha256(outgoing_bytes).hexdigest(),
        "iqDigest": "sha256:" + hashlib.sha256(iq_bytes).hexdigest(),
    }
    recorded = json.loads(PROVENANCE.read_text())
    for key, value in digests.items():
        require(
            recorded.get(key) == value,
            f"{key}: the vendored spec hashes to {value}, provenance says {recorded.get(key)}",
        )

    shapes = stanza_shapes(json.loads(outgoing_bytes)) + iq_shapes(json.loads(iq_bytes))
    require(len(shapes) > 100, f"only {len(shapes)} outgoing shapes; the scan is wrong")

    # One type per shape, and a name collision would silently merge two.
    named: dict[str, dict] = {}
    for shape in shapes:
        name = name_of(shape)
        suffix = 2
        while name in named:
            name = f"{name_of(shape)}{suffix}"
            suffix += 1
        named[name] = shape

    named, merged = merge_indistinguishable(named)

    emitter = Emitter()
    for name, shape in named.items():
        emitter.emit_struct(
            name, shape["attrs"], shape["children"], shape.get("moduleName", name),
            shape["stanzaType"],
        )

    by_tag: dict[str, list[tuple[str, dict]]] = {}
    for name, shape in named.items():
        by_tag.setdefault(shape["stanzaType"], []).append((name, shape))
    for tag in by_tag:
        by_tag[tag].sort(key=lambda pair: specificity(pair[1]))
    shadowed = unreachable_shapes(by_tag)

    lines = [
        "//! The outgoing derivation, generated from whatspec's `stanza` and `iq`",
        "//! domains.",
        "//!",
        "//! GENERATED FILE — do not edit by hand. Run `tools/generate-outgoing.py`.",
        "//!",
        "//! [`crate::generated`] derives what arrives. This derives what leaves, and",
        "//! it has to be a separate derivation rather than the same one applied twice:",
        "//! an outbound stanza wears the same tags as an inbound one and means the",
        "//! opposite. An `<ack>` inbound is the server acknowledging our send;",
        "//! outbound it is us acknowledging a delivery. Feeding one to the other",
        "//! derivation does not fail — it answers confidently and wrongly, which two",
        "//! engines can agree on.",
        "",
        # A tag with a single unguarded shape emits `if tag { if let Ok(..) }`,
        # which clippy would collapse. Left as it is: the outer test is per tag
        # and the inner per shape, and flattening the one case where a tag has
        # exactly one shape would make the dispatch read differently depending
        # on how many shapes a tag happens to have.
        "#![allow(clippy::collapsible_if, clippy::too_many_lines, clippy::match_same_arms)]",
        "",
        "extern crate alloc;",
        "",
        "use wa_wire_codec::{Jid, NodeRef, Value};",
        "",
        "use crate::error::DeriveError;",
        "use crate::extract;",
        "",
    ]
    lines += [s + "\n" for s in emitter.structs]

    # The event enum.
    lines += [
        "/// One stanza the client sent, typed.",
        "#[derive(Debug, Clone, PartialEq)]",
        "#[non_exhaustive]",
        "pub enum OutgoingEvent<'a> {",
    ]
    for name in named:
        lines += [f"    /// `{name}`.", f"    {name}({name}<'a>),"]
    lines += ["}", ""]

    lines += [
        "impl OutgoingEvent<'_> {",
        "    /// The stanza tag this event was derived from.",
        "    #[must_use]",
        "    pub const fn tag(&self) -> &'static str {",
        "        match self {",
    ]
    for name, shape in named.items():
        lines.append(f"            Self::{name}(_) => {rust_str(shape['stanzaType'])},")
    lines += ["        }", "    }", ""]

    lines += [
        "    /// The name of the shape this event was derived as.",
        "    ///",
        "    /// The tag alone does not identify a shape — forty-three different",
        "    /// `<ack>` builders exist — so a divergence reported by tag leaves a",
        "    /// reader unable to say which two things disagreed.",
        "    #[must_use]",
        "    pub const fn shape(&self) -> &'static str {",
        "        match self {",
    ]
    for name in named:
        lines.append(f"            Self::{name}(_) => {rust_str(name)},")
    lines += ["        }", "    }", ""]

    lines += [
        "    /// Whether two derivations mean the same thing.",
        "    ///",
        "    /// Two different shapes never do: a stanza that derives one way for one",
        "    /// engine and another way for another is the divergence, not a detail.",
        "    #[must_use]",
        "    pub fn semantic_eq(&self, other: &OutgoingEvent<'_>) -> bool {",
        "        match (self, other) {",
    ]
    for name in named:
        lines.append(
            f"            (OutgoingEvent::{name}(a), OutgoingEvent::{name}(b)) => a.semantic_eq(b),"
        )
    lines += ["            _ => false,", "        }", "    }", "}", ""]

    # Dispatch.
    lines += [
        "/// Derive a typed event from a stanza the client sent.",
        "///",
        "/// # Errors",
        "///",
        "/// [`DeriveError::UnknownStanza`] when no builder in the spec produces a",
        "/// stanza of this shape. Common rather than exceptional: the spec covers",
        "/// what WA Web builds, and an engine is entitled to send something it does",
        "/// not — which is itself worth seeing rather than guessing at.",
        "pub fn derive_outgoing<'a>(node: &NodeRef<'a>) -> Result<OutgoingEvent<'a>, DeriveError> {",
    ]
    for tag, entries in sorted(by_tag.items()):
        lines.append(f"    if node.tag().eq_str({rust_str(tag)}) {{")
        for name, shape in entries:
            guards = consts_of(shape["attrs"])
            condition = " && ".join(
                f"node.attr_eq({rust_str(k)}, {rust_str(v)})" for k, v in guards
            )
            if condition:
                lines.append("        // guarded by " + ", ".join(f"{k}={v}" for k, v in guards))
                lines.append(f"        if {condition}")
                lines.append(f"            && let Ok(inner) = {name}::derive(node)")
                lines.append("        {")
            else:
                lines.append(f"        if let Ok(inner) = {name}::derive(node) {{")
            lines.append(f"            return Ok(OutgoingEvent::{name}(inner));")
            lines.append("        }")
        lines.append("    }")
    lines += ["    Err(DeriveError::UnknownStanza)", "}", ""]

    # Tags, and what could not be expressed.
    tags = sorted(by_tag)
    lines += [
        "/// Every stanza tag the outgoing derivation models.",
        f"pub const OUTGOING_TAGS: [&str; {len(tags)}] = [",
        *[f"    {rust_str(t)}," for t in tags],
        "];",
        "",
        "/// Builder attributes this generator could not express, named rather than",
        "/// dropped in silence.",
        f"pub const UNMODELLED_OUTGOING: [&str; {len(sorted(set(drops)))}] = [",
        *[f"    {rust_str(d)}," for d in sorted(set(drops))],
        "];",
        "",
        "/// Builders that produce the same stanza as another, folded into it.",
        "///",
        "/// whatspec records a module per builder, and two modules can build one",
        "/// stanza while differing in something no reader can see: whether a value",
        "/// is handed in or computed, or whether one of them models an optional",
        "/// attribute the other leaves out. The pair is `(folded, survivor)`.",
        "///",
        "/// Recomputed from the spec on every run, so a pair separates by itself",
        "/// the day whatspec records something that tells them apart.",
        f"pub const MERGED_OUTGOING: [(&str, &str); {len(merged)}] = [",
        *[f"    ({rust_str(a)}, {rust_str(b)})," for a, b in sorted(merged)],
        "];",
        "",
        "/// Shapes no stanza can ever derive as, and which one claims them instead.",
        "///",
        "/// These differ from an earlier shape in nothing a reader can see: an",
        "/// attribute the spec calls `dynamic` in one and `string` in the other is",
        "/// the same attribute on the wire — the difference is whether the builder",
        "/// was handed the value or computed it — and an extra optional attribute",
        "/// discriminates nothing.",
        "///",
        "/// Listed rather than dropped, and rather than reordered around: reordering",
        "/// would only move the problem to the other shape.",
        f"pub const UNREACHABLE_OUTGOING: [(&str, &str); {len(shadowed)}] = [",
        *[f"    ({rust_str(k)}, {rust_str(v)})," for k, v in sorted(shadowed.items())],
        "];",
        "",
    ]

    # Generated tests for generated code (D-042). 210 shapes is well past what
    # anyone would keep fixtures for by hand, and a hand-written fixture against
    # a generated shape drifts the moment the spec moves.
    #
    # Table-driven rather than one test per shape, and the reason is coverage
    # rather than taste: an assertion's failure path is a region that a passing
    # test never enters, so 210 tests with a formatted message each contribute
    # some 550 regions that can only be reached by breaking the build. One loop
    # over a table has one such region and says as much when it fails.
    lines += [
        "#[cfg(test)]",
        "mod outgoing_tests {",
        "    use super::*;",
        "    use crate::testing::Fixture;",
        "    extern crate alloc;",
        "    use alloc::vec::Vec;",
        "",
        "    /// A shape, a stanza built the way its builder builds one, and the",
        "    /// same with every optional attribute written.",
        "    struct Case {",
        "        shape: &'static str,",
        "        lean: fn() -> Vec<u8>,",
        "        full: fn() -> Vec<u8>,",
        "        /// A stanza contradicting one of the shape's pinned values, for",
        "        /// the shapes that pin one.",
        "        wrong_pin: Option<fn() -> Vec<u8>>,",
        "    }",
        "",
    ]
    for name, shape in named.items():
        if name in shadowed:
            continue
        lines += [
            f"    fn lean_{snake(name)}() -> Vec<u8> {{",
            f"        {fixture_for(shape)}.build().bytes().to_vec()",
            "    }",
            f"    fn full_{snake(name)}() -> Vec<u8> {{",
            f"        {fixture_for(shape, True)}.build().bytes().to_vec()",
            "    }",
        ]
        if consts_of(shape["attrs"]):
            lines += [
                f"    fn wrong_pin_{snake(name)}() -> Vec<u8> {{",
                f"        {fixture_for(shape, False, break_pin=True)}.build().bytes().to_vec()",
                "    }",
            ]
        lines.append("")

    reachable = [(n, sh) for n, sh in named.items() if n not in shadowed]
    lines += [f"    const CASES: [Case; {len(reachable)}] = ["]
    for name, shape in reachable:
        pin = (
            f"Some(wrong_pin_{snake(name)})"
            if consts_of(shape["attrs"])
            else "None"
        )
        lines += [
            "        Case {",
            f"            shape: {rust_str(name)},",
            f"            lean: lean_{snake(name)},",
            f"            full: full_{snake(name)},",
            f"            wrong_pin: {pin},",
            "        },",
        ]
    lines += ["    ];", ""]

    lines += [
        "    /// Every shape derives from a stanza built the way its builder builds",
        "    /// one, and derives as *itself*.",
        "    ///",
        "    /// Landing on a different shape is not a wrinkle to paper over: it",
        "    /// means two builders in the spec produce stanzas this dispatch cannot",
        "    /// tell apart, and the ones that genuinely cannot be told apart are",
        "    /// listed in `UNREACHABLE_OUTGOING` and excluded from this table.",
        "    #[test]",
        "    fn every_shape_derives_as_itself() {",
        "        for case in &CASES {",
        "            let stanza = (case.lean)();",
        "            let node = parse_bytes(&stanza);",
        "            let Ok(derived) = derive_outgoing(&node) else {",
        "                panic!(\"{} does not derive\", case.shape);",
        "            };",
        "            assert_eq!(derived.shape(), case.shape);",
        "",
        "            // Derivation is pure, and each shape carries its own",
        "            // comparison: one that could not recognise its own output",
        "            // would report every stanza as a divergence.",
        "            let again = derive_outgoing(&node).expect(\"derives twice\");",
        "            assert!(derived.semantic_eq(&again));",
        "        }",
        "    }",
        "",
        "    /// Every shape derives with its optional attributes present.",
        "    ///",
        "    /// Which shape claims it is deliberately not asserted: the optional",
        "    /// attributes make the stanza richer, and a richer shape is tried",
        "    /// first by design. What this reaches is the `Some` side of every",
        "    /// optional field and the comparison arm that reads it.",
        "    #[test]",
        "    fn every_shape_derives_with_its_optional_attributes() {",
        "        for case in &CASES {",
        "            let stanza = (case.full)();",
        "            let node = parse_bytes(&stanza);",
        "            let Ok(full) = derive_outgoing(&node) else {",
        "                panic!(\"{} does not derive with every attribute\", case.shape);",
        "            };",
        "            assert!(full.semantic_eq(&full));",
        "        }",
        "    }",
        "",
        "    /// A pinned value is enforced, not merely used to pick a shape.",
        "    ///",
        "    /// A `const` on a child is invisible to dispatch, which can only guard",
        "    /// on the stanza's own attributes — two `abt get` requests differ only",
        "    /// in what their `<props>` child pins. So every shape checks its own",
        "    /// pins, and a stanza contradicting one is not that shape.",
        "    #[test]",
        "    fn a_contradicted_pin_is_not_that_shape() {",
        "        let mut checked = 0;",
        "        for case in &CASES {",
        "            let Some(build) = case.wrong_pin else { continue };",
        "            checked += 1;",
        "            let stanza = build();",
        "            let node = parse_bytes(&stanza);",
        "            if let Ok(derived) = derive_outgoing(&node) {",
        "                assert_ne!(",
        "                    derived.shape(),",
        "                    case.shape,",
        "                    \"a contradicted pin still derived as its own shape\"",
        "                );",
        "            }",
        "        }",
        "        assert!(checked > 50, \"only {checked} shapes pin a value\");",
        "    }",
        "",
        "    fn parse_bytes(bytes: &[u8]) -> wa_wire_codec::NodeRef<'_> {",
        "        wa_wire_codec::Parser::new(crate::testing::FIXTURE_TABLE)",
        "            .parse(bytes)",
        "            .expect(\"a generated fixture parses\")",
        "    }",
        "}",
        "",
    ]

    TARGET.write_text("\n".join(lines), encoding="utf-8")
    subprocess.run(["rustfmt", "--edition", "2024", str(TARGET)], check=True)
    for drop in sorted(set(drops)):
        print(f"  unmodelled: {drop}")
    for folded, survivor in sorted(merged):
        print(f"  merged: {folded} — one stanza with {survivor}")
    for name, claimant in sorted(shadowed.items()):
        print(f"  unreachable: {name} — {claimant} claims its stanzas")
    print(
        f"{TARGET.relative_to(ROOT)}: {len(named)} shapes "
        f"({len(shapes) - len(list(iq_shapes(json.loads(iq_bytes))))} stanza, "
        f"{len(iq_shapes(json.loads(iq_bytes)))} iq), {len(tags)} tags, "
        f"{len(set(drops))} unmodelled, {len(merged)} merged, "
        f"{len(shadowed)} unreachable"
    )


if __name__ == "__main__":
    main()
