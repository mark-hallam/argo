#!/usr/bin/env python3
"""Enforce CLAUDE.md invariant: "No `unwrap`/`expect` outside `#[cfg(test)]`".

Scans `crates/*/src/**/*.rs` and classifies every `.unwrap()` and `.expect(`
callsite. Exits 0 when zero PRODUCTION violations are found, 1 otherwise.

Classifications:
  TEST-CONTEXT  inside #[cfg(test)] mod, *_tests/, or *test* helper file
  DOC-EXAMPLE   inside a `///` or `//!` doc-comment line
  COMMENT       inside a `//` line comment or `/* ... */` block comment
  BUILD-SCRIPT  inside a build.rs (separate enforcement track)
  PRODUCTION    production-path violation — FIX

Heuristics, intentionally simple:
  - A line that, after stripping leading whitespace, starts with `///`, `//!`
    is a doc comment.
  - A line that starts with `//` (and isn't `///`/`//!`) is an ordinary
    comment.
  - Block comments `/* ... */` are tracked across lines.
  - A `#[cfg(test)]`, `#[cfg(any(test, ...))]`, or `#[cfg(all(test, ...))]`
    attribute immediately preceding a `mod <name>` opens a test region.
    The region extends until its matching closing brace at the same brace
    depth.
  - We also track `#[cfg(test)]` attached to a single `fn`/`impl`/`struct`/
    `enum` item; everything inside is TEST-CONTEXT.
  - Anything inside `mod tests`/`mod test`/`mod *_tests` is also treated as
    TEST-CONTEXT (covers `#[cfg(test)] mod tests` patterns where the
    attribute may sit on a separate line we already handled).
  - Files under `tests/`, `benches/`, `examples/` directories are excluded
    from scanning (test code by Cargo convention).

This is a defensive classifier — it errs on the side of marking ambiguous
matches as PRODUCTION so they receive a deliberate fix.
"""

from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable

UNWRAP_RE = re.compile(r"\.unwrap\(\)|\.expect\(")
CFG_TEST_RE = re.compile(
    r"^\s*#\[cfg\(\s*(?:test|any\([^)]*\btest\b[^)]*\)|all\([^)]*\btest\b[^)]*\))\s*\)\]"
)
MOD_RE = re.compile(r"^\s*(?:pub(?:\s*\([^)]*\))?\s+)?mod\s+([A-Za-z_][A-Za-z0-9_]*)")
TEST_MOD_NAME_RE = re.compile(r"^(?:tests?|.*_tests?|test_.*)$")


@dataclass
class Finding:
    file: str
    line: int
    column: int
    pattern: str
    classification: str
    snippet: str


def is_test_file_path(p: Path) -> bool:
    parts = p.parts
    if "tests" in parts or "benches" in parts or "examples" in parts:
        return True
    name = p.name
    if name == "build.rs":
        return False  # handled separately as BUILD-SCRIPT
    # By-convention test-only file names: `tests.rs`, `test.rs`,
    # `*_tests.rs`, `*_test.rs`, `tests_*.rs`, `test_*.rs`.
    if name in ("tests.rs", "test.rs"):
        return True
    if name.endswith("_tests.rs") or name.endswith("_test.rs"):
        return True
    if name.startswith("tests_") or name.startswith("test_"):
        return True
    return False


# Cache parent-mod test-cfg lookups so we don't reparse every parent
# for every child mod.
_PARENT_TEST_MOD_CACHE: dict[Path, tuple[set[str], set[str]]] = {}

_PATH_ATTR_RE = re.compile(r'^\s*#\[\s*path\s*=\s*"([^"]+)"\s*\]')

# Global cache of files that are reached via at least one `#[cfg(test)]`
# edge in the mod-include graph. Populated lazily per crate root the
# first time any file in that crate is scanned.
_TEST_CONTEXT_FILES: set[Path] = set()
_GRAPH_BUILT_FOR_CRATES: set[Path] = set()


def _crate_root(p: Path) -> Path | None:
    """Walk up until we find a directory containing Cargo.toml."""
    cursor = p.parent
    while True:
        if (cursor / "Cargo.toml").is_file():
            return cursor
        if cursor.parent == cursor:
            return None
        cursor = cursor.parent


