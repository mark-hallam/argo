# ops-conductor-tick-stale-in-review-demote-after-2h — auto-demote stale IN_REVIEW rows

**Owner**: claude or codex. **Depends on**: nothing.
**Authored against**: origin/main `289d3077f` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh ops-conductor-tick-stale-in-review-demote-after-2h`.

## Why this is P2

Conductor protocol Step 5c says "demote stale IN_REVIEW rows where PR has been DIRTY >2h since last fix-request" but this is enforced by-hand each tick. The current ledger has 38 IN_REVIEW rows — many of those are zombie state (PR was merged but row never flipped, or PR was closed but row was left IN_REVIEW). A scripted check would catch them deterministically.

## Scope

1. Write `scripts/check-stale-in-review-rows.py` (or .sh) that:
   - Walks every `IN_REVIEW` row in `docs/build-plan/BUILD_STATUS.md`.
   - For each, extracts the PR number (`| #NNNN |` column).
   - Runs `gh pr view NNNN --json mergeable,state,closedAt,mergedAt` per PR.
   - Reports any row where: (a) PR is CLOSED/MERGED but ledger says IN_REVIEW (ledger drift), or (b) PR is OPEN + DIRTY (mergeable=CONFLICTING) for >2h since last commit (stale).
2. Output a table of stale rows + a recommended action per row (`flip-to-MERGED`, `demote-to-TODO`, `escalate-to-operator`).
3. Wire it into the conductor tick prompt as Step 5c.
4. Run on current ledger; if drift exists, file the appropriate fixes inline.

## Files you may touch

- `scripts/check-stale-in-review-rows.py` (or .sh) — new.
- `docs/build-plan/PROTOCOL_LOOP_PROMPT.md` — wire in Step 5c.

## Files you must NOT touch

- BUILD_STATUS.md row contents (the script reports, doesn't mutate — at least in this brief's scope).
- Anything under `crates/`.

## Acceptance criteria

- [ ] Script runs on current `BUILD_STATUS.md` and produces a recommendation table.
- [ ] Step 5c reference added to PROTOCOL_LOOP_PROMPT.md.
- [ ] If any drift rows found, they're documented in the PR body (one per row).
- [ ] Diff is scripts + docs only.

## Branch + PR

- Branch: `claude-ops-conductor-tick-stale-in-review-demote-after-2h` or `codex-…`.
- PR title: `ops(conductor): detect stale IN_REVIEW rows automatically (ops-conductor-tick-stale-in-review-demote-after-2h)`.

## Notes

- ~80-150 LOC bash or Python.
- Authorized under `--skip-clippy` per evaluator-pre-verified ops-* path.
