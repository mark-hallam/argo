# fix-pr-2291-rebase-demo-seed-cli-against-current-main

**Owner**: claude or codex.
**Authored against**: origin/main `08920d212` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-pr-2291-rebase-demo-seed-cli-against-current-main`.
**Blocks**: `mission-demo-seed-data-bootstrap` (PR #2291) — currently CONFLICTING.

🟡 **P1 OPS — rebase PR #2291 (demo-seed CLI / mission-demo-seed-data-bootstrap) against current main.**

PR #2291 (1203+/-4 across 18 files, the full demo-seed CLI handler + crate) went CONFLICTING during the T09-30/T10-00/T10-15 merge train. Likely conflict surfaces: `web/lib/sample-personas.ts` (touched by #2284 demo-roster-Peta merge) + the `mission-demo-seed-data-bootstrap.md` brief itself (the re-author under function-vs-dressing at b1ff75b73).

## Scope

`gh pr checkout 2291 && git fetch origin main && git rebase origin/main`. Likely conflict surfaces (verify with `git status --porcelain` post-rebase-start):

1. **`web/lib/sample-personas.ts`** — if #2284 added Peta and #2291 also did, dedupe; keep #2284's version (it's the canonical roster source per memory `project_pm_cohorts_peta_patrick`).
2. **`docs/build-plan/streams/mission-demo-seed-data-bootstrap.md`** — keep the function-vs-dressing-relaxed version (Pass A 10-20 LLM turns + Pass B 100-200 fixture rows). Memory: `feedback_no_theater_in_demo`.
3. **Other**: any cargo / Cargo.lock conflicts, resolve by taking HEAD.

Verify: `cargo check -p argo-cli -p argo-demo-seed` clean. Then `git push --force-with-lease origin claude-mission-demo-seed-data-bootstrap`.

Conductor will then evaluator-coverage-request the PR (it's substantial runtime work) before admin-merge.

## Acceptance criteria

- [ ] PR #2291 is MERGEABLE per `gh pr view 2291 --json mergeable`.
- [ ] `web/lib/sample-personas.ts` has Peta exactly once (deduped if needed).
- [ ] `mission-demo-seed-data-bootstrap.md` retains its Pass A / Pass B function-vs-dressing structure.
- [ ] `cargo check -p argo-cli -p argo-demo-seed --tests` clean.
- [ ] PR body has the rebase command output + conflict resolution notes.

## May-touch

- `web/lib/sample-personas.ts`
- `docs/build-plan/streams/mission-demo-seed-data-bootstrap.md`
- `crates/argo-cli/Cargo.toml` + `Cargo.lock` (if needed for rebase resolution)
- This brief file (status updates)

## Out of scope

- Extending the seed CLI's scope (consume current shape)
- Changing the Pass A / Pass B counts (already operator-approved in the function-vs-dressing reframe)

## Effort

~20-40 min — rebase + 1-2 file conflict resolution + cargo verify.
