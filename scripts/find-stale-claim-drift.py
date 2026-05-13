#!/usr/bin/env python3
"""Find ledger rows whose stale-note directive disagrees with state."""

from __future__ import annotations

import re
import sys
from pathlib import Path

DEFAULT_LEDGER = Path("docs/build-plan/BUILD_STATUS.md")
MARKER_RE = re.compile(
    r"STALE-ROAR-INVALIDATION|STALE-CLAIM-DEMOTE|STALE-SWEEP|"
    r"no remote-branch push in",
    re.IGNORECASE,
)
VERB_RE = re.compile(
    r"return(?:ing)?\s+to\s+TODO|demot(?:e|ed|ing)|"
    r"releas(?:e|ed|ing)|flip(?:ped)?\s+back\s+to\s+TODO",
    re.IGNORECASE,
)
STATES = {"CLAIMED", "IN_REVIEW"}
META_RE = re.compile(
    r"flags?\s+rows|regex|marker(?:s)?\s+plus|note\s+carr(?:y|ies)",
    re.IGNORECASE,
)


def usage() -> str:
    return """usage: scripts/find-stale-claim-drift.py [--quiet] [--limit N] [ledger]

Find BUILD_STATUS.md rows whose state is CLAIMED or IN_REVIEW while the
note contains a stale-claim marker plus a release/demote/return-to-TODO
verb. The tool is read-only: it never edits the ledger.

Exit codes:
  0  no stale-claim drift found
  1  stale-claim drift detected
  2  usage error or unreadable ledger

Cron example:
  */10 * * * * cd /path/to/argo && scripts/find-stale-claim-drift.py --quiet
"""


def parse_args(argv: list[str]) -> tuple[bool, int | None, Path]:
    quiet = False
    limit = None
    ledger = DEFAULT_LEDGER
    i = 0
    while i < len(argv):
        arg = argv[i]
        if arg in {"-h", "--help"}:
            print(usage(), end="")
            raise SystemExit(0)
        if arg == "--quiet":
            quiet = True
        elif arg == "--limit":
            i += 1
            if i >= len(argv):
                raise ValueError("--limit requires a number")
            try:
                limit = int(argv[i])
            except ValueError as exc:
                raise ValueError("--limit requires a number") from exc
            if limit < 0:
                raise ValueError("--limit must be non-negative")
        elif arg.startswith("--"):
            raise ValueError(f"unknown flag: {arg}")
        else:
            ledger = Path(arg)
        i += 1
    return quiet, limit, ledger


def parse_row(line: str) -> tuple[str, str, str] | None:
    if not line.startswith("| "):
        return None
    cells = line.rstrip("\n").split("|")
    if len(cells) < 7:
        return None
    brief = cells[1].strip()
    state = cells[3].strip()
    note = "|".join(cells[5:-1]).strip()
    if brief.startswith("**") or state not in STATES:
        return None
    return brief, state, note


def clip(text: str, size: int = 150) -> str:
    normalized = " ".join(text.split())
    if len(normalized) <= size:
        return normalized
    return normalized[: size - 3].rstrip() + "..."


def directive_marker(note: str) -> str | None:
    for marker in MARKER_RE.finditer(note):
        start = max(0, marker.start() - 120)
        end = min(len(note), marker.end() + 180)
        context = note[start:end]
        if VERB_RE.search(context) and not META_RE.search(context):
            return marker.group(0)
    return None


def find_drift(text: str) -> list[tuple[str, str, str, str]]:
    matches = []
    for line in text.splitlines():
        parsed = parse_row(line)
        if parsed is None:
            continue
        brief, state, note = parsed
        marker = directive_marker(note)
        if marker:
            matches.append((brief, state, marker, clip(note)))
    return matches


def main(argv: list[str]) -> int:
    try:
        quiet, limit, ledger = parse_args(argv)
    except ValueError as exc:
        print(f"error: {exc}", file=sys.stderr)
        print(usage(), file=sys.stderr, end="")
        return 2

    try:
        text = ledger.read_text(encoding="utf-8")
    except OSError as exc:
        print(f"error: cannot read {ledger}: {exc}", file=sys.stderr)
        return 2

    rows = find_drift(text)
    print(f"Found {len(rows)} rows with stale-claim drift")
    shown = rows if limit is None else rows[:limit]
    if not quiet:
        for brief, state, marker, note in shown:
            print(f"{brief} | {state} | {marker} | {note}")
    return 1 if rows else 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
