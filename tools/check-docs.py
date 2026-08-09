#!/usr/bin/env python3
"""Check the design document against the code it describes.

Three times in one day the same defect appeared in a different file: a count of
capabilities that the code had moved past. Once it reached crates.io, where a
published README cannot be edited. Every instance was prose that was true when
written, and every one was found by a person re-reading rather than by a test.

So the checks here are the subset of that prose a machine can settle. Nothing
here reads for meaning; it compares a claim against the thing claimed.

    python3 tools/check-docs.py
"""

from __future__ import annotations

import re
import subprocess
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DESIGN = ROOT / "DESIGN.md"
CAPABILITY_RS = ROOT / "crates" / "wa-wire-contract" / "src" / "capability.rs"
CONTRACT_TOML = ROOT / "crates" / "wa-wire-contract" / "Cargo.toml"

NUMBER_WORDS = {
    "two": 2, "three": 3, "four": 4, "five": 5, "six": 6,
    "seven": 7, "eight": 8, "nine": 9, "ten": 10, "eleven": 11, "twelve": 12,
}


class Failures:
    """Collected findings, each with the line that produced it."""

    def __init__(self) -> None:
        self.items: list[tuple[str, int | None, str]] = []

    def add(self, check: str, line: int | None, message: str) -> None:
        # Two phrasings can match the same sentence; report it once.
        if (check, line, message) not in self.items:
            self.items.append((check, line, message))

    def __bool__(self) -> bool:
        return bool(self.items)


def line_of(text: str, index: int) -> int:
    return text.count("\n", 0, index) + 1


def real_capabilities() -> list[str]:
    """The capability identifiers, read from the source of truth."""
    source = CAPABILITY_RS.read_text()
    match = re.search(r"ALL[^=]*=\s*&?\[(.*?)\];", source, re.S)
    if not match:
        raise SystemExit("check-docs: could not find Capability::ALL")

    variants = re.findall(r"Self::(\w+)", match.group(1))
    names = dict(re.findall(r"Self::(\w+)\s*=>\s*\"([^\"]+)\"", source))
    missing = [variant for variant in variants if variant not in names]
    if missing:
        raise SystemExit(f"check-docs: no string name for {missing}")

    return [names[variant] for variant in variants]


def current_text(text: str) -> str:
    """Everything the document asserts is true now.

    The changelog is excluded: it records what was true at a revision, and has
    to be able to quote the wrong count that a later revision fixed. Anchors
    and citations are still checked there, since those must resolve whenever
    they were written.
    """
    at = text.find("\n## Changelog")
    return text if at == -1 else text[:at]


def check_capability_names(text: str, capabilities: list[str], out: Failures) -> None:
    """Every capability the document names must exist."""
    known = set(capabilities)
    for match in re.finditer(r"`(l0\.[a-z.-]+|lifecycle\.[a-z.-]+)`", text):
        name = match.group(1)
        if name not in known:
            out.add(
                "capability-names",
                line_of(text, match.start()),
                f"`{name}` is not a capability. Known: {', '.join(sorted(known))}",
            )


def check_capability_count(text: str, capabilities: list[str], out: Failures) -> None:
    """A stated number of capabilities must be the real one.

    This is the check that would have caught all three instances. The phrasings
    are the ones the document actually uses; a new phrasing escapes until it is
    added here, which is why the names check above runs unconditionally.
    """
    total = len(capabilities)
    patterns = [
        r"`?Capability::ALL`?\s+has\s+(\w+)\s+members",
        r"(?:the\s+)?(\w+)\s+capabilit(?:y|ies)\s+are\s+a\s+versioned\s+surface",
        r"(\w+)\s+capability\s+identifiers",
        r"has\s+(\w+)\s+members",
    ]
    for pattern in patterns:
        for match in re.finditer(pattern, text, re.I):
            word = match.group(1).lower()
            stated = NUMBER_WORDS.get(word)
            if stated is None and word.isdigit():
                stated = int(word)
            if stated is None:
                continue
            if stated != total:
                out.add(
                    "capability-count",
                    line_of(text, match.start()),
                    f"says {word} ({stated}), but there are {total}",
                )


def check_in_repo_refs(text: str, out: Failures) -> None:
    """A `path:line` reference into this repository must still resolve."""
    for match in re.finditer(r"`([A-Za-z0-9_][A-Za-z0-9_/.-]*\.(?:rs|ts|go|py|toml|md)):(\d+)(?:-(\d+))?`", text):
        path, start, end = match.group(1), int(match.group(2)), match.group(3)
        target = ROOT / path
        if not target.exists():
            # Not ours. Bare filenames belong to the engine repositories and are
            # handled by check_external_refs.
            continue

        total = len(target.read_text().splitlines())
        last = int(end) if end else start
        if last > total:
            out.add(
                "stale-ref",
                line_of(text, match.start()),
                f"`{path}:{match.group(2)}` is past the end of the file ({total} lines)",
            )


