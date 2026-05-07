# Fix + rebase codex-connector-authorize-cli PR #905

**Owner**: codex. **Depends on**: nothing (high-priority recovery task).

## Scope

PR #905 (`codex-connector-authorize-cli`) was opened tick 178 by a
post-compaction recovery tab that self-exited before fixing the clippy
errors the conductor flagged. The PR has 3 `possible_missing_else`
errors (all from compacted single-line if-chains during in-place
conflict resolution) plus has gone CONFLICTING since.

Conductor's tick-197 comment on the PR lists the exact errors:
- `crates/argo-cli/src/handlers/connector_list_status.rs:19` cols 223, 343
- `crates/argo-cli/src/handlers/connector_revoke.rs:21` col 288

Original brief: `docs/build-plan/streams/codex-connector-authorize-cli.md`.
The rest of the work is correct — just needs the clippy fix + a rebase.

## Approach

This is NOT a new implementation. Recovery procedure:

1. `cd ~/code/argo-codex`
2. `git fetch origin codex-connector-authorize-cli:codex-connector-authorize-cli-recovery`
3. `git worktree add /tmp/rebase-905 origin/main`
4. `cd /tmp/rebase-905 && git checkout codex-connector-authorize-cli-recovery`
5. `git rebase origin/main` — resolve conflicts STRICTLY within original may-touch list
6. Fix 3 clippy errors:
   - Open `crates/argo-cli/src/handlers/connector_list_status.rs:19` — split single-line `if` chains into multi-line `if { } else { }` blocks
   - Open `crates/argo-cli/src/handlers/connector_revoke.rs:21` — same fix at col 288
   - Use clippy's auto-fix where safe: `cargo clippy --fix --workspace --allow-dirty --allow-staged -- -D warnings`
7. `cargo build && cargo clippy --workspace --all-targets -- -D warnings` — must be clean
8. `cargo test -p argo-cli --bin argo connector` — focused tests pass
9. `git push --force-with-lease origin codex-connector-authorize-cli-recovery:codex-connector-authorize-cli`
10. Confirm PR #905 is `MERGEABLE`: `gh pr view 905 --json mergeable`
11. If MERGEABLE: do NOT merge yourself — the conductor handles merges.

## Files you may touch
- `crates/argo-cli/src/handlers/connector_list_status.rs` — clippy fix only
- `crates/argo-cli/src/handlers/connector_revoke.rs` — clippy fix only
- Files inside the rebase conflict resolution
- BUILD_STATUS.md — narrowly: note the recovery on the row

## Acceptance criteria
- [ ] All 3 `possible_missing_else` errors fixed
- [ ] PR #905 rebased onto current `origin/main`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-cli --bin argo connector` clean
- [ ] `gh pr view 905 --json mergeable` returns `MERGEABLE`
- [ ] No expansion of scope

## Branch + PR
- Pushes to existing branch `codex-connector-authorize-cli` (PR #905).
- Do NOT open a new PR.
