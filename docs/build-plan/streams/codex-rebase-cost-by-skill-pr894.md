# Rebase codex-cost-by-skill-cli PR #894

**Owner**: codex. **Depends on**: nothing (high-priority recovery task).

## Scope

PR #894 (`codex-cost-by-skill-cli`) was opened by a worker tab that
self-exited before rebasing. The PR has been hanging `CONFLICTING`
since the skill-marketplace + tenant-quota landings — both touched
adjacent areas of `argo-cli/handlers/budget*.rs` and the SkillCmd /
TenantCmd dispatchers.

Original brief: `docs/build-plan/streams/codex-cost-by-skill-cli.md`.
Work is correct — just needs to land on top of current main.

## Approach

This is NOT a new implementation. Recovery procedure:

1. `cd ~/code/argo-codex`
2. `git fetch origin codex-cost-by-skill-cli:codex-cost-by-skill-cli-recovery`
3. `git worktree add /tmp/rebase-894 origin/main`
4. `cd /tmp/rebase-894 && git checkout codex-cost-by-skill-cli-recovery`
5. `git rebase origin/main`
6. Resolve conflicts STRICTLY within the original brief's may-touch list.
   Conflicts likely in:
   - `crates/argo-cli/src/handlers/mod.rs` — multiple landings added pub mods
   - `crates/argo-cli/src/cli_budget_cmds.rs` — #927 just landed `--format csv` adding new variants; merge both expansions
   - `crates/argo-cli/src/handlers/budget.rs` — same — keep both renderers
   - `crates/argo-store/src/lib.rs` — post-#915 split; re-exports may have moved
7. `cargo build && cargo clippy --workspace --all-targets -- -D warnings`
8. `cargo test -p argo-cli --bin argo budget` — focused tests pass
9. `git push --force-with-lease origin codex-cost-by-skill-cli-recovery:codex-cost-by-skill-cli`
10. Confirm PR #894 is `MERGEABLE`: `gh pr view 894 --json mergeable`
11. If MERGEABLE: do NOT merge yourself — the conductor handles merges.

## Files you may touch
- Only files inside the rebase conflict resolution. No new code.
- BUILD_STATUS.md — narrowly: note the recovery on the row.

## Acceptance criteria
- [ ] PR #894 rebased onto current `origin/main`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-cli --bin argo budget` clean
- [ ] `gh pr view 894 --json mergeable` returns `MERGEABLE`
- [ ] No expansion of scope

## Branch + PR
- Pushes to existing branch `codex-cost-by-skill-cli` (PR #894).
- Do NOT open a new PR.
