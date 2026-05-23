# ops-prevent-concurrent-worker-git-reset-wipe-race

**Authored against**: origin/main `9d9f8fd40` on `2026-05-24`.
**Validate before claim**: bash scripts/check-brief-freshness.sh ops-prevent-concurrent-worker-git-reset-wipe-race

## Why this brief exists

Evaluator round-37 (commit `9b07a2ce6` on 2026-05-24) observed a **live wipe-race**:

> LIVE WIPE-RACE this round (concurrent worker `git reset --hard` blew away report dir + heartbeat + LATEST.md mid-session; only SHARED.md append survived; re-applied atomically).

This is the second observation of the same class of bug (the conductor heartbeat from tick-114 also noted: "Concurrent-worker race observed — ledger edits reverted twice by worker status-pushes, re-applied via perl + atomic commit chain.").

The shape of the bug: workers run `git reset --hard origin/main` as part of `claim-with-retry` or zombie-recovery in `scripts/claude-worker-loop.sh`. If a concurrent process (conductor, evaluator, auditor) has uncommitted edits in the worktree — particularly to `docs/skycot/audit/scheduled/LATEST.md`, `docs/skycot/agents/heartbeats/evaluator.json`, or evaluator report dirs — those edits are wiped silently. Re-applying them atomically (the workaround) is fragile and conductors can't always recover; the round-37 wipe lost the heartbeat and LATEST.md.

The fix isn't a single line — it requires:
1. **Diagnostic**: identify which `git reset --hard` call in `scripts/claude-worker-loop.sh` is the wiper (or whether it's `git fetch + reset` in the supervisor).
2. **Pre-reset stash**: before any `git reset --hard`, stash uncommitted changes; after the reset, restore the stash if it carried files outside the worker's may-touch list (heartbeats, audit reports, SHARED.md).
3. **Lock file**: optionally, a `.swarm-lock` advisory file that workers respect — if held by conductor/evaluator/auditor, defer the reset for N seconds.
4. **Move volatile state out of the worktree**: this is the cleanest fix — heartbeats and LATEST.md could live in `~/.skycot-state/` (untracked, outside any worker's reset path). But this is a larger refactor and may need its own brief.

This brief lands the **pre-reset stash + restore for the 3 well-known volatile paths** (heartbeats, scheduled audit dir, SHARED.md). Out of scope: full state relocation.

## Acceptance criteria

1. `scripts/claude-worker-loop.sh` (or whichever bash script in `scripts/` issues the offending `git reset --hard`): before any reset, stash uncommitted changes that match the three protected globs:
   - `docs/skycot/agents/heartbeats/*.json`
   - `docs/skycot/audit/scheduled/*.md` (including `LATEST.md`)
   - `docs/skycot/agents/SHARED.md`
2. After the reset, restore the stash. If a conflict arises (because the reset moved to a newer main where those files were touched), prefer the upstream version and emit a `⚠ wipe-race observed, dropped local volatile-state edit` line to the worker log.
3. Add a hermetic smoke test (`scripts/test/test-worker-loop-protects-volatile-state.sh`?) that creates a fake local edit in each of the 3 protected paths, runs the reset logic, and asserts the edit survived (or was logged as dropped, never silently wiped).
4. shellcheck clean.
5. No regression in the existing `claim-with-retry` flow — pre-flight + claim a clean TODO row still works in the smoke harness.

## Parallel-surface declarations

- No new pub surface. Bash-only.
- Related script work: `scripts/claude-worker-loop.sh` (the supervisor), `scripts/claim-with-retry.sh` (the claim path).

## Investigation hint

The worker-loop is the most likely culprit, but the conductor's own `git pull --ff-only` and the evaluator's scheduled-task prompts also touch the worktree. Worker should pre-flight `git grep -nE 'reset --hard|reset -—hard|git\.reset' -- scripts/` to enumerate every reset site before adding the stash wrapper.

## May-touch

- `scripts/claude-worker-loop.sh`
- `scripts/claim-with-retry.sh`
- Any new `scripts/test/test-worker-loop-*.sh` smoke harness

## Must NOT touch

- The advisory `.swarm-lock` design — out of scope for this brief.
- Moving volatile state out of `docs/` — also out of scope (would be its own brief).
- Conductor's `git pull --ff-only` path (the conductor is singleton, so it can't race itself).

## LOC budget

~80-150 LOC bash + ~30-60 LOC smoke harness.

## Priority

🟡 P2 OPS reliability. Round-37 wipe lost the heartbeat + LATEST.md mid-session (re-applied atomically as a workaround). Closing this prevents the next wipe.

## Source

Evaluator round-37 (commit `9b07a2ce6` on 2026-05-24):

> LIVE WIPE-RACE this round (concurrent worker `git reset --hard` blew away report dir + heartbeat + LATEST.md mid-session; only SHARED.md append survived; re-applied atomically).

Conductor tick-114 heartbeat (`docs/skycot/agents/heartbeats/conductor.json`):

> Concurrent-worker race observed — ledger edits reverted twice by worker status-pushes, re-applied via perl + atomic commit chain.