def _parse_mod_edges(file: Path) -> list[tuple[Path, bool]]:
    """Return a list of (child_file, cfg_test_attached) edges declared
    by `file`. Resolves both `mod foo;` and `#[path = "rel.rs"] mod foo;`
    forms. Skips declarations whose target file does not exist (the
    target may be generated at build time, etc.)."""
    try:
        text = file.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
        return []
    edges: list[tuple[Path, bool]] = []
    pending_cfg = False
    pending_path: str | None = None
    file_dir = file.parent
    file_stem = file.stem
    # Rust resolves child modules relative to:
    #   - lib.rs / main.rs / mod.rs: same dir
    #   - foo.rs (non-mod.rs): "foo/" subdir
    if file.name in ("lib.rs", "main.rs", "mod.rs"):
        child_dir = file_dir
    else:
        child_dir = file_dir / file_stem
    for raw in text.splitlines():
        s = raw.strip()
        if not s or s.startswith("//"):
            continue
        if CFG_TEST_RE.match(raw):
            pending_cfg = True
            continue
        m_path = _PATH_ATTR_RE.match(raw)
        if m_path:
            pending_path = m_path.group(1)
            continue
        if s.startswith("#["):
            continue
        m = re.match(
            r"^\s*(?:pub(?:\s*\([^)]*\))?\s+)?mod\s+([A-Za-z_][A-Za-z0-9_]*)\s*;",
            raw,
        )
        if m:
            mod_name = m.group(1)
            target: Path | None = None
            if pending_path:
                # path is relative to the file containing the mod decl.
                target = (file_dir / pending_path).resolve()
            else:
                # Try sibling `mod_name.rs`, then `mod_name/mod.rs` in
                # both candidate child dirs.
                for base in {file_dir, child_dir}:
                    cand1 = base / f"{mod_name}.rs"
                    cand2 = base / mod_name / "mod.rs"
                    if cand1.is_file():
                        target = cand1.resolve()
                        break
                    if cand2.is_file():
                        target = cand2.resolve()
                        break
            if target is not None and target.exists():
                edges.append((target, pending_cfg))
            pending_cfg = False
            pending_path = None
            continue
        # Inline `mod foo { ... }` — same-file, ignored for the cross-
        # file graph (in-file cfg(test) tracking handles it inside
        # scan_file).
        pending_cfg = False
        pending_path = None
    return edges


def _ensure_graph_for(crate_dir: Path) -> None:
    """Build the transitive test-context closure for every .rs file
    under `crate_dir/src/`, caching results in _TEST_CONTEXT_FILES.
    Walks the mod-include graph starting from the crate roots
    (lib.rs / main.rs)."""
    if crate_dir in _GRAPH_BUILT_FOR_CRATES:
        return
    _GRAPH_BUILT_FOR_CRATES.add(crate_dir)
    src = crate_dir / "src"
    if not src.is_dir():
        return
    # Build the entire mod-include graph for this crate up-front.
    graph: dict[Path, list[tuple[Path, bool]]] = {}
    rs_files = [
        p.resolve() for p in src.rglob("*.rs") if "target" not in p.parts
    ]
    for f in rs_files:
        graph[f] = _parse_mod_edges(f)

    # Walk from crate roots, propagating cfg_test marker downward.
    # A file becomes test-context if any path from a root to it goes
    # through at least one cfg(test) edge. Once a node is test-context,
    # all of its descendants are also test-context.
    roots = [src / "lib.rs", src / "main.rs"]
    roots = [r.resolve() for r in roots if r.is_file()]

    # `visited` maps file -> "best" status seen so far: True if it's
    # been reached as test-context, False if reached only as production.
    # If we later reach a file as test-context, we propagate that
    # downward. Use a worklist algorithm.
    status: dict[Path, bool] = {}

    worklist: list[tuple[Path, bool]] = [(r, False) for r in roots]
    while worklist:
        node, is_test = worklist.pop()
        # Already at "at least as test as we'd set" → skip.
        if node in status and status[node] == is_test:
            continue
        # If we previously visited as production, and now visiting as
        # test, we overwrite to test (test is "stronger"); production
        # cannot overwrite test.
        if node in status and status[node] and not is_test:
            continue
        status[node] = is_test
        for child, edge_cfg in graph.get(node, []):
            child_is_test = is_test or edge_cfg
            worklist.append((child, child_is_test))

    for f, is_test in status.items():
        if is_test:
            _TEST_CONTEXT_FILES.add(f)


