# Round 74 — S1 — PR #2339 superseded by operator commit `5c4abb23e`

- **Date**: 2026-05-25 (UTC) / 2026-05-26 09:30 AEST Melbourne
- **Round**: 74
- **Pillar / Claim**: Process — supervisor reliability (ω-r68 RECURRENCE chain)
- **Verdict**: 🟡 PARTIAL — **P2 finding**: PR #2339 PRE-MERGE-PASSED at r73 head `15388f861` is now **DIRTY/CONFLICTING** because operator-authored commit `5c4abb23e` (~2h after r73 ship) landed a **competing-but-equivalent** root-cause fix for the same 3 callsites. PR #2339 should be **closed STALE-SUPERSEDED** OR rebased to retain only its test-seam value.

## Setup

```bash
gh pr view 2339 --json mergeable,mergeStateStatus,baseRefOid,headRefOid,files
# → mergeable=UNKNOWN→CONFLICTING, mergeStateStatus=DIRTY, baseRefOid=2ae5e9303 (8 commits behind current main af5e4acb5)

git fetch origin pull/2339/head:pr-2339
git show pr-2339 -- scripts/worker-supervisor.sh
git show 5c4abb23e -- scripts/worker-supervisor.sh
git log --oneline origin/main -10 -- scripts/worker-supervisor.sh
```

## Observed

**Same root cause, same shape of fix, 2h apart, by the same author (mark-hallam):**

| Aspect | PR #2339 (codex-authored 21:31:40Z) | Operator commit `5c4abb23e` (22:30:58Z) |
|---|---|---|
| Helper function name | `wait_for_stop_clear` | `wait_for_stop_to_clear` |
| Sleep cadence | `STOP_POLL_SECONDS=5` (env-overridable) | hardcoded `sleep 30` |
| Callsites replaced | 3 (top-of-loop / mid-RAM-wait / mid-backoff) | **same 3 callsites** |
| Old behavior fixed | `exit 0` → permanent slot death | `exit 0` → permanent slot death |
| New behavior | pause-loop + `continue`/`continue 2` | pause-loop + return-to-caller |
| Operator-relight procedure simplifies to | `rm /tmp/argo-stop ~/.argo/stop` | `rm /tmp/argo-stop ~/.argo/stop` |
| Extra value | env-var test seam for sentinels + new `test-worker-supervisor-rc0-respawn.sh` wired as Gate 7/8 of pre-merge-gate | none beyond the core pause-not-halt fix |

**Three places where the diffs collide on the same `if stop_present; then ... exit 0; fi` blocks** at `scripts/worker-supervisor.sh:308-310`, `342-344`, `406-408`. Both diffs replace those blocks with structurally-identical pause-not-halt logic. Git cannot 3-way merge because each side rewrites the same lines differently.

**Surviving unique value in PR #2339** if rebased:

1. New test script `scripts/test-worker-supervisor-rc0-respawn.sh` (+89 LOC) drives an env-overridden supervisor + fake worker + STOP-sentinel to prove the pause behavior. r73 PRE-MERGE-PASS observed it running in 4.137s wall.
2. Pre-merge gate addition (lines 74-101 of `conductor-pre-merge-gate.sh`) wires the test as Gate 7/8.
3. Three new env-overrides (`STOP_SENTINEL_TMP`/`STOP_SENTINEL_HOME`/`STOP_POLL_SECONDS`) + WORKER_SCRIPT-overridable ANTHROPIC_API_KEY / claude-on-PATH gates — these enable the test seam.

The function-name difference (`wait_for_stop_clear` vs `wait_for_stop_to_clear`) is cosmetic.

## Anti-theater check

Diffed both branches against base `2000d11db` (pre-fix `scripts/worker-supervisor.sh`):

- Operator commit changes lines 57-92 (adds helper) + 306-320 + 332-358 + 401-422 (3 callsite replacements).
- PR #2339 changes lines 37-69 (env-vars + helper) + 257-282 (gate-overrides) + 308-410 (3 callsite replacements + `BACKOFF` literal → env-var).

Both touch identical line ranges for the 3 callsites; both diffs are non-trivial; neither is a no-op. The DIRTY status is **real** not a GitHub stale-cache flake.

**Confirmation that operator-fix is already DEPLOYED**: live gateway uptime 10348s (~2.87h), restarted some time after r73 ship (22:21Z). Supervisor watchdog ping commit `af5e4acb5` shows the supervisor is alive on the new code path. r73's recommendation #2 (operator-only `launchctl kickstart`) appears to have been done at the same time.

## Gap / follow-up brief candidates

**Recommended action chain**:

1. 🟡 **P2 — close-or-rebase decision for PR #2339**. Two paths:
   - **(a) CLOSE PR #2339 as STALE-SUPERSEDED-BY-5c4abb23e** and author a thin follow-up brief preserving the test seam value only: `ops-add-rc0-respawn-regression-test-on-superseded-supervisor`. Scope: just the test script + the Gate 7/8 wire + the 3 env-overrides needed to make the test runnable. Probably ~50 LOC net (drops the 3 callsite changes since they're already on main). Risk: low; preserves the only durable value left in PR #2339.
   - **(b) Rebase PR #2339 onto current `af5e4acb5`** and resolve conflicts in favor of the operator's already-merged version (drop PR #2339's pause-loop changes; keep the test infra + env-overrides). Risk: medium; the env-var test seam touches the same `if stop_present` blocks the operator just rewrote.

   Recommend **(a)** — close-and-replace is more honest about what shipped and avoids relitigating the same lines.

2. 🟢 **P0 OPS** (carry from r70-r73 unchanged): MERGE **PR #2323** — 6th-round merge stall on a CLEAN/MERGEABLE branch (head `c53f63424`, 666 additions); live `/api/v1/skills` + `/api/v1/workflows` confirmed still HTTP 404 this round. Conductor has separately authored `fix-pr-2323-rebase-gateway-endpoints` (commit `a84871a22`) and superseded it via `a218196ab` — those status commits don't merge anything; the PR remains open and untouched.

3. 🟡 **P3** (carry from r72): author `ops-schema-lint-distinguish-leads-vs-lags-error-message` (~3 LOC cosmetic follow-up to #2334).

## Process observation

The pattern here — sibling agent or operator authoring a competing fix while a PR is in evaluator-PRE-MERGE-PASS state — is the same pattern flagged in r71 POSTSCRIPT (where 2 of 3 brief candidates were already authored by a sibling). **The evaluator → conductor handoff loop is fast enough now (~50min) that 2 agents can land overlapping fixes for the same defect within one evaluator round.** Worth surfacing as a P3 process brief if it recurs: `ops-claim-brief-before-evaluator-pre-merge-pass-to-avoid-overlap-with-direct-commit`. Not authoring this round; flagging for next.
