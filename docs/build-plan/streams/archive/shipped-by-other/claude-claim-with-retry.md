# claude-claim-with-retry — auto-retry on ledger race-loss (small-scope helper)

**Owner**: claude. **Depends on**: nothing. **Phase A — process improvement.**

## Why

Worker tab self-exits report that race-loss on the BUILD_STATUS.md claim commit costs ~10min per occurrence. With ~10 active tabs racing on the ledger, this happens regularly enough to materially degrade throughput. Currently HARD RULE 7 says "pick a different brief" — that's correct for the case where the SAME brief got claimed by another tab, but for the more common case of "another tab's unrelated status update on a different row landed first," forcing a re-claim wastes effort.

This brief adds a tiny retry helper that:
1. Detects "non-fast-forward" specifically (race-loss, recoverable)
2. Auto-retries up to 3 times: pull --rebase, re-add the BUILD_STATUS.md edit, re-push
3. Exits non-zero if retries exhausted (worker falls through to HARD RULE 7 different-brief logic)
4. Distinguishes recoverable race-loss from unrecoverable failures (auth, network)

## Scope

**Small narrow surface** — single bash script + 1-line worker-prompt mention.

## Approach

1. **NEW `scripts/claim-with-retry.sh`** (~80 LOC bash):
   ```bash
   #!/usr/bin/env bash
   # Wrap the claim-commit-push sequence with bounded race-loss retry
   # Usage: scripts/claim-with-retry.sh <brief-id>
   ```
   - Reads BUILD_STATUS.md, edits the brief-id row to CLAIMED
   - Commits, pushes
   - On `non-fast-forward` failure: `git pull --rebase`, re-apply edit, re-push
   - Cap at 3 retries with sleep $((RANDOM % 5 + 1)) jitter
   - If brief is now CLAIMED-by-another (real race-loss): exit non-zero, worker falls through
   - If push fails for other reasons: exit non-zero with the error

2. **Update CLAUDE.md HARD RULE 7** — add line:
   "Or use `scripts/claim-with-retry.sh <brief-id>` which auto-retries non-fast-forward races up to 3× before falling through to this rule."

3. **Worker prompts** — 1-line note suggesting the helper.

## Files you may touch

- `scripts/claim-with-retry.sh` (NEW, ≤ 100 LOC)
- `scripts/claim-with-retry-tests.sh` (NEW, ≤ 100 LOC) — bash smoke tests using a temp git repo
- `CLAUDE.md` HARD RULE 7 — narrowly: 1-line addition
- `docs/build-plan/PROTOCOL_WORKER_PROMPT_CLAUDE.md` — narrowly: 1-line note
- `docs/build-plan/PROTOCOL_WORKER_PROMPT_CODEX.md` — narrowly: 1-line note

## Files you must NOT touch

- BUILD_STATUS.md (no real status change; this is tooling)
- Any Rust crate (pure bash)

## Acceptance criteria

- [ ] `scripts/claim-with-retry.sh <existing-brief-id>` succeeds on a clean main
- [ ] Race-loss simulated via `git push --force` from a sibling clone → script auto-recovers up to 3 retries
- [ ] If brief is genuinely CLAIMED-by-another (verified via `grep CLAIMED ledger`): script exits 1 with clear message
- [ ] Auth failure / network failure: exits 1 with that error surfaced (no infinite retry)
- [ ] CLAUDE.md HARD RULE 7 updated with reference to the script
- [ ] Both worker prompts updated
- [ ] Smoke test bash script passes locally

## Branch + PR

- Branch: `claude-claim-with-retry`
- PR title: `tooling: auto-retry on ledger race-loss (claude-claim-with-retry)`

## Notes

- Conservative bounded retry. If race storm is real (10+ tabs), retries will exhaust → fall through to HARD RULE 7 — no degradation vs today.
- Distinguishing race-loss from other failures: `git push 2>&1 | grep -q 'non-fast-forward'`. Anything else exits without retry.
- Don't extend beyond the claim step. Push-feature-branch + PR-creation should not auto-retry — those are workflow steps where worker decision matters.
- Tested in isolation; pure bash; no Rust crates touched.
