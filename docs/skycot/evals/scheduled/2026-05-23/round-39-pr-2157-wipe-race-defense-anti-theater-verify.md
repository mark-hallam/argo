# Round 39 — PR #2157 wipe-race defense anti-theater verify

**Date**: 2026-05-23 (real UTC; round-38 stamped 2026-05-24 due to Melbourne-local-as-Z bug)
**Round**: 39 [scheduled-task hourly fire, read-only]
**Pillar / Claim**: Process integrity / Swarm protocol (no mission claim — ops contract)
**Verdict**: 🟢🟢 **PASS** (gold-standard anti-theater; closes round-37/38 surfaced recovery brief)

## Setup

PR [#2157](https://github.com/mark-hallam/argo/pull/2157) (`ops: prevent concurrent-worker git reset wipe-race`, merge commit `cd338914f`, 2026-05-24 01:50 AEST local = ~15:50 UTC on 2026-05-23) was merged in tick-117. The PR was the targeted closure of round-37's observed wipe-race incident (commit `9b07a2ce6`) where `git reset --hard origin/main` in `scripts/claim-with-retry.sh`'s race-loss recovery silently wiped `docs/skycot/audit/scheduled/LATEST.md`, the evaluator heartbeat, and the round's report dir — leaving only SHARED.md's append-only edit intact.

I verified by:
1. Reading the production helpers at [scripts/claim-with-retry.sh:37-88](scripts/claim-with-retry.sh:37) (`VOLATILE_PATHS`, `stash_volatile_state`, `restore_volatile_state`).
2. Reading the integration site at [scripts/claim-with-retry.sh:226-237](scripts/claim-with-retry.sh:226) — the stash-then-reset-then-restore sandwich around the destructive `git reset --hard "$REMOTE/$BRANCH"` recovery path.
3. Running `bash scripts/test/test-worker-loop-protects-volatile-state.sh` cold off origin/main → **11/11 assertions pass**.
4. Reading the smoke harness at [scripts/test/test-worker-loop-protects-volatile-state.sh:1-211](scripts/test/test-worker-loop-protects-volatile-state.sh) to confirm it exercises the production helpers via `source`, not a parallel mock copy.

## Observed

Smoke output (full):

```
[1/4] all 3 protected paths survive reset --hard
✓ stash_volatile_state returned 0 (created stash)
✓ heartbeats/evaluator.json edit survived reset
✓ audit/scheduled/LATEST.md edit survived reset
✓ agents/SHARED.md edit survived reset
[2/4] stash_volatile_state returns 1 when nothing dirty
✓ clean tree → return 1 (skip restore)
[3/4] negative control: non-protected file NOT preserved
✓ non-protected dirt → return 1 (correctly out of scope)
✓ non-protected file correctly wiped by reset (scope is the 3 globs)
[4/4] conflict path: upstream wins, '⚠ wipe-race' logged
✓ conflict-setup: stash returned 0
✓ conflict resolved to upstream (LOCAL_VARIANT dropped)
✓ wipe-race warning emitted on conflict
✓ post-conflict tree clean + stash drained
=== Results: 11 passed, 0 failed ===
```

Production seam wiring (verbatim):

```bash
# scripts/claim-with-retry.sh:37-41
VOLATILE_PATHS=(
    'docs/skycot/agents/heartbeats'
    'docs/skycot/audit/scheduled'
    'docs/skycot/agents/SHARED.md'
)
```

```bash
# scripts/claim-with-retry.sh:225-237 — the recovery-path sandwich
git -C "$REPO_ROOT" rebase --abort >/dev/null 2>&1 || true
stash_volatile_state "$REPO_ROOT"
local stash_rc=$?
git -C "$REPO_ROOT" reset --hard "$REMOTE/$BRANCH" >/dev/null 2>&1
if [[ "$stash_rc" -eq 0 ]]; then
    restore_volatile_state "$REPO_ROOT"
fi
```

## Anti-theater check

Four reasons this is not theatre:

1. **Smoke harness sources the real script.** Line 38: `source "$CLAIM_SCRIPT"` — the test exercises the production `stash_volatile_state` + `restore_volatile_state` functions directly, not parallel test-only definitions. A refactor that broke the helpers would flip the test red.

2. **Test 3 is a genuine negative control.** It writes a `CONCURRENT_NON_PROTECTED_EDIT` to `docs/skycot/agents/UNRELATED.md` (path outside the 3 globs), runs `stash_volatile_state`, asserts return code = 1 (no stash created), THEN runs `git reset --hard origin/main`, THEN asserts the unrelated-file dirt was NOT preserved. A "stash everything" theater implementation would pass test 1 but fail test 3 — the scope is precisely the 3 globs.

3. **Test 4 creates a real upstream divergence.** It clones the bare remote into a second worktree, edits LATEST.md to `UPSTREAM_VARIANT`, commits, pushes. Worker then fetches + resets + restores. Test asserts both that `UPSTREAM_VARIANT` (not `LOCAL_VARIANT`) wins AND that `⚠ wipe-race observed` is emitted to stderr AND that the working tree is clean afterward (no leftover conflict markers, no orphaned stash entry). A weaker implementation that silently dropped the warning, or left the stash dangling, would fail.

4. **Source-vs-execute guard is real.** Lines 253-255: `if [[ "${BASH_SOURCE[0]:-$0}" == "${0}" ]]; then main "$@"; fi` — sourcing the script does NOT trigger a real claim attempt. Without this guard the smoke harness would hang on `git push` or burn a real ledger row.

The production sandwich at line 226-237 wraps the destructive reset in exactly the order the docstring promises: stash → reset → conditional-restore (skipped when stash didn't take, with the failure mode documented inline as "lesser evil — exactly what the unwrapped reset already did").

## Gap / follow-up briefs

**NONE.** This PR fully closes round-37's surfaced `ops-worker-refuse-reset-hard-with-unstaged-eval-writes` and round-38's `ops-prevent-concurrent-worker-git-reset-wipe-race` recommendations.

Re-surface for visibility:

- The conductor's `git pull --ff-only` path is explicitly out of scope per the PR body ("singleton; can't race itself"). Verified at PR body line "Out of scope (per brief)".
- The `.swarm-lock` advisory file is also out of scope — would be its own brief. Not currently observed as load-bearing.

22nd anti-theater-honest substrate/ops contract on main this week.
