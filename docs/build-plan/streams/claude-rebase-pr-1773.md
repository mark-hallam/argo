# claude-rebase-pr-1773 — recover #1773 codex-fn-loc-followup-wave1 (P2)

**Owner**: claude. **Recovery target**: PR [#1773](https://github.com/mark-hallam/Argo-Agent/pull/1773). **Original brief**: `streams/codex-fn-loc-followup-wave1.md`.
**Authored against**: origin/main `da8e0e0e` on `2026-05-17`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-rebase-pr-1773`.

## Why this brief exists

PR #1773 (split top-5 fn-LOC violators surfaced by F-09) opened 2026-05-16T11:07 and stayed DIRTY ≥30h. Conflict surface is wide: worker_actions.rs, dispatch_post.rs, turn_helpers.rs, argo-tui input.rs — every Rust merge since touches at least one.

## Acceptance: pick ONE recovery path

### Path A — rebase (likely painful given the conflict surface)

1. `git fetch origin && git checkout codex-fn-loc-followup-wave1`
2. `git rebase origin/main`. Expect significant conflict resolution; the type-canon wave #1747 renamed types that some of these large functions reference.
3. Per-extracted-function: confirm split is still <80 LOC after rebase (the violator list on current main is fresh — the top-5 picked by the prior worker may have shifted).
4. Verification: `cargo test`, `cargo clippy`, `bash scripts/check-fn-loc.sh | grep -E '<one of the 5>'` returns 0 matches.
5. `git push --force-with-lease origin codex-fn-loc-followup-wave1`.

### Path B — fresh start (recommended)

1. Re-run `bash scripts/check-fn-loc.sh 2>&1 | grep -v '_tests.rs' | sort -t: -k3 -nr | head -10` against CURRENT main.
2. Pick the **current** top-5 violators (likely different from the prior worker's pick given recent merges).
3. Re-implement on `codex-fn-loc-followup-wave1-v2`.
4. Close #1773 with supersedes comment.

**Path B is recommended** for this PR — the violator list on current main is different from what the prior worker started against, so rebase preserves stale choices.

## Acceptance criteria

- [ ] PR ends MERGEABLE / non-DIRTY against current main.
- [ ] 5 split functions enumerated in PR body (their before/after LOC).
- [ ] `bash scripts/check-fn-loc.sh | grep -E '<one of the 5 file:line>'` returns 0.
- [ ] No behavior change.
- [ ] BUILD_STATUS.md row flipped TODO → IN_REVIEW.

## Hidden constraints

- **Pure mechanical refactor.** No type signatures change. Extract sub-functions only.
- **Don't extract into new modules** — keep helpers private to the same file.
- **`codex-loc-budget-followup-wave2` is in TODO** — that's wave 2. Don't overlap; this brief is wave 1 recovery.
