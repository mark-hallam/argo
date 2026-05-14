#!/usr/bin/env python3
"""Detect duplicate public type names across crates.

The default mode is warning-only: findings are printed and the script exits 0.
Use --enforce to turn findings into exit code 1 after the canonicalization
briefs finish.
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path

DECL_RE = re.compile(
    r"^\s*pub(?:\([^)]*\))?\s+(?P<kind>struct|enum|type)\s+"
    r"(?P<name>[A-Za-z_][A-Za-z0-9_]*)\b",
    re.MULTILINE,
)


@dataclass(frozen=True)
class Decl:
    """One public type declaration found in a crate."""

    kind: str
    name: str
    crate: str
    path: Path
    line: int


def repo_root() -> Path:
    return Path(__file__).resolve().parents[1]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--enforce",
        action="store_true",
        help="exit 1 when duplicate public type names are found",
    )
    parser.add_argument(
        "--check-changed-files",
        action="store_true",
        help="only report duplicate groups touched by staged Rust files",
    )
    parser.add_argument(
        "--allowlist",
        default="scripts/lint-parallel-pub-types.allowlist",
        help="allowlist path, relative to repo root unless absolute",
    )
    return parser.parse_args()


def staged_rust_files(root: Path) -> set[Path]:
    output = subprocess.run(
        ["git", "-C", str(root), "diff", "--cached", "--name-only", "--", "crates/**/*.rs"],
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
    ).stdout
    return {root / line.strip() for line in output.splitlines() if line.strip()}


def crate_name(path: Path, root: Path) -> str | None:
    try:
        rel = path.relative_to(root)
    except ValueError:
        return None
    parts = rel.parts
    if len(parts) >= 4 and parts[0] == "crates" and parts[2] == "src":
        return parts[1]
    return None


def iter_decls(root: Path) -> list[Decl]:
    decls: list[Decl] = []
    for path in sorted((root / "crates").glob("*/src/**/*.rs")):
        crate = crate_name(path, root)
        if crate is None:
            continue
        try:
            text = path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            text = path.read_text(errors="replace")
        for match in DECL_RE.finditer(text):
            line = text.count("\n", 0, match.start()) + 1
            decls.append(
                Decl(
                    kind=match.group("kind"),
                    name=match.group("name"),
                    crate=crate,
                    path=path,
                    line=line,
                )
            )
    return decls


def load_allowlist(root: Path, raw_path: str) -> set[tuple[str, str, str]]:
    path = Path(raw_path)
    if not path.is_absolute():
        path = root / path
    allowed: set[tuple[str, str, str]] = set()
    if not path.exists():
        return allowed
    for line_no, raw in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
        line = raw.split("#", 1)[0].strip()
        if not line:
            continue
        parts = line.split()
        if len(parts) != 3:
            print(
                f"warning: ignoring malformed allowlist line {path}:{line_no}: {raw}",
                file=sys.stderr,
            )
            continue
        name, left, right = parts
        a, b = sorted((left, right))
        allowed.add((name, a, b))
    return allowed


def duplicate_findings(
    decls: list[Decl], allowlist: set[tuple[str, str, str]], changed: set[Path] | None
) -> list[tuple[str, list[Decl], list[tuple[str, str]]]]:
    by_name: dict[str, list[Decl]] = defaultdict(list)
    for decl in decls:
        by_name[decl.name].append(decl)

    findings: list[tuple[str, list[Decl], list[tuple[str, str]]]] = []
    for name, group in sorted(by_name.items()):
        crates = sorted({decl.crate for decl in group})
        if len(crates) < 2:
            continue
        if changed is not None and not any(decl.path in changed for decl in group):
            continue
        unallowed_pairs = []
        for i, left in enumerate(crates):
            for right in crates[i + 1 :]:
                if (name, left, right) not in allowlist:
                    unallowed_pairs.append((left, right))
        if unallowed_pairs:
            findings.append((name, sorted(group, key=lambda d: (d.crate, d.path, d.line)), unallowed_pairs))
    return findings


def print_findings(root: Path, findings: list[tuple[str, list[Decl], list[tuple[str, str]]]]) -> None:
    if not findings:
        print("parallel-pub-types: ok")
        return
    print(f"parallel-pub-types: {len(findings)} duplicate public type name(s)")
    for name, group, pairs in findings:
        crates = ", ".join(sorted({decl.crate for decl in group}))
        pair_text = ", ".join(f"{a}/{b}" for a, b in pairs)
        print(f"- {name}: crates={crates}; unallowed_pairs={pair_text}")
        for decl in group:
            rel = decl.path.relative_to(root)
            print(f"  {decl.kind:<6} {decl.crate:<32} {rel}:{decl.line}")


def main() -> int:
    args = parse_args()
    root = repo_root()
    changed = staged_rust_files(root) if args.check_changed_files else None
    if args.check_changed_files and not changed:
        print("parallel-pub-types: no staged Rust files")
        return 0
    findings = duplicate_findings(iter_decls(root), load_allowlist(root, args.allowlist), changed)
    print_findings(root, findings)
    if findings and args.enforce:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
