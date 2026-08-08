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
    "contentString": ("Value<'a>", "extract::content_string(node)?"),
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

# Three buckets, because they are three different facts about the derivation.
# A shrinking `drops` is progress; `request_scoped` never shrinks and is not
# debt; `untyped` is a field that crosses at less than its declared type.
drops: list[str] = []
untyped: list[str] = []
request_scoped: list[str] = []

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
    if enum_without_variants(out):
        # Decided here rather than at emit time because the fixture builder
        # walks the same spec separately: a decision made in one pass and not
        # the other produces a struct requiring a field the fixture omits.
        out["untypedEnum"] = out["method"]
        out["method"] = "attrString" if out.get("required") else "maybeAttrString"
    if not out.get("required"):
        out["method"] = OPTIONAL_FORM.get(out["method"], out["method"])
    return out


def enum_without_variants(field: dict) -> bool:
    """Whether the spec calls a field an enum and lists nothing it can be.

    Happens where the values live on sibling shapes as literal guards rather
    than on the field. Reconstructing the set from those would be inference,
    so the field is read as text instead, which is what the spec supports.
    """
    if (field.get("method") or "") not in ENUM_METHODS:
        return False
    keys = field.get("enumKeys")
    if keys is None:
        keys = [v.get("value") for v in (field.get("enumRef") or {}).get("variants", [])]
    return not [k for k in keys or [] if isinstance(k, str)]


def ordered_variants(spec_variants: list[dict]) -> list[dict]:
    """Richest first, and not as a preference.

    `NewsletterMessageAck`'s required fields are a subset of
    `NewsletterQuestionResponseAck`'s, so the leaner one accepts every stanza
    the richer one does — trying it first would claim them all and the richer
    variant would never derive. This is D-041 one level down: the rule that
    orders shapes of a tag orders alternatives of a mixin for the same reason.
    """
    return sorted(
        spec_variants,
        key=lambda v: (
            -len(guards_of(v)),
            -sum(1 for f in v["fields"] if f.get("required")),
            -len(v["fields"]),
        ),
    )


def guards_of(variant: dict) -> list[tuple[str, str]]:
    """The `attr` assertions that tell this variant from its siblings.

    `tag` assertions are the parent's and hold for every variant; `reference`
    ones need the request and are reported elsewhere (D-100).
    """
    return [
        (a["name"], a["value"])
        for a in variant.get("assertions", [])
        if a.get("kind") == "attr" and "value" in a
    ]


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
            # The same name read two ways: `verified_name` arrives as a child
            # element in one shape and as an attribute in another, and both are
            # real. Dropping either would lose a field that is on the wire, and
            # choosing between them by which came first is arbitrary, so both
            # are emitted and the later one carries its category.
            alias = f"{name}_{category(field['method'])}"
            if alias not in chosen:
                aliased = dict(field)
                aliased["name"] = alias
                aliased["wireName"] = field.get("wireName") or name
                aliased["tag"] = field.get("tag") or name
                chosen[alias] = aliased
                order.append(alias)
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


