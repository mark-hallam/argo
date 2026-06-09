#!/usr/bin/env python3
"""capture-recurring-nits.py — DEC-042 Stage 1 B-3 recurring-nit proposer.

The *compounding* build-loop step: turn repeated boundary-notes into captured
rules so the defect/nit rate trends DOWN (Boris's "3-4 occurrences -> lint
rule" habit, DEC-042 §1.2 / §3 B-3). Today a mistake becomes a *repeated*
ledger boundary-note instead — e.g. the `await_holding_lock` /
`routes_settings_credentials` clippy blocker is boundary-noted 40+ times.

PROPOSER ONLY (DEC-042 §7 + DEC-034 operator-as-verifier): NEVER auto-applies.
It does NOT touch CLAUDE.md, the DEC register, or .github/CI — operator-judgment
surfaces. `--draft` writes DRAFT `streams/capture-<slug>.md` briefs marked
awaiting-approval (NOT queued TODO); a human wires any actual rule.

Mines: (a) BUILD_STATUS.md note cells, (b) the last N merged-PR commit bodies on
origin/main (best-effort; skipped with --no-git). Normalization: for each nit
marker (pre-existing / boundary-note / known-flake / flake) it reads a window
around the marker, extracts snake_case tokens, strips a trailing `_test(s)`, and
drops process/infra vocabulary. A note counts once per distinct token. Patterns
with count >= THRESHOLD (default 3) that are NOT already captured (no existing
`capture-<slug>` brief, not in KNOWN_FLAKES.md) become findings.

Usage: `--quiet` prints only the count (the recurring_nit_count metric);
`--draft` writes the DRAFT briefs. Exit status: always 0 (advisory).
"""
from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from collections import Counter, defaultdict
from datetime import date
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
DEFAULT_LEDGER = REPO / "docs" / "build-plan" / "BUILD_STATUS.md"
DEFAULT_STREAMS = REPO / "docs" / "build-plan" / "streams"
DEFAULT_KNOWN_FLAKES = REPO / "docs" / "build-plan" / "KNOWN_FLAKES.md"

THRESHOLD_DEFAULT = 3
PR_BODY_COUNT = 40
WINDOW = 90
MIN_TOKEN_LEN = 5

NIT_MARKER = re.compile(r"pre-existing|boundary.?note|known.?flake|\bflake\b", re.IGNORECASE)
TOKEN = re.compile(r"[a-z][a-z0-9]*(?:_[a-z0-9]+)+")
TRAILING_TEST = re.compile(r"_tests?$")

# Process / infrastructure vocabulary that shows up *inside* nit notes but is
# not itself a recurring nit. Kept deliberately small: real test/file/lint
# identifiers must NOT be suppressed (those are the nits we want to surface).
STOPWORDS = {
    "in_review", "claude_todo", "codex_todo", "tenant_id", "worker_id",
    "known_flakes", "cargo_target_dir", "mission_backlog", "schema_version",
    "expected_schema_version", "feature_matrix", "build_status",
    "anthropic_api_key", "llm_input_tokens", "claim_with_retry", "may_touch",
    "origin_main", "cross_tenant", "git_pull", "skycot_db_key_base64",
}


def _run(cmd: list[str], timeout: int = 20) -> str | None:
    """Run a command, return stripped stdout, or None on any failure."""
    try:
        out = subprocess.run(
            cmd, cwd=REPO, capture_output=True, text=True, timeout=timeout
        )
    except (subprocess.SubprocessError, OSError):
        return None
    if out.returncode != 0 and not out.stdout.strip():
        return None
    return out.stdout.strip()


def canonical(token: str) -> str:
    """Collapse `_test`/`_tests` suffix so foo_tests and foo are one nit."""
    return TRAILING_TEST.sub("", token)


def slug_for(pattern: str) -> str:
    """`routes_settings_credentials` -> `capture-routes-settings-credentials`."""
    return "capture-" + pattern.replace("_", "-")


