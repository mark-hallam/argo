# Mission load-bearing wire — Sweep stale BUILD_STATUS rows

**Owner**: claude. **Depends on**: none.
**Authored against**: origin/main `1e01c3a7` on `2026-05-13`.

> SOURCE: MISSION_BACKLOG #276. `BUILD_STATUS.md` has accumulated ~50 BLOCKED rows and a long tail of MERGED rows from earlier tracks. Some BLOCKED rows reference dependencies that have since shipped — they're cascade-unblockable. Some MERGED rows from old tracks could be archived to a separate file to keep the live ledger scannable. **Tier-Doc / ledger hygiene.**

## Scope

Three-pass sweep over `docs/build-plan/BUILD_STATUS.md`:

1. **Pass 1: Cascade-unblock**. For every BLOCKED row, parse its dependency citation (most rows use the form `BLOCKED on mission-<X> MERGED`). For each such row, check whether the cited dependency is now MERGED on origin/main. If yes, flip the row from BLOCKED → TODO with a `**Cascade-unblocked 2026-05-13 by sweep (<cited-brief> MERGED)**` annotation. Do NOT modify the may-touch list or scope — that's a separate review.

2. **Pass 2: Detect stale BLOCKED**. For BLOCKED rows whose dependency citation is itself ambiguous, missing, or references something that doesn't exist (e.g. cites `mission-foo-bar` where no such row exists in either MERGED-or-active state), flag with a `**Stale dependency citation 2026-05-13 — needs operator triage**` annotation. Do not auto-promote.

3. **Pass 3: Archive ancient MERGED rows**. For any row in MERGED state with PR number < 1300 (a rough cut-off corresponding to ~tick-60-era work), move the entire row line to a NEW file `docs/build-plan/BUILD_STATUS_ARCHIVE_2026Q2.md`. Preserve row content verbatim. The live ledger header gets a one-line pointer at the top of the historical section: `> Historical rows pre-#1300 archived to BUILD_STATUS_ARCHIVE_2026Q2.md`.

NOT in scope: re-litigating BLOCKED reasons; rewriting may-touch lists; touching the ledger header (the operator amendments live there); editing any TODO / CLAIMED / IN_REVIEW rows; touching streams/*.md briefs.

## Files you may touch

- `docs/build-plan/BUILD_STATUS.md` — three classes of edit (cascade-unblock annotations, stale-flag annotations, ancient-row moves)
- `docs/build-plan/BUILD_STATUS_ARCHIVE_2026Q2.md` (NEW) — archive sink
- Optional: append a one-line sweep summary to the ledger's "Recent housekeeping" subsection (verify it exists; if not, do NOT create one)

## Files you must NOT touch

- The BUILD_STATUS.md **header** (lines 1–~90) — that's operator-amendment territory
- Any TODO / CLAIMED / IN_REVIEW row's status field — only BLOCKED → TODO transitions are permitted
- Any may-touch or scope text inside a row — sweep is metadata-only
- Any `streams/*.md` brief
- Any code

## Acceptance criteria

- [ ] Every BLOCKED row whose dependency citation resolves to a now-MERGED row is flipped to TODO with a cascade-unblock annotation
- [ ] Every BLOCKED row whose dependency citation is ambiguous is flagged for triage
- [ ] Every MERGED row with PR <1300 is moved to `BUILD_STATUS_ARCHIVE_2026Q2.md`
- [ ] Live ledger LOC drops measurably (rough target: <80% of pre-sweep size after archive move)
- [ ] Archive file is self-contained — readable without the live ledger
- [ ] Zero edits to TODO / CLAIMED / IN_REVIEW rows
- [ ] Zero edits to header
- [ ] BUILD_STATUS.md row for this brief updated

## Branch + PR

- Branch: `claude-mission-sweep-stale-build-status-rows`
- PR title: `docs: sweep BUILD_STATUS (cascade-unblock + archive pre-#1300 MERGED rows)`

## Notes

- Use a script in `/tmp` to drive the diff (don't commit it). Hand-editing 50+ rows is error-prone.
- Cascade-unblock heuristic: most rows cite dependencies in the form `BLOCKED on \`mission-<slug>\`` or `BLOCKED by claude/<rowname>`. Parse permissively. False positives are worse than false negatives — when in doubt, flag for triage rather than auto-unblock.
- The PR <1300 cutoff is deliberately mechanical; the operator can adjust the cutoff in a follow-up if they want a different archive window.
- Worker should expect a **large** diff file count of 1 (`BUILD_STATUS.md`) plus 1 new file. The line count moved into the archive could be 100s of lines.
- After the PR merges, fresh agents reading RESUME.md → BUILD_STATUS.md will face a meaningfully shorter ledger. That's the win.