def semantic_compare(name: str, ty: str) -> str:
    """How one scalar field is compared."""
    if ty == "Value<'a>":
        return f"self.{name}.semantic_eq(other.{name})"
    if ty == "Option<Value<'a>>":
        return (
            f"match (self.{name}, other.{name}) {{ "
            f"(Some(a), Some(b)) => a.semantic_eq(b), (None, None) => true, _ => false }}"
        )
    if ty == "Jid<'a>":
        return f"self.{name}.semantic_eq(other.{name})"
    if ty == "Option<Jid<'a>>":
        return (
            f"match (self.{name}, other.{name}) {{ "
            f"(Some(a), Some(b)) => a.semantic_eq(b), (None, None) => true, _ => false }}"
        )
    return f"self.{name} == other.{name}"


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
        # Only list items are compared through the trait, so only they need the
        # impl. Emitting it for every struct would leave impls with no caller.
        self.list_items: set[str] = set()
        # Mixin groups already emitted. The same group appears under several
        # shapes and describes the same alternatives each time, so it becomes
        # one type rather than one per site.
        self.unions: set[str] = set()
        # Each group's spec variants, keyed by the emitted enum name, so the
        # test generator can build one fixture per alternative. The shape
        # fixtures only ever exercise the leanest, which would leave every
        # richer variant generated and unrun.
        self.union_variants: dict[str, list[dict]] = {}
        self.pending_trait_impls: list[str] = []

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
        comparisons: list[str] = []

        for field in dedupe(self.flatten(fields, name), name):
            emitted = self.emit_field(name, field, decls, inits, accessors, comparisons)
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

        # Two engines can encode one value differently and both be right, so a
        # conformance run compares meaning rather than bytes. `node` is excluded
        # on purpose: it is the frame this came from, and comparing frames is
        # what `semantic_eq` exists to avoid.
        struct += [
            "",
            f"impl {name}<'_> {{",
            "    /// Whether two derivations mean the same thing, whatever form",
            "    /// each field arrived in.",
            "    ///",
            "    /// The originating node is excluded: two engines may encode one",
            "    /// stanza differently and both be right.",
            "    #[must_use]",
            # A shape whose fields the generator could not express has nothing
            # to compare; naming the parameter would only warn.
            f"    pub fn semantic_eq(&self, {'other' if comparisons else '_other'}: &{name}<'_>) -> bool {{",
        ]
        if len(comparisons) == 1:
            struct.append(f"        {comparisons[0]}")
        elif comparisons:
            # Parenthesised: a bare `match` in leading position parses as a
            # statement, and the `&&` after it becomes a reference.
            joined = "\n            && ".join(f"({c})" for c in comparisons)
            struct.append(f"        {joined}")
        else:
            struct.append("        true")
        struct += ["    }", "}"]
        self.structs.append("\n".join(struct))
        self.pending_trait_impls.append(name)

    def emit_union(
        self, owner: str, field: dict, decls: list[str], inits: list[str],
        comparisons: list[str],
    ) -> bool:
        """A mixin group whose variants are alternatives on the same node.

        Named after its variants rather than after the spec's field name, which
        spells the alternation out and repeats itself
        (`ackPaidAckPaidConversationOrAckPaidGroupConversationConversationMixinGroup`).
        Naming it from the variants makes the same mixin appearing under two
        shapes generate one type, which is what a mixin is.
        """
        spec_variants = field["unionVariants"]
        enum_name = "Or".join(pascal(v["name"]) for v in spec_variants)
        name = snake(field["name"])

        ordered = ordered_variants(spec_variants)

        if enum_name not in self.unions:
            self.unions.add(enum_name)
            self.union_variants[enum_name] = spec_variants
            for variant in spec_variants:
                self.emit_struct(f"{enum_name}{pascal(variant['name'])}", variant["fields"])
            self.emit_union_enum(enum_name, spec_variants, ordered)

        required = bool(field.get("required"))
        ty = f"{enum_name}<'a>" if required else f"Option<{enum_name}<'a>>"
        call = (
            f"{enum_name}::derive(node)?"
            if required
            else f"{enum_name}::maybe_derive(node)"
        )
        decls += [
            f"    /// `{field['name']}`, one of "
            + ", ".join(f"`{v['name']}`" for v in spec_variants)
            + ".",
            f"    pub {name}: {ty},",
        ]
        inits.append(f"{name}: {call},")
        if required:
            comparisons.append(f"self.{name}.semantic_eq(&other.{name})")
        else:
            comparisons.append(
                f"match (&self.{name}, &other.{name}) {{ "
                f"(Some(a), Some(b)) => a.semantic_eq(b), "
                f"(None, None) => true, _ => false }}"
            )
        return True

    def emit_union_enum(
        self, enum_name: str, spec_variants: list[dict], ordered: list[dict]
    ) -> None:
        """The enum over a mixin group's alternatives, and how one is chosen."""
        lines = [
            f"/// One of the alternatives in whatspec's `{enum_name}` mixin group.",
            "///",
            "/// Variants are tried richest-first: where one variant's required",
            "/// fields are a subset of another's, the leaner one accepts every",
            "/// stanza the richer one does, and trying it first would claim them",
            "/// all (D-041).",
            "#[derive(Debug, Clone, PartialEq)]",
            "#[non_exhaustive]",
            f"pub enum {enum_name}<'a> {{",
        ]
        for variant in spec_variants:
            case = pascal(variant["name"])
            lines += [
                f"    /// `{variant['name']}`.",
                f"    {case}({enum_name}{case}<'a>),",
            ]
        lines += ["}", "", f"impl<'a> {enum_name}<'a> {{"]

        # `derive` — the required form, which must land on a variant.
        lines += [
            "    /// Derive whichever alternative this node satisfies.",
            "    ///",
            "    /// # Errors",
            "    ///",
            "    /// [`DeriveError::UnknownStanza`] when the node satisfies none of",
            "    /// them, which is the honest answer: the mixin says the stanza is",
            "    /// one of these and it is not.",
            "    pub fn derive(node: &NodeRef<'a>) -> Result<Self, DeriveError> {",
            "        Self::maybe_derive(node).ok_or(DeriveError::UnknownStanza)",
            "    }",
            "",
            "    /// Derive whichever alternative this node satisfies, or nothing.",
            "    #[must_use]",
            "    pub fn maybe_derive(node: &NodeRef<'a>) -> Option<Self> {",
        ]
        for variant in ordered:
            case = pascal(variant["name"])
            guards = guards_of(variant)
            condition = " && ".join(
                f"node.attr_eq({rust_str(key)}, {rust_str(value)})"
                for key, value in guards
            )
            indent = "        "
            if condition:
                lines.append(f"{indent}// guarded by " + ", ".join(f"{k}={v}" for k, v in guards))
                lines.append(f"{indent}if {condition}")
                lines.append(f"{indent}    && let Ok(inner) = {enum_name}{case}::derive(node)")
                lines.append(f"{indent}{{")
            else:
                lines.append(f"{indent}if let Ok(inner) = {enum_name}{case}::derive(node) {{")
            lines.append(f"{indent}    return Some(Self::{case}(inner));")
            lines.append(f"{indent}}}")
        lines += ["        None", "    }", "}", "",
                  f"impl {enum_name}<'_> {{",
                  "    /// Whether two alternatives mean the same thing.",
                  "    #[must_use]",
                  f"    pub fn semantic_eq(&self, other: &{enum_name}<'_>) -> bool {{",
                  # Named rather than `Self`, which would bind `other`'s
                  # lifetime to this one and reject a comparison between two
                  # derivations that borrow different frames — which is the
                  # only comparison there is.
                  "        match (self, other) {"]
        for variant in spec_variants:
            case = pascal(variant["name"])
            lines.append(f"            ({enum_name}::{case}(a), {enum_name}::{case}(b)) => a.semantic_eq(b),")
        # Two different alternatives never mean the same thing; the arm is
        # unreachable only when a mixin has one variant, which none does.
        lines += ["            _ => false,", "        }", "    }", "}"]
        self.structs.append("\n".join(lines))

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
        accessors: list[str], comparisons: list[str],
    ) -> bool:
        method = field.get("method") or ""
        name = snake(field["name"])
        # The spec records the field's name in the bundle's own casing and the
        # attribute's name on the wire separately, and they differ for fifty of
        # them. Reading by the former finds nothing and fails no test, because
        # a fixture built from the same source is wrong in the same way.
        key = rust_str(field.get("wireName") or field["name"])
        doc = f"    /// `{field['name']}`, via `{method or 'mixin'}`."

        if method in SCALAR_METHODS:
            if field.get("untypedEnum"):
                untyped.append(
                    f"{owner}.{field['name']}: {field['untypedEnum']} without variants"
                )
                doc = (
                    f"    /// `{field['name']}`, via `{field['untypedEnum']}`. The spec\n"
                    "    /// records no variants for it, so it crosses as text."
                )
            ty, call = SCALAR_METHODS[method]
            if not field.get("required") and method in OPTIONAL_JID:
                ty, call = f"Option<{ty}>", OPTIONAL_JID[method]
            decls += [doc, f"    pub {name}: {ty},"]
            inits.append(f"{name}: {call.format(key=key)},")
            comparisons.append(semantic_compare(name, ty))
            return True

        if method in ENUM_METHODS:
            enum_name = self.enum_for(owner, field)
            if enum_name is None:
                # `normalise` rewrites these before they arrive, so reaching
                # here means an enum with variants that still would not emit.
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
            comparisons.append(f"self.{name} == other.{name}")
            return True

        if method in CHILD_METHODS:
            child_name = f"{owner}{pascal(field['name'])}"
            self.emit_struct(child_name, field.get("children", []))
            tag = rust_str(field.get("tag") or field["name"])
            if method == "child":
                comparisons.append(f"self.{name}.semantic_eq(&other.{name})")
                decls += [doc, f"    pub {name}: alloc::boxed::Box<{child_name}<'a>>,"]
                inits.append(
                    f"{name}: alloc::boxed::Box::new({child_name}::derive("
                    f"&extract::child(node, {tag})?)?),"
                )
            else:
                comparisons.append(
                    f"match (&self.{name}, &other.{name}) {{ "
                    f"(Some(a), Some(b)) => a.semantic_eq(b), "
                    f"(None, None) => true, _ => false }}"
                )
                decls += [doc, f"    pub {name}: Option<alloc::boxed::Box<{child_name}<'a>>>,"]
                inits.append(
                    f"{name}: match extract::maybe_child(node, {tag}) {{ "
                    f"Some(child) => Some(alloc::boxed::Box::new("
                    f"{child_name}::derive(&child)?)), None => None }},"
                )
            return True

        if field.get("type") == "union" and field.get("unionVariants"):
            return self.emit_union(owner, field, decls, inits, comparisons)

        if method in LIST_METHODS:
            item_name = f"{owner}{pascal(field['name'])}"
            self.emit_struct(item_name, field.get("children", []))
            tag = rust_str(field.get("tag") or field["name"])
            comparisons.append(
                f"crate::semantic::iter_eq(self.{name}(), other.{name}())"
            )
            self.list_items.add(item_name)
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
    # The wire name, for the same reason the reader uses it: a fixture built
    # from the other name would agree with a reader that also used it, and the
    # pair would pass while neither matched a real stanza.
    name = rust_str(field.get("wireName") or field["name"])
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
    if method == "contentString":
        # `.bytes` writes a scalar body, which is what a text body is on the
        # wire — the binary node encoding has no separate string-body form.
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
    if field.get("type") == "union" and field.get("unionVariants"):
        return union_fixture(field, full)
    return None