ENGINES = {name: ROOT.parent / name for name in ("whatsapp-rust", "zapo", "Baileys", "hypermeow")}

# How far a cited line may sit from the evidence for it. Wide, because a
# citation often points inside a function while naming the function.
DRIFT_WINDOW = 20

# Citations whose filename matches more than one engine file, so which one the
# document means cannot be settled from the name.
AMBIGUOUS: list[str] = []


# Tried in order, first one that has the file wins. Naming a single default
# per repository does not survive contact: Baileys' `origin/HEAD` points at a
# `master` from before the monorepo layout, where none of these paths exist.
BRANCHES = ("origin/develop", "origin/main", "origin/master", "develop", "main", "master")

# Which branch each citation was read at, for the run's summary.
BRANCH_USED: dict[str, str] = {}


def read_at_default_branch(path: Path) -> list[str] | None:
    """A file's lines on its repository's release branch, or None if absent.

    Read from the branch rather than the working tree so the check does not
    depend on what the reader happens to have checked out. Ours was on a PR
    branch, which made two correct citations look stale.
    """
    repo = path
    while repo != repo.parent and not (repo / ".git").exists():
        repo = repo.parent
    if not (repo / ".git").exists():
        return path.read_text(errors="replace").splitlines()

    relative = path.relative_to(repo)
    for branch in BRANCHES:
        shown = subprocess.run(
            ["git", "-C", str(repo), "show", f"{branch}:{relative}"],
            capture_output=True, text=True,
        )
        if shown.returncode == 0:
            BRANCH_USED[repo.name] = branch
            return shown.stdout.splitlines()

    return None


def resolve_in_engines(path: str) -> list[Path]:
    """Every engine file whose path ends with this reference.

    References are written the way a reader would cite them, which is often a
    bare filename. Resolving means searching; an ambiguous name is reported
    rather than guessed at.
    """
    suffix = "/" + path
    found: list[Path] = []
    for root in ENGINES.values():
        if not root.is_dir():
            continue
        for candidate in root.rglob(Path(path).name):
            if candidate.is_file() and str(candidate).endswith(suffix) and "node_modules" not in candidate.parts:
                found.append(candidate)

    return found


def check_external_refs(text: str, out: Failures) -> None:
    """A citation into an engine must still point at what it claims.

    These are the document's evidence, so the check is that they are true, not
    that they are absent. What rots is the line number: a reference into
    Baileys named `transport.decrypt(frame)` for eighteen revisions and now
    lands on a variable declaration two functions away.

    Read from the engine's default branch rather than the working tree, since
    that is what the document audits and the reader may have anything checked
    out. A ref marked *(PR #N)* in the matrix is about a branch and is not
    checked here.
    """
    AMBIGUOUS.clear()
    missing = sorted(name for name, path in ENGINES.items() if not path.is_dir())
    if missing:
        # A reference resolves by searching the checkouts, so a missing one is
        # indistinguishable from a deleted file. Skipping is the honest read;
        # CI has no engines beside it and would otherwise fail on all of them.
        print(f"check-docs: engines not checked out ({', '.join(missing)}), citations unchecked")
        return

    for offset, line_text in enumerate(text.splitlines(), start=1):
        if not re.search(r"`[A-Za-z0-9_][A-Za-z0-9_/.-]*\.(?:rs|ts|go):\d+", line_text):
            continue

        # A table row cites one engine per cell, so a reference is evidence for
        # what its own cell says and nothing in a neighbouring column.
        cells = line_text.split("|") if line_text.lstrip().startswith("|") else [line_text]
        line_number = offset

        for cell in cells:
            for ref, snippet in cited_pairs(cell):
                check_citation(ref, snippet, line_number, out)

    if BRANCH_USED:
        read = ", ".join(f"{repo} at {branch}" for repo, branch in sorted(BRANCH_USED.items()))
        print(f"check-docs: citations read from {read}")
    if AMBIGUOUS:
        names = ", ".join(sorted(set(AMBIGUOUS)))
        print(f"check-docs: {len(set(AMBIGUOUS))} citations name more than one engine file, unchecked: {names}")


def is_code_snippet(snippet: str) -> bool:
    """Whether a backticked string is evidence a citation can be checked against.

    Excludes the citations themselves, capability names, and bare numbers: none
    of those appear in the file being cited.
    """
    if re.fullmatch(r"[A-Za-z0-9_][A-Za-z0-9_/.-]*\.(?:rs|ts|go|py|toml|md):\d+(?:-\d+)?", snippet):
        return False
    if re.fullmatch(r":?\d+(?:-\d+)?", snippet):
        return False
    if re.fullmatch(r"(?:l0|lifecycle)\.[a-z.-]+", snippet):
        return False

    return True


