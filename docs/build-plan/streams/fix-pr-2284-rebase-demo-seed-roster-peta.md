# fix-pr-2284-rebase-demo-seed-roster-peta

**Owner**: claude or codex.
**Authored against**: origin/main `2bb33b002` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-pr-2284-rebase-demo-seed-roster-peta`.
**Blocks**: `mission-demo-seed-data-bootstrap` (CLAIMED, in progress; this rebase must land before bootstrap can ship clean).

🟡 **P1 OPS — rebase PR #2284 against current main to clear the merge conflict.**

PR #2284 (`demo: canonize seed worker roster with Peta`) was opened at T09-15 by a worker who claimed `mission-demo-seed-worker-roster-canon-peta-included`. During the 15-PR T09-30 merge train, the PR conflicted against #2282 (P-02 brief refresh) and / or `mission-demo-seed-data-bootstrap` brief re-author at b1ff75b73 (function-vs-dressing relax). Conductor T09-30 admin-merge attempt failed with `GraphQL: Pull Request has merge conflicts (mergePullRequest)`.

The author needs to rebase. This brief lets a fresh worker take it over if the original author doesn't respond quickly.

## Scope

1. `git fetch origin main && git checkout claude-mission-demo-seed-worker-roster-canon-peta-included` (the PR #2284 branch name).
2. `git rebase origin/main`.
3. Resolve conflicts. Likely surfaces:
   - `docs/build-plan/streams/mission-demo-seed-data-bootstrap.md` — re-authored at `b1ff75b73` under the function-vs-dressing rule (Pass A / Pass B split, 5 workers not 6, see brief content). Keep the re-authored version's Pass-A/B split + canon roster section.
   - `web/lib/sample-personas.ts` — if both #2284 and the function-vs-dressing relax added Peta, dedupe.
   - `docs/build-plan/BUILD_STATUS.md` — likely diverged ledger rows; keep the post-T09-30-merge versions (auto-status commits land workers' state).
4. `git push --force-with-lease origin claude-mission-demo-seed-worker-roster-canon-peta-included`.
5. Re-request admin-merge from conductor (open a comment on the PR saying "rebased + force-pushed; ready for re-merge").
6. Update the ledger row note for `mission-demo-seed-worker-roster-canon-peta-included` to reflect the rebase.

## Acceptance criteria

- [ ] PR #2284 is MERGEABLE (no conflicts) per `gh pr view 2284 --json mergeable`.
- [ ] The Peta persona is present in `web/lib/sample-personas.ts` exactly once (deduped if needed).
- [ ] `mission-demo-seed-data-bootstrap.md` retains its Pass A / Pass B re-author + canon roster section.
- [ ] PR body has the rebase command + conflict resolution note.

## May-touch

- `web/lib/sample-personas.ts`
- `docs/build-plan/streams/mission-demo-seed-data-bootstrap.md`
- `docs/build-plan/BUILD_STATUS.md` (narrow: ledger row note refresh)
- This brief file (status updates)

## Out of scope

- Extending Peta's persona content (the original PR's content is the load-bearing artifact; just unstick it)
- Adding Patrick (separate decision per the original brief's Option A/B question)

## Effort

~15-30 min. Pure rebase + light conflict resolution.
