#!/usr/bin/env python3
"""Find drift between authoritative source-of-truth docs and shipped reality.

Sibling to `find-stale-claim-drift.py` (which sweeps BUILD_STATUS.md). This
one sweeps the two human-curated docs that drift most reliably:

  * docs/skycot/V1_DEFINITION.md — the v1.0 ship checklist. Specific
    items here are flagged when their underlying work has shipped but
    the checkbox is still `[ ]`. We use a hand-maintained registry of
    known-shipped items (KNOWN_SHIPPED below). Add to it as ship-blockers
    land.

  * docs/build-plan/RESUME.md — the fresh-session primer. Two patterns
    are flagged:
      - a `Last sweep:` line older than 30 days (advisory; the brief
        that introduced this script set the cadence at 14 days, but we
        give callers room before alarming).
      - any line answering "Is codex active?" with "no" or "offline",
        which historically contradicts the live-status banner on line 12.

The tool is read-only.

Exit codes:
  0  no drift found
  1  drift detected
  2  usage error or unreadable file

Wire-up: the conductor's tick prompt may invoke this alongside
`scripts/find-stale-claim-drift.py` to surface paper-trail drift.
"""

from __future__ import annotations

import re
import sys
from dataclasses import dataclass
from datetime import date, datetime
from pathlib import Path

DEFAULT_V1 = Path("docs/skycot/V1_DEFINITION.md")
DEFAULT_RESUME = Path("docs/build-plan/RESUME.md")

# Hand-maintained registry of items that have shipped. Each entry says:
#   - id: the brief-id or short marker the ship checklist line mentions
#   - pr: the merge PR number(s) (string; use post-#NNNN for follow-ups)
# When the V1_DEFINITION.md line containing `id` still starts with `[ ]`,
# we flag it: the box should be checked + annotated with the PR.
KNOWN_SHIPPED: list[tuple[str, str]] = [
    ("claude-skill-library-worker-load-wire", "#1682 (cross-tenant test #1702)"),
    ("claude-migration-slot-dedup-and-validator", "post #1735"),
]

# "Is codex active?" Q&A drift markers — substrings whose presence in
# the same Q&A line means RESUME.md still believes codex is offline.
CODEX_OFFLINE_PATTERNS = (
    re.compile(r"is\s+codex\s+active.*\bno\b", re.IGNORECASE),
    re.compile(r"is\s+codex\s+active.*offline", re.IGNORECASE),
    re.compile(r"is\s+codex\s+active.*skip\s+codex", re.IGNORECASE),
)

LAST_SWEEP_RE = re.compile(r"Last sweep:\s*(\d{4}-\d{2}-\d{2})")
LAST_SWEEP_STALE_DAYS = 30


@dataclass
class Drift:
    kind: str
    file: Path
    detail: str


def usage() -> str:
    return """usage: scripts/find-stale-doc-drift.py [--quiet] [--v1 PATH] [--resume PATH]

Sweep V1_DEFINITION.md + RESUME.md for known-shipped drift and
RESUME.md self-contradictions.

  --quiet         suppress per-row output; print only the summary.
  --v1 PATH       override docs/skycot/V1_DEFINITION.md path.
  --resume PATH   override docs/build-plan/RESUME.md path.

Exit 0 if no drift, 1 if drift, 2 on usage error.
"""


def parse_args(argv: list[str]) -> tuple[bool, Path, Path]:
    quiet = False
    v1 = DEFAULT_V1
    resume = DEFAULT_RESUME
    i = 0
    while i < len(argv):
        arg = argv[i]
        if arg in {"-h", "--help"}:
            print(usage(), end="")
            raise SystemExit(0)
        if arg == "--quiet":
            quiet = True
        elif arg == "--v1":
            i += 1
            if i >= len(argv):
                raise ValueError("--v1 requires a path")
            v1 = Path(argv[i])
        elif arg == "--resume":
            i += 1
            if i >= len(argv):
                raise ValueError("--resume requires a path")
            resume = Path(argv[i])
        elif arg.startswith("--"):
            raise ValueError(f"unknown flag: {arg}")
        else:
            raise ValueError(f"unexpected positional argument: {arg}")
        i += 1
    return quiet, v1, resume


def scan_v1(v1_path: Path) -> list[Drift]:
    if not v1_path.exists():
        return [Drift("missing-file", v1_path, "file not found")]
    drifts: list[Drift] = []
    text = v1_path.read_text(encoding="utf-8")
    lines = text.splitlines()
    for shipped_id, pr in KNOWN_SHIPPED:
        for idx, line in enumerate(lines, start=1):
            if shipped_id not in line:
                continue
            stripped = line.lstrip()
            if stripped.startswith("- [ ]") or stripped.startswith("* [ ]"):
                drifts.append(
                    Drift(
                        "unchecked-shipped",
                        v1_path,
                        f"line {idx}: `{shipped_id}` is shipped ({pr})"
                        f" but checkbox is unchecked",
                    )
                )
    return drifts


def scan_resume(resume_path: Path) -> list[Drift]:
    if not resume_path.exists():
        return [Drift("missing-file", resume_path, "file not found")]
    drifts: list[Drift] = []
    text = resume_path.read_text(encoding="utf-8")

    # Codex active/offline contradiction sweep.
    for idx, line in enumerate(text.splitlines(), start=1):
        for pat in CODEX_OFFLINE_PATTERNS:
            if pat.search(line):
                drifts.append(
                    Drift(
                        "codex-offline-contradiction",
                        resume_path,
                        f"line {idx}: '{line.strip()[:120]}' — "
                        f"contradicts current codex-active banner",
                    )
                )
                break

    # Last sweep staleness.
    match = LAST_SWEEP_RE.search(text)
    if match is None:
        drifts.append(
            Drift(
                "last-sweep-missing",
                resume_path,
                "no `Last sweep: YYYY-MM-DD` line at top of file",
            )
        )
    else:
        try:
            sweep_date = datetime.strptime(match.group(1), "%Y-%m-%d").date()
        except ValueError:
            drifts.append(
                Drift(
                    "last-sweep-malformed",
                    resume_path,
                    f"`Last sweep:` value '{match.group(1)}' is not YYYY-MM-DD",
                )
            )
        else:
            age = (date.today() - sweep_date).days
            if age > LAST_SWEEP_STALE_DAYS:
                drifts.append(
                    Drift(
                        "last-sweep-stale",
                        resume_path,
                        f"last sweep {sweep_date.isoformat()} is {age} days "
                        f"old (>{LAST_SWEEP_STALE_DAYS}d threshold)",
                    )
                )
    return drifts


def main(argv: list[str]) -> int:
    try:
        quiet, v1_path, resume_path = parse_args(argv)
    except ValueError as exc:
        print(f"error: {exc}", file=sys.stderr)
        print(usage(), file=sys.stderr, end="")
        return 2

    drifts: list[Drift] = []
    drifts.extend(scan_v1(v1_path))
    drifts.extend(scan_resume(resume_path))

    counts: dict[str, int] = {}
    for d in drifts:
        counts[d.kind] = counts.get(d.kind, 0) + 1
    summary = ", ".join(f"{k}={v}" for k, v in sorted(counts.items())) or "none"
    print(f"Found {len(drifts)} doc-drift item(s) ({summary})")

    if not quiet:
        for d in drifts:
            print(f"[{d.kind}] {d.file}: {d.detail}")

    return 1 if drifts else 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
