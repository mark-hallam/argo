# claude-stale-claim-sweep-tool — operator tool for finding stale-claim ledger drift

**Owner**: claude. **Depends on**: nothing.
**Authored against**: origin/main `abc8c2ff` on `2026-05-13`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-stale-claim-sweep-tool`.

## Scope

Twice in the 2026-05-13 session, conductor ticks found rows whose status column read `CLAIMED` or `IN_REVIEW` but whose note column contained a `STALE-ROAR-INVALIDATION` or `STALE-CLAIM-DEMOTE` directive saying "Returning to TODO". The protocol's annotation pattern relies on humans (or future-conductor) to flip the state column; nothing currently catches the mismatch. The 3-day-stale codex doc briefs (`claude-error-dashboard-runbook` + `claude-troubleshooting-guide-expand`) are the documented case.

Ship a small operator tool — `scripts/find-stale-claim-drift.py` (or `.sh`) — that scans `docs/build-plan/BUILD_STATUS.md` and reports rows where:

- The state column is `CLAIMED` or `IN_REVIEW`, **but**
- The note contains a `STALE-ROAR-INVALIDATION`, `STALE-CLAIM-DEMOTE`, `STALE-SWEEP`, or similar directive that says "Returning to TODO", "stale, demote", "no remote-branch push" past a stated threshold, etc.

The tool runs read-only — prints a per-row mismatch summary the operator (or conductor) can use to author a one-line ledger-flip commit. **Does NOT mutate the ledger.**

**Not in scope**: actually editing the ledger (always operator decision); auto-detecting general stale claims (the existing 2h `STALE-SWEEP` protocol covers IN_REVIEW; this tool is specifically for the annotation-vs-state drift).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `<NEW: scripts/find-stale-claim-drift.py>` (≤200 LOC) — Python preferred (no compile, no Cargo dep). Reads `docs/build-plan/BUILD_STATUS.md`, walks every `| brief-id | owner | state | pr | note |` row, regex-matches the STALE markers in the note, reports.
- `<NEW: scripts/find-stale-claim-drift-tests.sh>` (optional, ≤80 LOC) — shell-driven snapshot test with 2-3 fixture rows.

## Files you must NOT touch

- `docs/build-plan/BUILD_STATUS.md` — the tool reads it; never writes.
- Any other script in `scripts/` — additive only.
- Anything in `crates/`.

## Acceptance criteria

- [ ] `scripts/find-stale-claim-drift.py` runs against the live ledger and:
  - Prints a header summary: `Found N rows with stale-claim drift`.
  - For each match, prints: `<brief-id> | <state> | <stale-marker-text>` (one line, plus the actual offending note clipped to ~150 chars).
  - Exits 0 if no drift found; exits 1 if drift detected (cron-friendly).
- [ ] Regex covers at least these 4 markers: `STALE-ROAR-INVALIDATION`, `STALE-CLAIM-DEMOTE`, `STALE-SWEEP`, `no remote-branch push in`.
- [ ] False-positive guard: only fire if the note ALSO contains a "Returning to TODO" / "demote" / "release" verb. A row whose note merely *mentions* "stale" in passing should not flag.
- [ ] Documentation: `scripts/find-stale-claim-drift.py --help` prints what it does, exit codes, and a one-line cron example.
- [ ] No external deps. Stdlib only.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-stale-claim-sweep-tool`
- PR title: `scripts: find-stale-claim-drift.py — ledger annotation/state mismatch finder (claude-stale-claim-sweep-tool)`

## Notes for the implementer

- Pure Python stdlib; no `argparse` overkill — a 2-flag CLI (`--quiet`, `--limit N`) is fine.
- Snapshot-test the regex with at least 3 fixture cases: a real CLAIMED row with a STALE-CLAIM-DEMOTE note (true positive), a CLAIMED row with no stale marker (true negative), a row whose note mentions "stale" casually without a directive (false positive guard).
- Output is text, not JSON — operator reads it interactively. If JSON becomes useful later, that's a follow-up.
- This tool is itself the canonical implementation of "what is stale-claim drift"; if the conductor protocol evolves the markers, this tool's regex list is where to update.
- Operator authorized 2026-05-13 in conductor briefing (recommended a ~50-LOC python tool; this brief budgets up to 200 LOC for tests + docstring + sensible argparse).