def _is_test_context_file(p: Path) -> bool:
    crate = _crate_root(p)
    if crate is None:
        return False
    _ensure_graph_for(crate)
    return p.resolve() in _TEST_CONTEXT_FILES


def _cfg_test_mods_in(parent_file: Path) -> tuple[set[str], set[str]]:
    """Return (mod_names, path_targets) for `#[cfg(test)] mod <...>`
    declarations in `parent_file`. `mod_names` matches a sibling
    `<name>.rs`; `path_targets` matches by file basename for the
    `#[path = "x.rs"] mod <whatever>;` pattern."""
    if parent_file in _PARENT_TEST_MOD_CACHE:
        return _PARENT_TEST_MOD_CACHE[parent_file]
    test_mods: set[str] = set()
    test_paths: set[str] = set()
    try:
        text = parent_file.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
        _PARENT_TEST_MOD_CACHE[parent_file] = (test_mods, test_paths)
        return _PARENT_TEST_MOD_CACHE[parent_file]
    lines = text.splitlines()
    pending_cfg = False
    pending_path: str | None = None
    for raw in lines:
        s = raw.strip()
        if not s or s.startswith("//"):
            continue
        if CFG_TEST_RE.match(raw):
            pending_cfg = True
            continue
        m_path = _PATH_ATTR_RE.match(raw)
        if m_path:
            pending_path = m_path.group(1)
            continue
        if s.startswith("#["):
            continue
        m = re.match(
            r"^\s*(?:pub(?:\s*\([^)]*\))?\s+)?mod\s+([A-Za-z_][A-Za-z0-9_]*)\s*;",
            raw,
        )
        if m:
            if pending_cfg:
                if pending_path:
                    test_paths.add(pending_path)
                else:
                    test_mods.add(m.group(1))
            pending_cfg = False
            pending_path = None
            continue
        pending_cfg = False
        pending_path = None
    _PARENT_TEST_MOD_CACHE[parent_file] = (test_mods, test_paths)
    return _PARENT_TEST_MOD_CACHE[parent_file]


def is_test_via_parent_mod(p: Path) -> bool:
    """Return True if this file is included into its crate via a
    `#[cfg(test)] mod <name>;` (or `#[path = "x.rs"] #[cfg(test)] mod …;`)
    declaration in any sibling parent (mod.rs / sibling .rs / crate
    root lib.rs / main.rs)."""
    name = p.stem
    if name in ("mod", "lib", "main"):
        return False
    parent_dir = p.parent
    candidates: list[Path] = []
    # All sibling .rs files in the same dir (mod.rs and any other module
    # file that may use `#[path = "..."] mod …;` to pull in a sibling).
    if parent_dir.is_dir():
        for sib in parent_dir.iterdir():
            if sib.is_file() and sib.suffix == ".rs" and sib != p:
                candidates.append(sib)
    # Walk up to the crate root (where Cargo.toml lives) and add the
    # conventional crate roots, plus any sibling .rs in the crate-root
    # src/ dir.
    cursor = parent_dir
    while True:
        if (cursor / "Cargo.toml").is_file():
            for crate_root in ("lib.rs", "main.rs"):
                candidate = cursor / "src" / crate_root
                if candidate.is_file() and candidate not in candidates:
                    candidates.append(candidate)
                candidate = cursor / crate_root
                if candidate.is_file() and candidate not in candidates:
                    candidates.append(candidate)
            break
        if cursor.parent == cursor:
            break
        cursor = cursor.parent
    for cand in candidates:
        if not cand.is_file():
            continue
        mods, paths = _cfg_test_mods_in(cand)
        if name in mods:
            return True
        if p.name in paths:
            return True
        for target in paths:
            if str(p).endswith(target):
                return True
    return False