def extract_tokens(note: str) -> set[str]:
    """Distinctive canonical nit-tokens from windows around each nit marker.

    Localizing to a window keeps a long PR note's unrelated feature symbols out
    of the tally — only tokens near the "pre-existing"/"boundary-note" phrasing
    count as the nit.
    """
    low = note.lower()
    found: set[str] = set()
    for marker in NIT_MARKER.finditer(low):
        start = max(0, marker.start() - 10)
        end = min(len(low), marker.end() + WINDOW)
        # Don't cut a token in half at the window edge (avoids phantom tokens).
        tail = re.match(r"\w*", low[end:])
        if tail:
            end += tail.end()
        for raw in TOKEN.findall(low[start:end]):
            tok = canonical(raw)
            if len(tok) >= MIN_TOKEN_LEN and tok not in STOPWORDS:
                found.add(tok)
    return found


def ledger_notes(ledger: Path) -> list[str]:
    """Note cells of real (non-header) BUILD_STATUS.md rows."""
    notes: list[str] = []
    try:
        text = ledger.read_text(encoding="utf-8")
    except OSError:
        return notes
    for line in text.splitlines():
        if not line.startswith("| "):
            continue
        cells = line.split("|")
        if len(cells) < 7:
            continue
        brief = cells[1].strip()
        if not brief or brief.startswith("**"):
            continue
        notes.append("|".join(cells[5:-1]))
    return notes


def pr_body_notes(count: int) -> list[str]:
    """Last `count` merged-PR commit bodies on origin/main (best-effort)."""
    raw = _run(["git", "log", f"-n{count}", "--format=%B%x1e", "origin/main"])
    if not raw:
        return []
    return [chunk for chunk in raw.split("\x1e") if chunk.strip()]


def already_captured(pattern: str, streams: Path, known_text: str) -> bool:
    """A pattern is captured if a capture brief exists or it's a known flake."""
    if (streams / f"{slug_for(pattern)}.md").exists():
        return True
    return pattern in known_text


def collect_findings(
    notes: list[str], threshold: int, streams: Path, known_text: str
) -> list[dict]:
    counts: Counter[str] = Counter()
    examples: dict[str, list[str]] = defaultdict(list)
    for note in notes:
        for tok in extract_tokens(note):
            counts[tok] += 1
            if len(examples[tok]) < 3:
                examples[tok].append(" ".join(note.split())[:160])
    findings: list[dict] = []
    for pattern, count in counts.most_common():
        if count < threshold:
            continue
        if already_captured(pattern, streams, known_text):
            continue
        findings.append(
            {
                "pattern": pattern,
                "count": count,
                "slug": slug_for(pattern),
                "examples": examples[pattern],
                "proposed_capture": (
                    f"'{pattern}' boundary-noted {count}x without being captured. "
                    "Fix once OR add a scoped lint allow / KNOWN_FLAKES.md entry; "
                    "human decides, no auto-edit of CLAUDE.md / DEC / CI."
                ),
            }
        )
    return findings