def variant_fixture(variant: dict, full: bool = False) -> str:
    """A stanza satisfying one named alternative of a mixin group.

    Always on tag `ack`: every mixin group here asserts it, and a variant's
    `tag` assertion is the parent's rather than its own.
    """
    parts = ["Fixture::node(\"ack\")"]
    seen: set[str] = set()
    for key, value in guards_of(variant):
        seen.add(key)
        parts.append(f".attr({rust_str(key)}, {rust_str(value)})")
    for field in dedupe_quiet(variant["fields"]):
        if not full and not field.get("required"):
            continue
        key = field.get("wireName") or field.get("tag") or field["name"]
        if key in seen:
            continue
        value = fixture_value(field, full)
        if value:
            seen.add(key)
            parts.append(value)
    return "".join(parts)


def union_fixture(field: dict, full: bool) -> str | None:
    """Attributes that make one alternative of a mixin group derive.

    Inline rather than in a child: a mixin group parses the same node as the
    shape holding it, so its fields are that node's.

    The *leanest* variant, because a fixture only has to make the union derive
    and the leanest is the one whose requirements can always be met. Dispatch
    still tries the richer ones first and finds them unsatisfied, which
    exercises the ordering rather than bypassing it.
    """
    leanest = min(
        field["unionVariants"],
        key=lambda v: (
            len(guards_of(v)),
            sum(1 for f in v["fields"] if f.get("required")),
            len(v["fields"]),
        ),
    )
    parts: list[str] = []
    seen: set[str] = set()
    # The guards first: a variant with `edit="1"` needs that exact value, and a
    # generic `"x"` from the field below would contradict it.
    for key, value in guards_of(leanest):
        seen.add(key)
        parts.append(f".attr({rust_str(key)}, {rust_str(value)})")
    for inner in dedupe_quiet(leanest["fields"]):
        if not full and not inner.get("required"):
            continue
        key = inner.get("wireName") or inner.get("tag") or inner["name"]
        if key in seen:
            continue
        value = fixture_value(inner, full)
        if value:
            seen.add(key)
            parts.append(value)
    return "".join(parts) or None


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
            elif kind == "reference":
                # Not a gap. A reference assertion says this response's `from`
                # must equal the request's `to`, which needs the request that
                # this stanza answers. `derive` is a pure function of one
                # stanza (D-010), so this is outside what L1 can check at all.
                request_scoped.append(
                    f"{parser}: `{assertion.get('name')}` must match the request's "
                    f"`{'.'.join(assertion.get('referencePath') or [])}`"
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
        f"    proto_digest: {rust_str(provenance['protoDigest'])},",
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

    # The trait exists for `iter_eq`, so it is implemented for the types that
    # reach it and no others.
    for name in emitter.pending_trait_impls:
        if name not in emitter.list_items:
            continue
        lines += [
            f"impl crate::semantic::SemanticEq for {name}<'_> {{",
            "    fn semantic_eq(&self, other: &Self) -> bool {",
            f"        {name}::semantic_eq(self, other)",
            "    }",
            "}",
            "",
        ]

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
    lines += ["        }", "    }", ""]

    lines += [
        "    /// Whether two events mean the same thing.",
        "    ///",
        "    /// Different shapes never do, even for one tag: which shape matched",
        "    /// is part of what was derived, so two engines picking different",
        "    /// shapes for one stanza is exactly the divergence worth reporting.",
        "    #[must_use]",
        "    pub fn semantic_eq(&self, other: &Event<'_>) -> bool {",
        "        match (self, other) {",
    ]
    for variant, _struct, _tag in variants:
        lines.append(
            f"            (Self::{variant}(a), Event::{variant}(b)) => a.semantic_eq(b),"
        )
    lines += ["            _ => false,", "        }", "    }", "}", ""]

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
    lines += [f"    {rust_str(d)}," for d in sorted(set(drops))]
    lines += ["];", ""]

    lines += [
        "/// Fields the spec types more precisely than this derivation carries.",
        "///",
        "/// An `attrEnum` whose variants the spec never lists is the whole of",
        "/// this today: the values live on sibling shapes as literal guards, and",
        "/// reconstructing the set from those would be inference. The field",
        "/// crosses as text, which is what the spec supports.",
        "pub const UNTYPED_FIELDS: [&str; " + str(len(set(untyped))) + "] = [",
    ]
    lines += [f"    {rust_str(u)}," for u in sorted(set(untyped))]
    lines += ["];", ""]

    lines += [
        "/// Checks the spec states that L1 cannot make, by construction.",
        "///",
        "/// A reference assertion says a response's field must equal one from",
        "/// the request it answers. [`derive()`] is a pure function of a single",
        "/// stanza (D-010), and the request is not in it, so these are outside",
        "/// what this layer can evaluate rather than something it has not got to",
        "/// yet. A host that tracks outstanding requests can check them; this",
        "/// names them so that host knows what to check.",
        "///",
        "/// Unlike [`UNMODELLED_FIELDS`], a shrinking list here would mean the",
        "/// spec changed, not that the generator improved.",
        "pub const REQUEST_SCOPED_ASSERTIONS: [&str; "
        + str(len(set(request_scoped)))
        + "] = [",
    ]
    lines += [f"    {rust_str(r)}," for r in sorted(set(request_scoped))]
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
            f"    /// `{name}` agrees with itself and differs from another shape.",
            "    ///",
            "    /// Reflexivity is the floor: a comparison that cannot recognise",
            "    /// its own output would report every stanza as a divergence.",
            "    #[test]",
            f"    fn {snake(name)}_compares_semantically() {{",
            f"        let stanza = {fixture_for(entry['tag'], entry['shape']['fields'], True)}.build();",
            "        let node = parse(&stanza);",
            f"        let derived = {name}::derive(&node).expect(\"derives\");",
            f"        let again = {name}::derive(&node).expect(\"derives\");",
            "        assert!(derived.semantic_eq(&again));",
            "",

            "        // A stanza missing every optional field is a different",
            "        // derivation of the same shape, unless the shape has none.",
            f"        let bare = {fixture_for(entry['tag'], entry['shape']['fields'], False)}.build();",
            "        let bare_node = parse(&bare);",
            f"        if let Ok(bare_derived) = {name}::derive(&bare_node) {{",
            "            let full_is_bare = derived.semantic_eq(&bare_derived);",
            "            // Either they carry the same fields or they do not; both",
            "            // are valid, and the comparison must not panic either way.",
            "            let _ = full_is_bare;",
            "        }",
            "    }",
            "",
        ]
    # One test per union variant. The shape fixtures reach only the leanest
    # alternative, by construction: a fixture that satisfied a richer one would
    # satisfy the leaner one too and say nothing about which was chosen. So each
    # alternative is built from its own fields here, which is also the only
    # thing that exercises the richest-first ordering.
    for enum_name, spec_variants in sorted(emitter.union_variants.items()):
        for variant in spec_variants:
            case = pascal(variant["name"])
            builder = variant_fixture(variant)
            lines += [
                f"    /// `{enum_name}::{case}` is chosen for a stanza built from",
                f"    /// `{variant['name']}`'s own fields.",
                "    #[test]",
                f"    fn {snake(enum_name)}_selects_{snake(case)}() {{",
                f"        let stanza = {builder}.build();",
                "        let node = parse(&stanza);",
                f"        let derived = {enum_name}::derive(&node);",
                f'        assert!(derived.is_ok(), "{enum_name}::{case}: {{:?}}", derived.err());',
                f"        let chosen = derived.expect(\"derives\");",
                f"        assert!(matches!(chosen, {enum_name}::{case}(_)));",
                "",
                "        // Each alternative carries its own comparison, and one that",
                "        // could not recognise its own output would report every",
                "        // stanza as a divergence. Derivation is pure, so the second",
                "        // run is the same derivation and must compare equal.",
                f"        let again = {enum_name}::derive(&node).expect(\"derives\");",
                "        assert!(chosen.semantic_eq(&again));",
                "    }",
                "",
                f"    /// `{enum_name}::{case}` derives with every field it models.",
                "    ///",
                "    /// The required-only fixture never reaches the `Some` side of an",
                "    /// optional field, nor the comparison arm that reads it.",
                "    #[test]",
                f"    fn {snake(enum_name)}_selects_{snake(case)}_with_every_field() {{",
                f"        let stanza = {variant_fixture(variant, True)}.build();",
                "        let node = parse(&stanza);",
                f"        let Some(full) = {enum_name}::maybe_derive(&node) else {{",
                f'            panic!("{enum_name}::{case} derives with every field");',
                "        };",
                "        assert!(full.semantic_eq(&full));",
                "",
                "        // A derivation carrying optional fields does not mean the",
                "        // same as one without them.",
                f"        let bare = {variant_fixture(variant)}.build();",
                "        let bare_node = parse(&bare);",
                f"        if let Some(lean) = {enum_name}::maybe_derive(&bare_node) {{",
                "            let _ = full.semantic_eq(&lean);",
                "        }",
                "    }",
                "",
            ]
        # What an unrelated node does depends on whether any alternative is
        # unconditionally satisfiable. One with no guards and no required
        # fields matches everything — a catch-all — and a group holding one
        # never reports "none of these". Both are correct; which one this group
        # is, is worth a test rather than an assumption.
        catch_all = next(
            (
                v
                for v in ordered_variants(spec_variants)
                if not guards_of(v)
                and not any(f.get("required") for f in v["fields"])
            ),
            None,
        )
        if catch_all is None:
            lines += [
                f"    /// A node satisfying no `{enum_name}` alternative yields none.",
                "    #[test]",
                f"    fn {snake(enum_name)}_matches_nothing_when_no_variant_fits() {{",
                '        let stanza = Fixture::node("nothing-here").build();',
                "        let node = parse(&stanza);",
                f"        assert!({enum_name}::maybe_derive(&node).is_none());",
                f"        assert_eq!({enum_name}::derive(&node), Err(DeriveError::UnknownStanza));",
                "    }",
                "",
            ]
        else:
            case = pascal(catch_all["name"])
            lines += [
                f"    /// `{enum_name}` always derives: `{catch_all['name']}`",
                "    /// requires nothing, so it accepts any node the richer",
                "    /// alternatives turned down.",
                "    ///",
                "    /// Not a defect — the spec declares a variant with no fields of",
                "    /// its own — but it means this group can never report that a",
                "    /// stanza matched none of its alternatives.",
                "    #[test]",
                f"    fn {snake(enum_name)}_falls_back_to_{snake(case)}() {{",
                '        let stanza = Fixture::node("nothing-here").build();',
                "        let node = parse(&stanza);",
                "        assert!(matches!(",
                f"            {enum_name}::maybe_derive(&node),",
                f"            Some({enum_name}::{case}(_))",
                "        ));",
                "    }",
                "",
            ]
        if len(spec_variants) > 1:
            first, second = spec_variants[0], spec_variants[1]
            lines += [
                f"    /// Two different `{enum_name}` alternatives never mean the same.",
                "    #[test]",
                f"    fn {snake(enum_name)}_alternatives_are_not_interchangeable() {{",
                f"        let a = {variant_fixture(first)}.build();",
                f"        let b = {variant_fixture(second)}.build();",
                "        let (na, nb) = (parse(&a), parse(&b));",
                f"        let (Some(x), Some(y)) = ({enum_name}::maybe_derive(&na), {enum_name}::maybe_derive(&nb))",
                "        else {",
                "            panic!(\"both fixtures derive\");",
                "        };",
                "        assert!(x.semantic_eq(&x));",
                "        // Same alternative or not, comparing must not panic; where",
                "        // they differ, they must not compare equal.",
                "        if core::mem::discriminant(&x) != core::mem::discriminant(&y) {",
                "            assert!(!x.semantic_eq(&y));",
                "        }",
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
        "    /// Events of different shapes never mean the same thing.",
        "    #[test]",
        "    fn different_shapes_never_compare_equal() {",
        "        // Which shape matched is part of what was derived, so two",
        "        // engines picking different shapes for one stanza is exactly",
        "        // the divergence a conformance run should report.",
        "        let stanza = Fixture::node(\"receipt\")",
        "            .attr(\"id\", \"A\")",
        "            .jid_attr(\"from\", \"u\")",
        "            .build();",
        "        let node = parse(&stanza);",
        "        let a = derive(&node).expect(\"derives\");",
        "        assert!(a.semantic_eq(&a), \"an event agrees with itself\");",
        "",
        "        let other = Fixture::node(\"ack\")",
        "            .attr(\"id\", \"A\")",
        "            .attr(\"class\", \"message\")",
        "            .jid_attr(\"content\", \"u\")",
        "            .build();",
        "        let b = derive(&parse(&other)).expect(\"derives\");",
        "        assert!(!a.semantic_eq(&b));",
        "        assert!(!b.semantic_eq(&a));",
        "        assert_ne!(a.tag(), b.tag());",
        "    }",
        "",
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
          f"{len(emitter.enums)} enums, {len(by_tag)} tags, "
          f"{len(set(drops))} unmodelled, {len(set(untyped))} untyped, "
          f"{len(set(request_scoped))} request-scoped")
    if drops:
        for drop in sorted(set(drops)):
            print(f"  unmodelled: {drop}", file=sys.stderr)


if __name__ == "__main__":
    main()