def strip_block_comments(line: str, in_block: bool) -> tuple[str, bool]:
    """Strip /* ... */ block-comment content and return the stripped line
    plus the new in-block flag. Handles open and close on the same line."""
    out = []
    i = 0
    while i < len(line):
        if in_block:
            close = line.find("*/", i)
            if close == -1:
                return "".join(out), True
            i = close + 2
            in_block = False
        else:
            open_ = line.find("/*", i)
            if open_ == -1:
                out.append(line[i:])
                return "".join(out), False
            # Don't swallow `/*` that lives inside a line comment.
            line_comment = line.find("//", i)
            if line_comment != -1 and line_comment < open_:
                out.append(line[i:line_comment])
                return "".join(out), False
            out.append(line[i:open_])
            i = open_ + 2
            in_block = True
    return "".join(out), in_block


def strip_strings(
    line: str, raw_hashes: int, in_normal_string: bool = False
) -> tuple[str, int, bool]:
    """Replace the content of string literals (including multi-line raw
    and normal strings) with spaces so the brace counter sees only code.

    `raw_hashes`: number of `#` chars in the currently-open raw string
    (`r#"..."#`, `r##"..."##`, …) or 0 if no raw string is open.
    `in_normal_string`: True if the previous line ended inside an
    unterminated double-quoted string (rare but legal via `\` line
    continuation in `format!` and friends).

    Returns (cleaned_line, new_raw_hashes, new_in_normal_string).
    """
    out = []
    i = 0
    n = len(line)
    if in_normal_string:
        # Scan to the closing `"` (skipping `\\` escapes); everything
        # before it is string content.
        while i < n:
            if line[i] == "\\" and i + 1 < n:
                i += 2
                out.append("  ")
                continue
            if line[i] == '"':
                break
            i += 1
            out.append(" ")
        if i < n:
            out.append('"')
            i += 1
            in_normal_string = False
        else:
            return "".join(out), raw_hashes, True
    while i < n:
        if raw_hashes > 0:
            close = '"' + ("#" * raw_hashes)
            idx = line.find(close, i)
            if idx == -1:
                out.append(" " * (n - i))
                return "".join(out), raw_hashes, False
            out.append(" " * (idx - i))
            out.append(close)
            i = idx + len(close)
            raw_hashes = 0
            continue
        c = line[i]
        if c == "/" and i + 1 < n and line[i + 1] == "/":
            # Rest-of-line `//` comment — drop the remainder.
            return "".join(out), raw_hashes, in_normal_string
        if c == "r":
            # Detect raw-string opener: r#*"
            j = i + 1
            while j < n and line[j] == "#":
                j += 1
            if j < n and line[j] == '"' and j > i:
                hashes = j - i - 1
                if hashes == 0:
                    close_idx = line.find('"', j + 1)
                    out.append(line[i : j + 1])
                    if close_idx == -1:
                        out.append(" " * (n - (j + 1)))
                        return "".join(out), 0, False
                    out.append(" " * (close_idx - (j + 1)))
                    out.append('"')
                    i = close_idx + 1
                    continue
                # Hashed raw string — span possibly multi-line.
                out.append(line[i : j + 1])
                raw_hashes = hashes
                i = j + 1
                continue
            # Otherwise just a regular `r` char.
            out.append(c)
            i += 1
            continue
        if c == "b" and i + 1 < n and line[i + 1] == '"':
            # Byte string `b"..."`. Strip until next un-escaped `"`.
            out.append('b"')
            j = i + 2
            while j < n:
                if line[j] == "\\" and j + 1 < n:
                    j += 2
                    continue
                if line[j] == '"':
                    break
                j += 1
            out.append(" " * (j - (i + 2)))
            if j < n:
                out.append('"')
                i = j + 1
            else:
                i = j
            continue
        if c == '"':
            # Normal string: find closing un-escaped `"`.
            j = i + 1
            while j < n:
                if line[j] == "\\" and j + 1 < n:
                    j += 2
                    continue
                if line[j] == '"':
                    break
                j += 1
            out.append('"')
            out.append(" " * max(0, j - (i + 1)))
            if j < n:
                out.append('"')
                i = j + 1
            else:
                # String continues onto next line — Rust allows `\`
                # line-continuation at EOL inside a normal string.
                return "".join(out), raw_hashes, True
            continue
        if c == "'":
            # Distinguish a char literal (`'x'`, `'\n'`, `b'{'`) from a
            # lifetime (`'a`, `'static`). A char literal is followed by
            # a closing `'` within a short distance and may include an
            # escape; a lifetime is a quote followed by an identifier
            # and NOT closed by a `'`.
            if i + 2 < n and line[i + 1] == "\\":
                # Escape sequence: scan to the closing `'`.
                j = i + 2
                while j < n and line[j] != "'":
                    j += 1
                if j < n:
                    # Replace inner chars with spaces of same length.
                    out.append("'")
                    out.append(" " * (j - (i + 1)))
                    out.append("'")
                    i = j + 1
                    continue
                # Unclosed; treat as raw char (fall through).
            elif i + 2 < n and line[i + 2] == "'":
                # `'x'` — single char between quotes.
                out.append("'")
                out.append(" ")
                out.append("'")
                i += 3
                continue
            # Otherwise treat as a lifetime — emit the quote, let the
            # brace counter ignore it.
            out.append(c)
            i += 1
            continue
        out.append(c)
        i += 1
    return "".join(out), raw_hashes, in_normal_string


