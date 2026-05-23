# Round 38 — process drift sweep

- **Date**: 2026-05-24
- **Round**: 38
- **Pillar / Claim**: Process gates (evaluator's second voice on conductor's tick-level hygiene)
- **Verdict**: 🟢 mostly clean / 🟡 1 informational (gateway DOWN, repeated)

## Setup

```bash
curl -sf http://127.0.0.1:7400/health   # exit 22 (no output)
bash scripts/check-unread-handoffs.sh
bash scripts/check-findings-only-fix-by.sh
```

## Observed

### Gateway

`curl -sf http://127.0.0.1:7400/health` → exit 22, no body. **DOWN at session start.** Informational only — read-only evaluator; conductor's STEP 4.7 catches gateway health.

Pattern: gateway has been intermittently down across rounds 24/25/27/33/35/37/38. Round-25 had a brief mid-session crash captured by the companion Chrome-MCP evaluator. The hypothesis — worker-supervisor watchdog killing idle gateway — surfaced as `ops-gateway-reliability-investigate-mid-session-death` (queued from round-25, P2). Not re-surfaced this round.

### Unread handoffs

```
✅ No unread handoffs from other agents (cutoff 2026-05-24T15:13:00Z).
```

Clean.

### Findings-only fix-by tracker

```
🟢 eval-cron-duty-fires-agent-runner-end-to-end             within budget (2026-05-30, 7 days remaining)
🟢 eval-workflow-execution-runs-real-step                   within budget (2026-05-30, 7 days remaining)
🟢 mission-hire-audit-payload-shape-validation              within budget (2026-05-30, 7 days remaining)
🟢 mission-cu-macos-cross-tenant-capability-test            within budget (2026-05-30, 7 days remaining)
🟢 mission-cross-tenant-test-audit-export                   within budget (2026-05-25, 2 days remaining)
```

All 5 within budget. **`mission-cross-tenant-test-audit-export` has 2 days remaining (due tomorrow 2026-05-25).** Flag for next round's tick.

### Round-37 unactioned recommendations — status check

| Brief | Status |
|---|---|
| `ops-evaluator-scheduled-task-prompt-use-flavoured-heartbeat-path` (P1) | **LANDED** as `ops-evaluator-scheduled-task-prompt-write-side-runbook` (#2155, `d6f706638`) |
| `ops-evaluator-interactive-launch-template-use-flavoured-heartbeat-path` (P1) | **subsumed by #2155** |
| `ops-worker-refuse-reset-hard-with-unstaged-eval-writes` (P2 — wipe-race) | UNAUTHORED — re-surfacing |
| `eval-gateway-uuid-bound-routes-tenant-ownership-sweep` (P1 — round-35) | UNAUTHORED — re-surfacing |

Write-side runbook #2155 (`docs/skycot/ops/heartbeat-timestamp-hygiene.md`) is the operator's chosen closure. Different shape than my brief recommendation (runbook instead of code change), but same end state: future evaluator sessions stamp UTC correctly and use distinct heartbeat paths.

**This round's heartbeat write targets `evaluator.json` per the SKILL.md prompt's literal instruction.** If the runbook supersedes the prompt instruction, the SKILL.md prompt is now stale → follow-up `ops-evaluator-skill-md-update-from-runbook` brief warranted (prompt-template change, not runtime-code change).

### LIVE WIPE-RACE AGAIN — 2nd consecutive round

Between writing the 4 round-38 reports and `git add` succeeding, a concurrent worker `git reset --hard origin/main` blew away the entire `docs/skycot/evals/scheduled/2026-05-24/round-38-*.md` set. Staged metafiles survived because they were already in git's index. Re-wrote all 4 reports + committed atomically on second attempt. **The race vector is alive even with runbook #2155 in place.**

Round-37's brief recommendation `ops-worker-refuse-reset-hard-with-unstaged-eval-writes` (P2, defensive gate) — **promote to P1** given second consecutive observation. Worker's claim-with-retry recovery `git reset --hard` must inspect `docs/skycot/evals/` + `docs/skycot/agents/heartbeats/` for unstaged writes before nuking the worktree.

## Anti-theater check

The drift sweep is informational by design — exit-code-level signals (curl-22, scripts exit-0) prove the gates are working. Two reasons this still has value:

1. **Catches drift the conductor's `STEP 4.5b check-unread-handoffs.sh` would also catch** — second-voice confirmation.
2. **Tracks unactioned brief recommendations** across rounds. Round-37 surfaced two P1s; both landed (#2155 + #2152) by round-38. **Brief→ticket→PR loop is healthy this tick.**

## Gap / follow-up brief candidates

Re-surfaced + promoted:

- **`ops-worker-refuse-reset-hard-with-unstaged-eval-writes`** — **PROMOTE P2 → P1.** Wipe-race observed 2 rounds consecutive. ~20-40 LOC. Defensive gate in worker's `claim-with-retry` recovery flow: if `git status --porcelain` shows unstaged changes under `docs/skycot/evals/` or `docs/skycot/agents/heartbeats/` and *this worker didn't make them*, refuse the `git reset --hard` + surface to SHARED.md. The runbook from #2155 helps with timestamp hygiene but does NOT prevent the worktree-wipe vector.
- **`eval-gateway-uuid-bound-routes-tenant-ownership-sweep`** (P1, ~150-200 LOC) — third tick unauthored. Two instances of pattern landed: #2146 (`decide_request` tenant pin) + still-TODO `mission-gateway-session-export-tenant-bind`. Sweep enumerating every gateway route binding on UUID alone without tenant pre-flight.

**NEW from this round:**

- **`ops-evaluator-skill-md-update-from-runbook`** (P3, ~5-10 LOC) — once #2155's runbook is the source of truth, update the scheduled-task SKILL.md prompt to reference the runbook + write to a flavoured heartbeat path (e.g. `evaluator-scheduled.json`). Currently the SKILL.md prompt still says "write your fresh heartbeat at `docs/skycot/agents/heartbeats/evaluator.json`" — stale per the runbook. Low priority; the prompt won't get pulled-in automatically by the next scheduled fire.

## Closing

Process drift gates clean. Round-37's recommended P1 closures both landed (#2152 + #2155). 1 due-tomorrow finding-only test (cross-tenant audit-export) flagged. **WIPE-RACE PROMOTE: `ops-worker-refuse-reset-hard-with-unstaged-eval-writes` P2 → P1 after 2nd consecutive observation.**
