#!/usr/bin/env python3
"""Find ledger rows whose stale-note directive disagrees with state.

Catches three drift kinds:

1. directive-mismatch — row is CLAIMED/IN_REVIEW but note carries a
   stale-claim marker plus a release/demote/return-to-TODO verb. The
   conductor wrote the directive but never flipped the state column.

2. stale-recovery — row is IN_REVIEW/TODO/CLAIMED but the note says the
   underlying PR is already merged (STALE-FIXED / "already MERGED via" /
   "underlying PR merged"). Common with claude-rebase-pr-<N> recovery
   briefs that ship after the original PR was rebased through another
   path.

3. duplicate-row — same brief-id appears in more than one row. Worker
   claim-with-retry scripts fall through when this happens; conductor
   should keep the load-bearing row and delete the stale one.

The tool is read-only: it never edits the ledger.
"""

from __future__ import annotations

import re
import sys
from collections import defaultdict
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
RECOVERY_RE = re.compile(
    r"STALE-FIXED|underlying\s+PR\s+(?:already\s+)?merged|"
    r"PR\s+already\s+merged",
    re.IGNORECASE,
)
ACTIVE_STATES = {"CLAIMED", "IN_REVIEW"}
RECOVERY_STATES = {"CLAIMED", "IN_REVIEW", "TODO"}
TERMINAL_STATES = {"MERGED", "DONE", "DEFERRED-operator", "BLOCKED", "DROPPED"}
META_RE = re.compile(
    r"flags?\s+rows|regex|marker(?:s)?\s+plus|note\s+carr(?:y|ies)",
    re.IGNORECASE,
)


def usage() -> str:
    return """usage: scripts/find-stale-claim-drift.py [--quiet] [--limit N] [ledger]

Find BUILD_STATUS.md rows that drift from on-branch reality. Three
checks run on every invocation:

  * directive-mismatch — CLAIMED/IN_REVIEW row whose note says
    "Returning to TODO" / "demote" but state was never flipped.
  * stale-recovery — CLAIMED/IN_REVIEW/TODO row whose note says the
    underlying PR is already MERGED (e.g. recovery briefs that
    landed after the original was rebased through another path).
  * duplicate-row — same brief-id in more than one ledger row.

The tool is read-only: it never edits the ledger.

Exit codes:
  0  no drift found
  1  drift detected
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
    if brief.startswith("**") or not brief:
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


def recovery_marker(note: str) -> str | None:
    match = RECOVERY_RE.search(note)
    if not match:
        return None
    start = max(0, match.start() - 120)
    end = min(len(note), match.end() + 180)
    if META_RE.search(note[start:end]):
        return None
    return match.group(0)


def find_drift(text: str) -> list[tuple[str, str, str, str, str]]:
    matches: list[tuple[str, str, str, str, str]] = []
    rows_by_brief: dict[str, list[tuple[str, str]]] = defaultdict(list)

    for line in text.splitlines():
        parsed = parse_row(line)
        if parsed is None:
            continue
        brief, state, note = parsed
        rows_by_brief[brief].append((state, note))

        if state in ACTIVE_STATES:
            marker = directive_marker(note)
            if marker:
                matches.append(
                    ("directive-mismatch", brief, state, marker, clip(note))
                )

        if state in RECOVERY_STATES:
            marker = recovery_marker(note)
            if marker:
                matches.append(
                    ("stale-recovery", brief, state, marker, clip(note))
                )

    for brief, rows in rows_by_brief.items():
        if len(rows) <= 1:
            continue
        states = {state for state, _ in rows}
        if states.issubset(TERMINAL_STATES):
            continue
        states_str = ",".join(sorted(states))
        matches.append(
            (
                "duplicate-row",
                brief,
                states_str,
                f"x{len(rows)}",
                clip(rows[0][1]),
            )
        )

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
    by_kind: dict[str, int] = defaultdict(int)
    for kind, *_ in rows:
        by_kind[kind] += 1
    counts = ", ".join(f"{k}={v}" for k, v in sorted(by_kind.items())) or "none"
    print(f"Found {len(rows)} drift rows ({counts})")
    shown = rows if limit is None else rows[:limit]
    if not quiet:
        for kind, brief, state, marker, note in shown:
            print(f"[{kind}] {brief} | {state} | {marker} | {note}")
    return 1 if rows else 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
