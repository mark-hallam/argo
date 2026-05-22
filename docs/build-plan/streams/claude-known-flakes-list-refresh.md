# claude-known-flakes-list-refresh

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `22af242b0` on `2026-05-22`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-known-flakes-list-refresh`.

🟢 **P3 SIGNAL HYGIENE.** Source: pattern observation across ticks 38-42 — every recent PR body cites `KNOWN_FLAKES.md` as the reference for which test failures to ignore, but the file's reality model has drifted. Several PRs cited new pre-existing flakes (`a11y-contrast`, `brand-execution-polish`, `onboarding/wizard > Welcome step renders the headline`, `worker/Trust` missing-mock) that aren't in `KNOWN_FLAKES.md` yet. Refresh the file against actual current-main test runs so future PR-body "HARD RULE 2" claims are easy to verify.

## Scope

The brief delivers:
1. A fresh run of `pnpm test --run` (web) and `cargo test --workspace` (Rust) on clean `origin/main`.
2. A diff between the failures observed and the current `KNOWN_FLAKES.md` content.
3. An updated `KNOWN_FLAKES.md` that lists:
   - Each flake's test id (qualified name).
   - The exact failure message (one line).
   - When first observed (`git blame` the test file or grep PR bodies for the first mention).
   - Whether it's a flake (sometimes passes) or a stable pre-existing failure (always fails on clean main).
4. A short "How to verify a flake claim" recipe at the top of the file.

This is doc-and-data only. **NO test or production code touches.** If a flake should be FIXED, that's a separate sibling brief; this brief is the inventory.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `docs/build-plan/KNOWN_FLAKES.md` — replace contents with the refreshed inventory.

## Files you must NOT touch

- Any test file. If a flake needs fixing, open a sibling brief.
- Any production code.
- `BUILD_STATUS.md` outside the standard row flip on this brief's own row.

## Acceptance criteria

- [ ] `KNOWN_FLAKES.md` regenerated from a fresh run on current `origin/main`.
- [ ] Every flake entry has: test id, failure message (1 line), first-observed (date or PR #), classification (flake vs stable).
- [ ] "How to verify a flake claim" section at top with the exact `cargo test --workspace 2>&1 | grep FAIL` (or pnpm equivalent) recipe a PR author should run to confirm pre-existing.
- [ ] PR body lists every flake that was ADDED to the file + every flake that was REMOVED (no longer reproducing).
- [ ] `cargo test --workspace` ran in CI for the inventory; the run log (or summary) is attached to the PR body.
- [ ] No test or production code changes.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-known-flakes-list-refresh`
- PR title: `docs: refresh KNOWN_FLAKES.md against current main (claude-known-flakes-list-refresh)`

## Notes for the implementer

- ~80-150 LOC expected (mostly markdown).
- Running the full test suite cold could take 20-30 min on a fresh worktree. Warm cache helps. Document the elapsed time in the PR body so the next refresh knows what to budget.
- Don't try to FIX flakes here. The point is honest inventory, not heroic cleanup.
- For Rust flakes: include each test's crate (`-p argo-cli`) so reviewers can re-run targeted.
- For web/vitest flakes: include the test file path.
- If a flake disappears between this audit and previous mentions in PR bodies, mark it RESOLVED with the merge commit that likely fixed it (or just "no longer reproduces on `<SHA>`").
