# Rebase claude-skill-auction PR #900

**Owner**: claude. **Depends on**: nothing (high-priority recovery task).

## Scope

PR #900 (`claude-skill-auction`) was opened tick 156 by a worker tab
that self-exited before rebasing. The PR has been hanging
`CONFLICTING` against current main since #899 (worker-duty-scheduler)
landed — both touched `argo-orchestrator`. The conflict is contained
to one crate.

Original brief: `docs/build-plan/streams/claude-skill-auction.md`.
Original PR body lists 15 unit + 2 integration tests passing; that
work is correct, just needs to land on top of current main.

## Approach

This is NOT a new implementation. The work is already done in branch
`claude-skill-auction` (sha `d7cbbf2577edee7022f84577cc5bb0ecf9e661ea`
at last check). Recovery procedure:

1. `cd ~/code/argo`
2. `git fetch origin claude-skill-auction:claude-skill-auction-recovery`
3. `git worktree add /tmp/rebase-900 origin/main`
4. `cd /tmp/rebase-900 && git checkout claude-skill-auction-recovery`
5. `git rebase origin/main`
6. Resolve conflicts STRICTLY within the original brief's may-touch
   list. Do NOT expand scope. Conflicts will be in:
   - `crates/argo-orchestrator/src/lib.rs` (mod declarations — both
     PRs added new modules; merge both `pub mod` lines)
   - `crates/argo-orchestrator/Cargo.toml` (deps — keep both PRs'
     additions if they don't collide)
   - Likely conflict in any shared file both PRs touched
7. `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-orchestrator`
8. `git push --force-with-lease origin claude-skill-auction-recovery:claude-skill-auction`
9. Confirm PR #900 is now `MERGEABLE`: `gh pr view 900 --json mergeable`
10. If MERGEABLE: do NOT merge yourself — the conductor handles merges. Just confirm and exit.
11. Update BUILD_STATUS.md: row for `claude-skill-auction` stays IN_REVIEW with #900 (no status change needed unless the rebase fails).

## Files you may touch
- Only files inside the rebase conflict resolution. No new code.
- BUILD_STATUS.md — narrowly: add a one-line note in the
  `claude-skill-auction` row that the rebase landed (e.g.,
  "rebased onto main tick<N>")

## Acceptance criteria
- [ ] PR #900 rebased onto current `origin/main`
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-orchestrator` clean on the rebased branch
- [ ] `gh pr view 900 --json mergeable` returns `MERGEABLE`
- [ ] BUILD_STATUS.md row for `claude-skill-auction` updated with rebase confirmation
- [ ] Do NOT merge the PR yourself — conductor handles that.

## Branch + PR
- Branch: `claude-skill-auction` (existing — force-push-with-lease after rebase)
- PR: existing #900 (no new PR; just update via push)

## Failure path
If rebase conflicts can't be resolved within the brief's may-touch
list (i.e., the conflict spans files outside the original
may-touch), mark this brief BLOCKED with one-line reason and notify
the conductor. Don't expand scope.