def draft_body(finding: dict, threshold: int, sha: str, today: str) -> str:
    slug = finding["slug"]
    pattern = finding["pattern"]
    bullets = "\n".join(f"- {ex}" for ex in finding["examples"]) or "- (none captured)"
    return f"""# {slug}

**Owner**: unassigned (DRAFT — awaiting conductor+operator approval).
**Status**: DRAFT — auto-proposed by `scripts/capture-recurring-nits.py` (DEC-042 B-3). NOT queued as TODO.
**Authored against**: origin/main `{sha}` on `{today}`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh {slug}`.

> AUTO-PROPOSED, NOT AUTO-APPLIED. The B-3 capturer drafted this because the nit
> below was boundary-noted **{finding['count']}x** without ever being captured.
> A human decides: (a) fix once, (b) scoped lint allow, or (c) KNOWN_FLAKES.md
> entry. Do NOT auto-edit CLAUDE.md / DEC register / .github-CI from this brief —
> those are operator-judgment surfaces (DEC-042 §7, DEC-034).

## Why
`{pattern}` has been boundary-noted/flagged **{finding['count']}x** across
`BUILD_STATUS.md` + recent merged-PR bodies (threshold {threshold}). The same
nit is re-described on every PR instead of being fixed/allow-listed once — wasted
signal (DEC-042 §3 B-3, the compounding step).

Example notes:
{bullets}

## Proposed capture (pick ONE — human decides)
- **Fix once** (preferred when cheap): resolve the underlying issue so it stops recurring.
- **Allow-list**: add a scoped lint allow OR one canonical `KNOWN_FLAKES.md` entry so workers stop re-noting it.
- **Lint rule** (candidate; a human wires it to `.github`/CI — this brief must not): if it is a *class* of mistake, name the candidate lint here.

## Acceptance
- No fresh boundary-note for this nit on new PRs, OR it has one canonical `KNOWN_FLAKES.md` entry / lint allow.
- `scripts/capture-recurring-nits.py` no longer emits `{pattern}`.

## Effort
S (single-nit cleanup, or one allow-list / KNOWN_FLAKES entry).
"""


def write_drafts(findings: list[dict], threshold: int, streams: Path) -> list[str]:
    streams.mkdir(parents=True, exist_ok=True)
    sha = _run(["git", "rev-parse", "--short", "HEAD"]) or "unknown"
    today = date.today().isoformat()
    written: list[str] = []
    for finding in findings:
        path = streams / f"{finding['slug']}.md"
        if path.exists():
            continue
        path.write_text(draft_body(finding, threshold, sha, today), encoding="utf-8")
        try:
            written.append(str(path.relative_to(REPO)))
        except ValueError:
            written.append(str(path))
    return written


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description="Propose captures for recurring nits (DEC-042 B-3).")
    ap.add_argument("--threshold", type=int, default=THRESHOLD_DEFAULT, help="min occurrences to surface a pattern")
    ap.add_argument("--quiet", action="store_true", help="print only the count (the recurring_nit_count metric)")
    ap.add_argument("--draft", action="store_true", help="write DRAFT capture-<slug>.md briefs (NOT queued TODO)")
    ap.add_argument("--no-git", action="store_true", help="skip PR-body mining (deterministic; ledger only)")
    ap.add_argument("--ledger", type=Path, default=DEFAULT_LEDGER)
    ap.add_argument("--streams-dir", type=Path, default=DEFAULT_STREAMS)
    ap.add_argument("--known-flakes", type=Path, default=DEFAULT_KNOWN_FLAKES)
    ap.add_argument("--pr-bodies", type=int, default=PR_BODY_COUNT, help="how many recent merged-PR bodies to mine")
    args = ap.parse_args(argv)

    notes = ledger_notes(args.ledger)
    if not args.no_git:
        notes += pr_body_notes(args.pr_bodies)

    try:
        known_text = args.known_flakes.read_text(encoding="utf-8").lower()
    except OSError:
        known_text = ""

    findings = collect_findings(notes, args.threshold, args.streams_dir, known_text)

    written: list[str] = []
    if args.draft:
        written = write_drafts(findings, args.threshold, args.streams_dir)

    if args.quiet:
        print(len(findings))
        return 0

    payload = {
        "_schema": "dec042-b3-recurring-nits-v1",
        "_doc": "docs/skycot/decisions/DEC-042-continuous-self-improvement-loop.md#3",
        "_note": "PROPOSER ONLY — drafts capture briefs; never auto-applies to CLAUDE.md/DEC/CI.",
        "threshold": args.threshold,
        "recurring_nit_count": len(findings),
        "findings": findings,
        "drafts_written": written,
    }
    print(json.dumps(payload, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
