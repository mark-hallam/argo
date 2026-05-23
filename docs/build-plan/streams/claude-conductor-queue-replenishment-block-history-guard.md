# claude-conductor-queue-replenishment-block-history-guard

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-conductor-queue-replenishment-block-history-guard`.

🔴 **P0 CONDUCTOR PROCESS — Step 8 queue replenishment re-authors stale-BLOCKED briefs.**

Source: claude worker session end report 2026-05-23 (operator-relayed). The conductor's tick-92 +30-brief batch re-authored `claude-mcp-connector-empty-dir-cleanup` to TODO despite a prior BLOCKED commit (`7418b5045`) earlier the same day. Two independent claude workers then wasted iterations re-discovering the same impossible premise (the 7 dirs were deleted in `91d7d5c66` on 2026-05-12; operator's local checkout sees them only because of `.DS_Store` artifacts).

This is a real conductor-process regression. Step 8 has no guard against re-authoring brief-IDs that have been BLOCKED previously this session.

## Scope

Add a guard to Step 8 (queue replenishment) and to the brief-authoring helper that conductor uses:

1. **Pre-author check** — before writing a new brief stub to `docs/build-plan/streams/`:
   ```bash
   git log --all --oneline --grep="<brief-id>" docs/build-plan/BUILD_STATUS.md | head -5
   ```
   If ANY prior commit message contains `<brief-id> BLOCKED` OR `BLOCKED ... <brief-id>`, refuse to author. Print a loud warning + cite the BLOCKING commit.

2. **Pre-insert check** — before inserting a TODO row for `<brief-id>` into BUILD_STATUS.md:
   ```bash
   git log --all --oneline -S"<brief-id>" docs/build-plan/BUILD_STATUS.md | grep -i "blocked" | head -3
   ```
   If hits found, refuse insert + surface to operator.

3. **Wrap in a sidecar script** — `scripts/check-brief-block-history.sh <brief-id>`:
   - Exit 0: no prior BLOCK in git history for this brief-id
   - Exit 1: prior BLOCK found, print commit SHAs + dates
   - Exit 2: ledger has a current non-TODO row for this brief-id

4. **Document in PROTOCOL_LOOP_PROMPT.md Step 8**: "Before authoring or re-authoring a brief stub, run `bash scripts/check-brief-block-history.sh <brief-id>`. If exit != 0, choose a different brief — do not re-author."

## Parallelism

- Manager-worker allowed: no
- Max children: 0

## Files you may touch

- NEW: `scripts/check-brief-block-history.sh` (~50 LOC bash)
- NEW: `scripts/check-brief-block-history-tests.sh` (~80 LOC smoke tests)
- `docs/build-plan/PROTOCOL_LOOP_PROMPT.md` — narrowly scoped: amend Step 8 with the guard
- `docs/build-plan/BUILD_STATUS.md` — standard row flip

## Files you must NOT touch

- The existing 30+ TODO briefs (just gate FUTURE inserts).
- Other Step 8 logic.

## Acceptance criteria

- [ ] `scripts/check-brief-block-history.sh <brief-id>` exists, executable
- [ ] Exit codes: 0 clean / 1 prior-BLOCK / 2 currently-active-non-TODO
- [ ] Smoke test verifies the script catches `claude-mcp-connector-empty-dir-cleanup` (known prior-BLOCKED brief)
- [ ] PROTOCOL_LOOP_PROMPT.md Step 8 instructs the conductor to run this before any brief insert
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-conductor-queue-replenishment-block-history-guard`
- PR title: `ops(conductor): guard Step 8 against re-authoring stale-BLOCKED briefs (claude-conductor-queue-replenishment-block-history-guard)`

## Notes for the implementer

- ~100-200 LOC total. Script + tests + protocol amendment.
- This is the conductor protecting itself from its own past mistakes. The signal cost (2+ workers wasting an iteration each) is high enough to warrant the mechanical check.
- Cite the worker session-end report at SHARED.md timestamp 2026-05-23 (operator-relayed) + the original BLOCKING commit `7418b5045` + the re-author commit (the tick-92 batch — find via `git log --grep="replenish queue +30"`) verbatim in PR body.