REF = r"([A-Za-z0-9_][A-Za-z0-9_/.-]*\.(?:rs|ts|go)):(\d+)(?:-(\d+))?"


def cited_pairs(cell: str) -> list[tuple[tuple[str, str, str], str | None]]:
    """Each reference with the snippet it is a citation for.

    The document writes them as ``code`` followed by ``(file:line)``, so a
    reference belongs to the backticked run immediately before it and to
    nothing further away. Pairing every snippet on the line with every
    reference instead reads one engine's evidence against another's file.
    """
    pairs: list[tuple[tuple[str, str, str], str | None]] = []
    previous: str | None = None
    for match in re.finditer(r"`([^`]+)`", cell):
        inner = match.group(1)
        ref = re.fullmatch(REF, inner)
        if ref:
            pairs.append(((ref.group(1), ref.group(2), ref.group(3) or ""), previous))
            previous = None
        elif is_code_snippet(inner):
            previous = inner

    return pairs


def check_citation(ref: tuple[str, str, str], snippet: str | None, line_number: int, out: Failures) -> None:
    """Check one citation against the file it points into."""
    for path, start, end in [ref]:
        if (ROOT / path).exists():
            continue

        candidates = resolve_in_engines(path)
        if not candidates:
            out.add("external-ref", line_number, f"`{path}` matches no file in any engine checkout")
            continue
        if len(candidates) > 1:
            AMBIGUOUS.append(path)
            continue

        lines = read_at_default_branch(candidates[0])
        if lines is None:
            out.add("external-ref", line_number, f"`{path}` is gone from its repository's default branch")
            continue

        first, last = int(start), int(end or start)
        if last > len(lines):
            out.add(
                "external-ref",
                line_number,
                f"`{path}:{start}` is past the end of the file ({len(lines)} lines)",
            )
            continue

        if not snippet:
            continue

        window = "\n".join(lines[max(0, first - 1 - DRIFT_WINDOW) : last + DRIFT_WINDOW])
        # Identifiers rather than the snippet verbatim: prose cites a method as
        # `backing_bytes()` where the file declares `fn backing_bytes(&self)`.
        wanted = set(re.findall(r"[A-Za-z_][A-Za-z0-9_]{3,}", snippet))
        if wanted and not any(word in window for word in wanted):
            out.add(
                "external-ref",
                line_number,
                f"`{path}:{start}` no longer shows `{snippet}`",
            )


def check_anchors(text: str, out: Failures) -> None:
    """Every internal link must reach a heading that exists."""
    headings = set()
    for line in text.splitlines():
        if line.startswith("#"):
            slug = re.sub(r"[^\w\s-]", "", line.lstrip("#").strip().lower())
            headings.add(slug.replace(" ", "-"))

    for match in re.finditer(r"\]\(([^)]*DESIGN\.md)?#([^)]+)\)", text):
        anchor = match.group(2)
        if anchor not in headings:
            out.add("anchor", line_of(text, match.start()), f"#{anchor} has no heading")


def check_rfc_references(text: str, out: Failures) -> None:
    """Every RFC mentioned must be one the document defines."""
    defined = set(re.findall(r"^## (RFC-\d+)", text, re.M))
    for match in re.finditer(r"\b(RFC-\d+)\b", text):
        name = match.group(1)
        if name not in defined:
            out.add("rfc-ref", line_of(text, match.start()), f"{name} is referenced but never defined")


def check_published_version(text: str, out: Failures) -> None:
    """The document must state the version that is actually published."""
    version = tomllib.loads(CONTRACT_TOML.read_text())["package"]["version"]
    if f"crates.io/crates/wa-wire-contract" not in text:
        out.add("published", None, "no link to the published crate")
    if version not in text:
        out.add("published", None, f"wa-wire-contract is at {version}, which the document never mentions")


def main() -> int:
    text = DESIGN.read_text()
    capabilities = real_capabilities()
    out = Failures()

    asserted = current_text(text)
    check_capability_names(asserted, capabilities, out)
    check_capability_count(asserted, capabilities, out)
    check_in_repo_refs(text, out)
    check_external_refs(text, out)
    check_anchors(text, out)
    check_rfc_references(text, out)
    check_published_version(text, out)

    if not out:
        print(f"check-docs: DESIGN.md agrees with the code ({len(capabilities)} capabilities)")
        return 0

    for check, line, message in sorted(out.items, key=lambda item: (item[0], item[1] or 0)):
        where = f"DESIGN.md:{line}" if line else "DESIGN.md"
        print(f"{where}: [{check}] {message}")

    print(f"\ncheck-docs: {len(out.items)} findings")
    return 1


if __name__ == "__main__":
    sys.exit(main())