def count_braces(line: str) -> tuple[int, int]:
    """Count `{` and `}` chars in `line`. Strings should already have
    been stripped by `strip_strings`."""
    return line.count("{"), line.count("}")


def scan_file(path: Path) -> list[Finding]:
    findings: list[Finding] = []
    try:
        text = path.read_text(encoding="utf-8")
    except UnicodeDecodeError:
        return findings

    if path.name == "build.rs":
        # Mark all hits as BUILD-SCRIPT, but don't fail on them.
        for lineno, raw in enumerate(text.splitlines(), 1):
            for m in UNWRAP_RE.finditer(raw):
                findings.append(
                    Finding(
                        file=str(path),
                        line=lineno,
                        column=m.start() + 1,
                        pattern=m.group(0),
                        classification="BUILD-SCRIPT",
                        snippet=raw.strip()[:160],
                    )
                )
        return findings

    if is_test_file_path(path) or _is_test_context_file(path):
        # Mark all hits as TEST-CONTEXT.
        for lineno, raw in enumerate(text.splitlines(), 1):
            for m in UNWRAP_RE.finditer(raw):
                findings.append(
                    Finding(
                        file=str(path),
                        line=lineno,
                        column=m.start() + 1,
                        pattern=m.group(0),
                        classification="TEST-CONTEXT",
                        snippet=raw.strip()[:160],
                    )
                )
        return findings

    lines = text.splitlines()

    # Track stack of (open_brace_depth_at_open, kind) for test regions.
    # depth is incremented per `{` encountered after the cfg(test) attribute.
    test_stack: list[int] = []  # depths at which a test region is OPEN
    brace_depth = 0
    in_block_comment = False
    raw_hashes = 0  # number of `#` chars in an open multi-line raw string
    in_normal_string = False  # `"` opened on prior line, not yet closed
    pending_cfg_test = False  # set after we saw a #[cfg(test)] attribute
    # Track whether the next item to open a brace is a test region.

    for lineno, raw in enumerate(lines, 1):
        # If we're inside an open multi-line raw string when this line
        # starts, treat the whole line as code (raw strings can't open a
        # block comment).
        if raw_hashes > 0 or in_normal_string:
            stripped_for_comment = raw
        else:
            stripped_for_comment, in_block_comment = strip_block_comments(
                raw, in_block_comment
            )

        code_only, raw_hashes, in_normal_string = strip_strings(
            stripped_for_comment, raw_hashes, in_normal_string
        )

        ls = stripped_for_comment.lstrip()

        # Whole-line comment? Note: classify still happens per-match using
        # the raw line, so we don't skip — but we DO want to skip
        # cfg/mod detection for comment lines.
        is_line_comment = ls.startswith("//")
        is_doc_comment = ls.startswith("///") or ls.startswith("//!")

        if not is_line_comment:
            if CFG_TEST_RE.match(code_only):
                pending_cfg_test = True

            # If a `mod X` appears, decide whether it should open a test
            # region either because of the pending #[cfg(test)] attribute,
            # OR because the name itself is `tests` / `*_tests` / `test`.
            mod_match = MOD_RE.match(code_only)
            if mod_match:
                name = mod_match.group(1)
                opens_test_region = pending_cfg_test or bool(
                    TEST_MOD_NAME_RE.match(name)
                )
                pending_cfg_test = False  # consumed
                if opens_test_region:
                    # Whether the `{` lives on this line or later, the
                    # region's base depth is the current brace_depth and
                    # closes when we return to that level.
                    test_stack.append(brace_depth)

        opens, closes = count_braces(code_only)

        # Classify any callsites on this line BEFORE updating brace_depth,
        # because a closing brace at end-of-line ends the region only
        # AFTER the line's code ran.
        if not is_line_comment and not is_doc_comment:
            # in_test if any region in stack still open at current
            # brace_depth >= region depth
            in_test = any(brace_depth >= d for d in test_stack)
            # Match against `code_only` so `.unwrap()` appearing inside
            # string literals (e.g. `concat!(..., "…unwrap();")`) does
            # not get reported.
            for m in UNWRAP_RE.finditer(code_only):
                if is_doc_comment:
                    cls = "DOC-EXAMPLE"
                elif is_line_comment:
                    cls = "COMMENT"
                elif in_test:
                    cls = "TEST-CONTEXT"
                else:
                    cls = "PRODUCTION"
                findings.append(
                    Finding(
                        file=str(path),
                        line=lineno,
                        column=m.start() + 1,
                        pattern=m.group(0),
                        classification=cls,
                        snippet=raw.strip()[:160],
                    )
                )
        elif is_doc_comment:
            for m in UNWRAP_RE.finditer(raw):
                findings.append(
                    Finding(
                        file=str(path),
                        line=lineno,
                        column=m.start() + 1,
                        pattern=m.group(0),
                        classification="DOC-EXAMPLE",
                        snippet=raw.strip()[:160],
                    )
                )
        else:  # is_line_comment but not doc
            for m in UNWRAP_RE.finditer(raw):
                findings.append(
                    Finding(
                        file=str(path),
                        line=lineno,
                        column=m.start() + 1,
                        pattern=m.group(0),
                        classification="COMMENT",
                        snippet=raw.strip()[:160],
                    )
                )

        brace_depth += opens - closes
        if brace_depth < 0:
            brace_depth = 0

        # Pop any test regions whose depth is now >= current depth.
        # A region opened at depth D is closed when brace_depth < D + 1
        # is false at line-end (i.e., we returned to depth D).
        test_stack = [d for d in test_stack if brace_depth > d]

    return findings


