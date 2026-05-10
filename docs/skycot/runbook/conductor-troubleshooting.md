# SKYCOT — Conductor troubleshooting

Operator-facing footguns and confusing-but-non-fatal error messages
encountered while running the swarm conductor. Add new entries here
when an error surface wastes more than ~5 minutes of investigation
time on what turns out to be a no-op.

For broader audit / observability tooling, see
[observability.md](observability.md). For multi-machine ops, see
[../MULTI_MACHINE_OPS.md](../MULTI_MACHINE_OPS.md).

---

## Conductor footguns (common operator confusion)

### `gh pr merge --delete-branch` fails when a worker worktree is open

**Symptom.** After running `gh pr merge <num> --squash --delete-branch`,
the command exits non-zero with output like:

```
failed to delete local branch claude-foo: failed to run git: error: cannot delete branch 'claude-foo' used by worktree at '/private/tmp/foo'
```

**What it means.** The merge **succeeded** on GitHub. Only the
local-branch-delete sub-step failed, because a worker on this machine
(or another machine sharing the checkout) still has a worktree at
`/private/tmp/<brief-id>/` checked out on that branch. The non-zero
exit code makes it look like the merge bounced — it didn't.

**How to verify the merge actually landed.**

```bash
gh pr view <num> --json state,mergedAt --jq '.state + " @ " + (.mergedAt // "n/a")'
# Expect: MERGED @ 2026-...
```

If `state == MERGED`, treat the local-branch-delete failure as a
warning, not an error. Do NOT re-run the merge.

**Cleanup.** When the worker session ends naturally it removes its
worktree. To clean up immediately from the conductor side:

```bash
git worktree remove --force /private/tmp/<brief-id>
git worktree prune
git branch -d claude-<brief-id>          # now succeeds
```

The conductor's zombie-claim sweep (see `BUILD_STATUS.md` header
"CONDUCTOR HARD RULE — ZOMBIE-CLAIM SWEEP") will eventually surface
this brief if its ledger row stays stuck, but the worktree itself is
local-machine state and won't auto-clean across machines.
