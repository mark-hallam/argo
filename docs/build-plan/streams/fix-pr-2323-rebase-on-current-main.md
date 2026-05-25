# fix-pr-2323-rebase-on-current-main — PR #2323 gateway-mount Phase-A endpoints is 87 commits behind main; rebase + verify

**Owner**: claude or codex. **Depends on**: nothing.
**Authored against**: origin/main `24cb565b8` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-pr-2323-rebase-on-current-main`.

## Why this is P1 (5th-round merge stall — evaluator round-65/67/68/69)

PR #2323 (`fix-gateway-workflows-and-skills-marketplace-endpoints-404` / `Phase A — gateway — mount /api/v1/skills + /api/v1/workflows top-level list endpoints`) has been stuck for 4-5 evaluator rounds. The PR mounts the missing top-level list endpoints that the SPA needs to render `/skills` and `/workflows`. Source: `docs/skycot/audit/scheduled/LATEST.md` recommends shipping it; round-65/67/68/69 evaluator reports name it as a merge-stall.

T18-08 conductor flagged: "branch is 87 commits behind main, may need rebase brief — `git diff origin/main` shows 117 files / -7872 lines suggesting branch staleness". Today's check confirms PR is MERGEABLE on GitHub's mergeability indicator, but the diff is suspect because of the long staleness gap. Round-69 evaluator recommends: "after gh recomputes mergeability, MERGE under spirit-of-grant OR author fix-pr-2323-rebase-on-current-main brief if 87-commit drift broke it".

This brief is that rebase. Output: a fresh PR head that is current-main + ONLY the gateway-mount changes — no accidentally-reverted commits from the staleness gap.

## Scope

1. `git fetch origin && git checkout claude-fix-gateway-workflows-and-skills-marketplace-endpoints-404` (the PR #2323 branch).
2. Inspect `git log origin/main..HEAD --oneline` — should be a small number of commits authoring the gateway-mount changes. If many commits, identify the intent commits and discard merge artifacts.
3. `git rebase origin/main`. Resolve any conflicts preserving the gateway-mount intent.
4. Verify the resulting diff vs `origin/main` is small and exclusively touches gateway-mount files (likely `crates/argo-cli/src/handlers/gateway/routes_skills.rs`, `crates/argo-cli/src/handlers/gateway/routes_workflows.rs`, `crates/argo-cli/src/handlers/gateway/mod.rs` or similar — confirm against the PR body).
5. If the rebase yields a 117-file / -7872-line diff (the staleness symptom), the branch is bricked — instead, branch from current main, cherry-pick the intent commits manually, force-push, and update the PR description noting the recovery path.
6. Run crate-scoped tests: `cargo test -p argo-cli --test gateway_routes_*` (or whatever the relevant integration test is) + `cargo clippy -p argo-cli --all-targets -- -D warnings`.
7. Force-push (`git push --force-with-lease`) to update the PR.

**Not in scope**: adding new endpoints, changing endpoint behavior, touching SPA code.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- Whatever the original PR #2323 touches in `crates/argo-cli/src/handlers/gateway/`. Verify against `gh pr diff 2323 --name-only` before editing.

## Files you must NOT touch

- Files outside the original PR #2323's scope. The post-rebase diff must be a strict subset of what #2323 originally proposed.

## Acceptance criteria

- [ ] PR #2323 head is fresh on top of current `origin/main`.
- [ ] `git diff origin/main..HEAD --name-only` shows ≤10 files (vs the 117 staleness symptom).
- [ ] All touched files lie under `crates/argo-cli/src/handlers/gateway/` (or other gateway-route paths).
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.
- [ ] PR mergeStateStatus returns CLEAN after force-push and GH recomputes.
- [ ] BUILD_STATUS.md row for the original brief updated with rebase note.

## Branch + PR

- Branch: same as PR #2323's head — `claude-fix-gateway-workflows-and-skills-marketplace-endpoints-404` (do not create a new branch; rebase the existing one).
- PR title: unchanged.

## Notes for the implementer

- Check `gh pr view 2323 --json commits,files,additions,deletions` BEFORE rebasing to understand the original intent surface. If the PR's `additions+deletions` is small (~100-300 lines) but `git diff origin/main..HEAD` is 7872 lines, the staleness gap absorbed unrelated reverts — that's the bricked-branch case (Scope step 5).
- This is the `fix-pr-22*-rebase*` pattern explicitly allowed under the overnight `--skip-clippy` authorization (operator memory `feedback_overnight_genesis_track_p_merge_authorization_2026_05_25`, window expires 2026-05-26T22:00Z AEST).
- Fast-mode candidate. ~30-60 min wall clock.
- Reference round-69 eval report for the merge-stall history; this is the 4th-round (or 5th counting initial r65) carry.
