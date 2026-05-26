---
date: 2026-05-26
round: 80
pillar: process / drift / handoff hygiene
verdict: 🟢 PASS (all green except 1 carry-forward stale-recovery row)
---

# Round 80 · S4 — process drift sweep

## Verdict

🟢 **PASS** — all process drift checks GREEN except one carry-forward stale-recovery row from r79 still unresolved.

## Setup + Observed

```
$ bash scripts/check-gateway-freshness.sh
gateway_schema=v83 codebase_latest=v83 drift=0
STATUS: FRESH

$ bash scripts/check-findings-only-fix-by.sh
🟢 eval-cron-duty-fires-agent-runner-end-to-end  within budget (2026-05-30, 4d)
🟢 eval-workflow-execution-runs-real-step       within budget (2026-05-30, 4d)
🟢 mission-hire-audit-payload-shape-validation  within budget (2026-05-30, 4d)
🟢 mission-cu-macos-cross-tenant-capability-test within budget (2026-05-30, 4d)
Tally: 4 within-budget · 0 due-today · 0 overdue · 0 informational · 4 total.

$ python3 scripts/find-stale-claim-drift.py --quiet
Found 1 drift rows (stale-recovery=1)

$ bash scripts/check-unread-handoffs.sh
1 informational supervisor-watchdog ping (quota_probe=skipped, no action inferred)

$ curl -sf http://127.0.0.1:7400/health
{"status":"ok","version":"0.0.1","schema_version":83,"uptime_seconds":2449}

$ curl /audit /api/v1/skills /team
/audit:200 /api/v1/skills:200 /team:404 (expected — /team is SPA route, gateway only serves API)

Halt sentinels: /tmp/argo-stop ABSENT · ~/.argo/stop ABSENT
```

## Drift row detail

The single drift row is unchanged from r79:

```
[stale-recovery] fix-demo-seed-cli-operator-action-runbook | IN_REVIEW | STALE-FIXED |
  PR #2385 opened by codex on 2026-05-26: option (a) canonical runbook
  seed CLI step + verification curls; option (b) auto-seed not implemented.
```

The brief was marked STALE-FIXED but a PR (#2385) is in fact open carrying it. The states are inconsistent: a STALE-FIXED brief should have no live PR, and a live PR should not be on a STALE-FIXED brief. r79 S3 already flagged this; r80 confirms it persists unchanged.

The right fix is a single-line BUILD_STATUS.md amendment by the conductor (the evaluator's charter does not permit BUILD_STATUS.md edits): either (a) reopen the brief to IN_REVIEW and process #2385 normally, or (b) close #2385 as STALE-DUPLICATE.

## Anti-theater check

The four drift scripts are not coupled to each other — different code paths in `scripts/`, with `check-gateway-freshness.sh` reading the live socket, `find-stale-claim-drift.py` parsing BUILD_STATUS.md state-vs-PR-state, `check-findings-only-fix-by.sh` parsing fix-by deadlines from the ledger, `check-unread-handoffs.sh` parsing SHARED.md timestamp vs conductor cutoff. All four passing simultaneously is a stronger signal than any one of them passing.

The HTTP probes hit the gateway directly (PID 7348, port 7400, started 14:34 AEST). The `200` responses on `/audit` and `/api/v1/skills` would not be possible if the audit chain were tampering itself dead — the audit-write site is consulted on most route handlers.

## Recommendations for next conductor tick

P0 — none.

P1:
- (a) Resolve `fix-demo-seed-cli-operator-action-runbook` drift: single edit on BUILD_STATUS.md row OR a `gh pr close 2385` if STALE-DUPLICATE applies. r79 S3 already flagged.

P2:
- (b) Per S3, consider authoring `eval-argo-vertical-packs-loader-surface-pin` brief to extend the surface-pin wave into the just-merged-pitch-agent territory (#2384 demonstrates `argo-vertical-packs` is now a heavily-used loader crate; surface-pin coverage missing).
- (c) Per S2, Track P P-02 is now the single critical node blocking P-06/P-07/P-08. Treat as Track P critical path.

P3:
- (d) Acknowledge informational supervisor-watchdog ping (no action).

Cost: ≈4K tokens, ≈4min wall.
