# Round 106 — Process drift: gateway carry + findings tally

**Date**: 2026-05-27 (UTC)
**Round**: 106
**Pillar / Claim**: Process — runtime hygiene (axis 4)

## Verdict

🟡 **PARTIAL — P2** (runtime healthy except for the gateway-restart carry; the storm in scenario S1 is the dominant signal of this round)

## Setup

```
curl -sf -m 3 http://127.0.0.1:7400/health
# {"status":"ok","version":"0.0.1","schema_version":83,"uptime_seconds":50723}

bash scripts/check-gateway-freshness.sh
# gateway_schema=v83 codebase_latest=v84 drift=1
# STATUS: FRESH  (drift <= 2)

bash scripts/check-findings-only-fix-by.sh
# 🟢 eval-cron-duty-fires-agent-runner-end-to-end           within budget (3 days)
# 🟢 eval-workflow-execution-runs-real-step                 within budget (3 days)
# 🟢 mission-hire-audit-payload-shape-validation            within budget (3 days)
# 🟢 mission-cu-macos-cross-tenant-capability-test          within budget (3 days)
# Tally: 4 within-budget · 0 due-today · 0 overdue · 4 total

bash scripts/check-unread-handoffs.sh
# Found 3 handoffs newer than conductor cutoff
#   2026-05-27T16:20:06Z [supervisor-watchdog] (informational)
#   2026-05-27T16:30:00Z [evaluator] round 105 (already-landed)
#   2026-05-27T16:49:44Z [supervisor-watchdog] (informational)
# summary: already-landed=1, informational=2
```

## Observed

**Gateway**:
- Alive at :7400, schema v83 (codebase v84), drift=1 FRESH.
- Uptime 50,723s ~14.1h — process started 2026-05-27T04:21Z UTC = 14:21 AEST May 27.
- This is the **8th consecutive round** carrying "operator: please run `make restart-live-gateway`" (R98 first surface → R99 → R100 → R101 → R102 → R103 → R104 → R105 → R106). Drift hasn't worsened past 1 yet, so it remains FRESH not STALE.
- Conductor ghost-stamped T08-30 entry (the "🟡 PARTIAL but operator chose option-3 LIVE-flip" #2434 merge from R95 cycle) is now **11-round carry** (R96–R106), still below pilot threshold for promotion.

**Findings deadlines**:
- 4 within-budget findings all at 3 days remaining (deadline 2026-05-30). Slow burn.
- 0 overdue, 0 due-today. Healthy.

**Unread handoffs**:
- 3 entries newer than conductor cutoff 2026-05-27T16:12:00Z:
  - 2× supervisor-watchdog pings (informational; supervisor health snapshot lines)
  - 1× evaluator R105 self-handoff (already-landed; #2541 confirmed POST-MERGE)
- All ACK-able via heartbeat refresh per the check script's guidance.

**Conductor cutoff staleness**: 2h16m since last refresh; check script flagged "review PROTOCOL_LOOP_PROMPT.md Step 4.6 compliance". This is well below P2 threshold but worth noting.

## Anti-theater check

- Gateway freshness check runs `curl -sf` against the actual live gateway socket + diffs `--schema-version` from binary against latest schema migration on disk. Not a passing self-test.
- Findings check enumerates real `Findings-only fix-by` lines across the repo and compares to today's date. Real surface.
- Unread-handoffs check reads `docs/skycot/agents/SHARED.md` cutoff from `docs/skycot/agents/heartbeats/conductor.json` and grep-counts entries after that timestamp. Real surface.

## Gap / follow-up

- **Carry forward to R107**: gateway-restart at 8 rounds; not yet escalating to P1 because drift hasn't worsened past 1, but if codebase advances to v85 without restart this becomes investor-demo-impacting.
- **Carry forward to R107**: ghost-stamped conductor T08-30 entry at 11-round carry; if it persists past round 12 consider promoting to a "scripts/heal-ghost-conductor-stamps.sh" sweep brief (P3).
- **NEW for R106**: Once the P0 zombie-storm (S1) is remediated by operator supervisor-restart, R107 should verify (a) supervisor PID changed, (b) `worker-supervisor.sh:410` throttle invocation is in newly-started process's loaded code, (c) both storm rows force-demoted to BLOCKED, (d) zombie-release rate drops to ~0/min on origin/main.

## Severity rationale

P2 because:
- All measured drift remains within budget (drift=1, findings 3d remain, 0 overdue).
- 8-round gateway-restart carry is operator-decisional + non-urgent (drift hasn't crossed STALE threshold).
- The dominant signal of this round is S1 (P0 zombie storm), not the slow-burn process drift.

## Source

- This report.
- `scripts/check-gateway-freshness.sh` output captured live at fire time.
- `scripts/check-findings-only-fix-by.sh` output captured live at fire time.
- `scripts/check-unread-handoffs.sh` output captured live at fire time.
