# Round 86 — Process drift sweep

- **Date**: 2026-05-26T12:55Z (Melbourne 22:55 AEST)
- **Round**: 86
- **Pillar / Claim**: Operational hygiene — halt sentinels / gateway freshness / handoff budget / ledger drift / findings-only deadlines
- **Verdict**: 🟢 PASS (durably stable; 28-round 🟢 streak r57→r86 holding)

## Setup

```bash
ls /tmp/argo-stop ~/.argo/stop 2>&1
#   ls: ~/.argo/stop: No such file or directory
#   ls: /tmp/argo-stop: No such file or directory

curl -sf http://127.0.0.1:7400/health
#   {"status":"ok","version":"0.0.1","schema_version":83,"uptime_seconds":27279}

bash scripts/check-gateway-freshness.sh
#   gateway_schema=v83 codebase_latest=v83 drift=0
#   STATUS: FRESH  (drift <= 2)

bash scripts/check-unread-handoffs.sh
#   🔴 UNREAD HANDOFFS (1) … r85 evaluator line (my own — expected; conductor cutoff is what matters here)

python3 scripts/find-stale-claim-drift.py --quiet
#   Found 0 drift rows (none)

bash scripts/check-findings-only-fix-by.sh
#   🟢 4 within-budget · 0 due-today · 0 overdue · 0 informational · 4 total
```

## Observed

**Halt sentinels**: BOTH ABSENT (`/tmp/argo-stop`, `~/.argo/stop`). Swarm is ACTIVE.

**Gateway**:
- HTTP 200 on `/health`
- schema=v83 matches latest codebase migration (v83=v83 FRESH)
- uptime 27279s = **~7.58h durably stable**
- Spans the entire #2404 → #2405 → #2406 demo-blocker wave (4 PRs merged on this gateway instance
  without restart), demonstrating SPA-only fixes truly don't perturb runtime as expected

**Findings-only fix-by deadlines**: 4/4 within budget (all due 2026-05-30, 4 days remaining):
- eval-cron-duty-fires-agent-runner-end-to-end
- eval-workflow-execution-runs-real-step
- mission-hire-audit-payload-shape-validation
- mission-cu-macos-cross-tenant-capability-test

**Ledger drift**: `find-stale-claim-drift.py` reports 0 drift rows. The UX-AUDIT-T04-41 + T2007MEL
duplicate-row drifts from rounds 81-84 have all been dedupe'd by conductor.

**Unread handoffs**: only my own r85 evaluator line shows in `check-unread-handoffs.sh` (script
filters by conductor's cutoff at 11:36Z; r85 = 11:52Z is after). This is the expected "evaluator
sees its own unread handoff because conductor hasn't ticked yet to ingest it" race condition. Not
a finding.

**Demo-MP4 P0 progress** (the load-bearing v1.0 ship gate):
- **3/4 MERGED**: #2404 Scene 3 (onboarding demo-bypass) + #2405 Scene 6 (audit drill-in fallback)
  + #2406 Scene 5 (approvals/team counts agree)
- **1/4 worker-CLAIMED**: `fix-skills-marketplace-perpetual-loading` (claude worker claimed at
  commit `f2b37c86e`; no PR open yet; if claim is stale by r87 recommend conductor surface to
  operator)

**Recent merges since last round** (post-`90d5b8c79` would be r86's anchor):
- `ea161df3a` status: T11-48
- `5808ccd78` #2406 merged
- `5911257fa` status: claim fix-playwright-hire-flow-regression-cluster
- (only #2406 was substrate-affecting; all other commits are status / claim flips)

## Anti-theater check

Process scripts are not test-rewrites-itself-green surfaces — they read live state from the
ledger, the gateway HTTP endpoint, the filesystem, and BUILD_STATUS.md regex. Each verdict is a
fresh observation, not a cached assertion. The 28-round 🟢 streak (r57→r86) is durable because
each round re-runs the sweep cold; the streak doesn't carry from memory.

## Gap / follow-up

**No P0/P1 process findings this round.**

**P3 candidate**: `ops-conductor-pre-merge-gate-web-typecheck-pin` — see the typecheck-regression-sweep
report for shape. Out of v1.0 demo critical path. Out of scope unless the operator surfaces a
preference for tightening the web-only fix-* auth lane.

**P1 follow-through reminder** for conductor next tick:
- Surface to operator the unclaimed r85-S2 brief `fix-pr-2404-use-provision-builddemoprovisionresult-set-created-false`
  + the new r86 P2 brief shape `fix-pr-2366-provenance-block-hastext-accepts-optional`. Both fix
  the only 2 TS errors currently on main; together they restore green `pnpm typecheck`.
- The 1/4 outstanding demo-MP4 P0 (`fix-skills-marketplace-perpetual-loading`) is claude-worker-CLAIMED
  but no PR open. If the claim ages past r87 surface to operator for promotion or re-claim.

## Round 86 summary

3 scenarios this round:

| # | Subject | Verdict | Notes |
|---|---|---|---|
| 1 | PR #2406 (Scene 5 demo P0) POST-MERGE | 🟢 PASS | source-of-truth derive is structural; e2e regression-pin in place; 14 vitest PASS across approvals + team + workers; live gateway 7.58h stable |
| 2 | Typecheck regression sweep | 🟡 P2 | 2 TS errors on main: r85-S2 (use-provision.ts:58, brief exists TODO) + NEW r86 (ProvenanceBlock.tsx:36 from #2366); new brief candidate recommended |
| 3 | Process drift sweep | 🟢 PASS | halt absent, gateway FRESH, 0 drift rows, 4/4 findings within budget, demo-MP4 P0 progress 3/4 MERGED + 1/4 worker-CLAIMED |

28-round 🟢 streak r57→r86 holding on mission-claim + process dimensions. Build-gate finding is
separate-axis (web typecheck) and recommended for P2 brief — does not break the streak.

82nd anti-theater verification this week.