def iter_rust_files(root: Path) -> Iterable[Path]:
    crates = root / "crates"
    if not crates.is_dir():
        return
    for path in crates.rglob("*.rs"):
        # Skip `target/` if it lives anywhere under crates (it shouldn't,
        # but be safe).
        if "target" in path.parts:
            continue
        yield path


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--verbose", action="store_true")
    ap.add_argument("--all", action="store_true",
                    help="Print every finding, not just PRODUCTION.")
    ap.add_argument("--json", action="store_true",
                    help="Emit findings as JSON.")
    args = ap.parse_args()

    root = Path.cwd()
    all_findings: list[Finding] = []
    for path in iter_rust_files(root):
        all_findings.extend(scan_file(path))

    by_class: dict[str, int] = {}
    for f in all_findings:
        by_class[f.classification] = by_class.get(f.classification, 0) + 1

    prod = [f for f in all_findings if f.classification == "PRODUCTION"]

    if args.json:
        import json
        out = [
            dict(file=f.file, line=f.line, column=f.column, pattern=f.pattern,
                 classification=f.classification, snippet=f.snippet)
            for f in (all_findings if args.all else prod)
        ]
        print(json.dumps(out, indent=2))
    else:
        if args.verbose or args.all:
            print("# Counts by classification:")
            for cls in sorted(by_class):
                print(f"  {cls}: {by_class[cls]}")
            print()
        if prod:
            print(f"# PRODUCTION violations: {len(prod)}")
            for f in prod:
                print(f"{f.file}:{f.line}:{f.column} [{f.pattern}] {f.snippet}")
        else:
            print("OK: 0 PRODUCTION violations.")
            if args.verbose:
                print("Other classifications (non-blocking):")
                for cls in sorted(by_class):
                    if cls != "PRODUCTION":
                        print(f"  {cls}: {by_class[cls]}")

    return 1 if prod else 0


if __name__ == "__main__":
    sys.exit(main())
