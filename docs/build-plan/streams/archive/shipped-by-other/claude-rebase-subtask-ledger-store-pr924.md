# Split tests + rebase claude-subtask-ledger-store PR #924

**Owner**: claude. **Depends on**: nothing (high-priority recovery task).

## Scope

PR #924 (`claude-subtask-ledger-store`) was opened tick 226 and HOLD'd
by the conductor for a HARD RULE 6 violation — `crates/argo-store/src/subtask.rs`
is at **567 LOC**, over the project's 500-LOC ceiling. The author tab
self-exited before fixing it.

The fix is mechanical: tests are 196 of those 567 LOC. Project
convention (established by `tenant_quota_tests.rs` and similar
sibling test files) is to split tests into a sibling `*_tests.rs`
file when impl + tests would exceed 500.

All other content is verified clean by the conductor: migration 29,
runtime SubtaskSink trait, CLI list/show/tree, 7 unit + 3 integration
tests, clippy clean. Just needs the file split.

## Approach

This is NOT a new implementation. Recovery procedure:

1. `cd ~/code/argo`
2. `git fetch origin claude-subtask-ledger-store:claude-subtask-ledger-store-recovery`
3. `git worktree add /tmp/rebase-924 origin/main`
4. `cd /tmp/rebase-924 && git checkout claude-subtask-ledger-store-recovery`
5. `git rebase origin/main` — likely clean since #924 was branched recently
6. Open `crates/argo-store/src/subtask.rs`. Find the `#[cfg(test)] mod tests { ... }` block.
7. Cut everything from `#[cfg(test)]` through the matching closing `}` (~196 LOC) into a new file `crates/argo-store/src/subtask_tests.rs`. Replace the cut content with `#[cfg(test)] mod subtask_tests;` (or whatever wiring matches the established `tenant_quota_tests.rs` pattern — check `argo-store/src/lib.rs` for how tenant_quota_tests is registered).
8. After split: `subtask.rs` should be ~371 LOC, `subtask_tests.rs` ~196 LOC. Both well under 500.
9. `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-store --lib`
10. `git push --force-with-lease origin claude-subtask-ledger-store-recovery:claude-subtask-ledger-store`
11. Confirm PR #924 is `MERGEABLE`: `gh pr view 924 --json mergeable`
12. If MERGEABLE: do NOT merge yourself — the conductor handles merges.

## Files you may touch
- `crates/argo-store/src/subtask.rs` — strip tests block
- `crates/argo-store/src/subtask_tests.rs` (NEW) — moved tests block
- `crates/argo-store/src/lib.rs` — narrowly: 1-line `#[cfg(test)] mod subtask_tests;` declaration if the established pattern requires it (check tenant_quota_tests wiring first)
- BUILD_STATUS.md — narrowly: change `claude-subtask-ledger-store` row from NEEDS-REWORK back to IN_REVIEW with rebase note

## Acceptance criteria
- [ ] `crates/argo-store/src/subtask.rs` ≤ 500 LOC
- [ ] `crates/argo-store/src/subtask_tests.rs` exists and contains only the moved test block
- [ ] All 7 store unit tests still pass after the split
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] `gh pr view 924 --json mergeable` returns `MERGEABLE`
- [ ] No expansion of scope beyond the split + rebase

## Branch + PR
- Pushes to existing branch `claude-subtask-ledger-store` (PR #924).
- Do NOT open a new PR.
