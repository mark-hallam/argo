# Rebase codex-recover-cache-cli PR #911

**Owner**: codex. **Depends on**: nothing (high-priority recovery task).

## Scope

PR #911 (`codex-recover-cache-cli`) was opened tick 178 by a worker tab
that self-exited before rebasing. The PR has been hanging `CONFLICTING`
against current main. **As of tick 197 it is FULLY UNBLOCKED** — both
PRIORITY architectural splits (#914 cli-rs-split + #915 argo-store-lib-split)
have merged. Conflicts are now mechanical to resolve.

Original brief: `docs/build-plan/streams/codex-recover-cache-cli.md`.
Original PR body verifies 7 store + 5 handler unit + 1 smoke + clippy
clean. The work is correct — just needs to land on top of current main.

## Approach

This is NOT a new implementation. Recovery procedure:

1. `cd ~/code/argo-codex`
2. `git fetch origin codex-recover-cache-cli:codex-recover-cache-cli-recovery`
3. `git worktree add /tmp/rebase-911 origin/main`
4. `cd /tmp/rebase-911 && git checkout codex-recover-cache-cli-recovery`
5. `git rebase origin/main`
6. Resolve conflicts STRICTLY within the original brief's may-touch list.
   Conflicts likely in:
   - `crates/argo-cli/src/cli.rs` (now post-#914 split — variant ordering may differ)
   - `crates/argo-cli/src/handlers/mod.rs` (mod declarations — both #914 and other landings added new modules; keep both)
   - `crates/argo-store/src/lib.rs` (now post-#915 split — `pub use` lines may need merging into the new sibling files)
7. `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli --bin argo recover && cargo test -p argo-store --lib`
8. `git push --force-with-lease origin codex-recover-cache-cli-recovery:codex-recover-cache-cli`
9. Confirm PR #911 is `MERGEABLE`: `gh pr view 911 --json mergeable`
10. If MERGEABLE: do NOT merge yourself — the conductor handles merges. Confirm and exit.

## Files you may touch
- Only files inside the rebase conflict resolution. No new code.
- BUILD_STATUS.md — narrowly: add a one-line note that rebase landed
  on `codex-recover-cache-cli` row (don't change status; conductor merges).

## Acceptance criteria
- [ ] PR #911 rebased onto current `origin/main`
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli --bin argo recover` clean
- [ ] `gh pr view 911 --json mergeable` returns `MERGEABLE`
- [ ] No expansion of scope beyond conflict resolution
- [ ] BUILD_STATUS.md row for `codex-recover-cache-cli` notes the rebase

## Branch + PR
- This task pushes to existing branch `codex-recover-cache-cli` (PR #911).
- Do NOT open a new PR.
