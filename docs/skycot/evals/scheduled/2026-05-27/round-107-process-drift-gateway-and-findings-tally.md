# Round 107 — Process-drift sweep

- **Date / fire**: 2026-05-27T19:26Z (06:26 AEDT 2026-05-28)
- **Round**: 107
- **Pillar**: process drift / conductor-evaluator second voice
- **Origin/main SHA at run**: `577827919`

## Verdict

🟡 **PARTIAL** — single drift carry continues (gateway schema v83 vs codebase v84, drift=1 within FRESH ≤2 ceiling, now **9-round carry** since R98). All other drift indicators are clean: 0 unread handoffs from non-conductor agents, 4 findings-only briefs all within 3-day budget, 0 stale-claim drift rows, 2 conductor ghost-stamp entries (both informational, expected during fresh-tick window).

Severity tier: **P3** (informational). The gateway carry is *not* a fresh defect — the codebase migration sequence shipped v84 cleanly and the v83 binary is the operator's currently-running investor-demo gateway. The "restart-live-gateway" operator action is the only resolution; treating as carry rather than escalating per established cadence (the brief `make-restart-live-gateway-detection-gap-coverage` covers this surface area).

## Setup

```bash
bash scripts/check-unread-handoffs.sh
bash scripts/check-findings-only-fix-by.sh
bash scripts/check-gateway-freshness.sh
curl -sf http://127.0.0.1:7400/health
```

## Observed

**Gateway freshness** (`scripts/check-gateway-freshness.sh`):
```
gateway_schema=v83 codebase_latest=v84 drift=1
STATUS: FRESH  (drift <= 2)
```

Live `/health`:
```json
{"status":"ok","version":"0.0.1","schema_version":83,"uptime_seconds":54197}
```

Gateway uptime ≈ 54,197s = 15.05 h. The R106-noted 8-round carry advances to 9. Operator action `make restart-live-gateway` still pending.

**Unread handoffs** (`scripts/check-unread-handoffs.sh`):
- 2 conductor ghost-stamp entries:
  - `2026-05-27T08:30:00Z [conductor] tick T08-30 interactive` — Claim 5 LIVE flip via #2434 (12-round carry; pre-cutoff legacy).
  - `2026-05-28T03:35:00Z [conductor] tick T13-25-aest scheduled` — most recent conductor tick (≈ 6 h prior to fire; ghost-stamped because conductor.json commit window=300s, but stamp shows actual tick was T13-25 AEDT = 03:35 UTC; this is a stale-stamp artifact of the scheduled-tick task rather than a genuinely unactioned entry).
- Script reports: `✅ No unread handoffs from other agents (cutoff 2026-05-28T03:35:00Z)`.
- 0 non-conductor handoffs queued for the next conductor tick. Watchdog pings at 19:19:37Z are informational (`slots=2/2 s1=ACTIVE s2=ACTIVE pulses=2/0-stale claude_pids=2/0hang remediated=none`).

**Findings-only fix-by** (`scripts/check-findings-only-fix-by.sh`):
```
🟢 eval-cron-duty-fires-agent-runner-end-to-end             within budget (2026-05-30, 3 days remaining)
🟢 eval-workflow-execution-runs-real-step                   within budget (2026-05-30, 3 days remaining)
🟢 mission-hire-audit-payload-shape-validation              within budget (2026-05-30, 3 days remaining)
🟢 mission-cu-macos-cross-tenant-capability-test            within budget (2026-05-30, 3 days remaining)

Tally: 4 within-budget · 0 due-today · 0 overdue · 0 informational · 4 total.
```

3-day budget remaining. No escalation needed this round.

**Storm-row sweep**:
- `fix-audit-list-kind-query-param-ignored` — BLOCKED with conductor stale-fixed note pointing at PR #2527 (`fix-audit-filter-key-action-type-vs-kind-vocab-alignment` — IN_REVIEW). Once #2527 merges, conductor will flip to MERGED-STALE-FIXED. No drift; correct conductor handling.
- `fix-landing-hero-mobile-stack-and-tablet-nav` — MERGED via #2561.

## Anti-theater check

The gateway-carry rollup is not noise-floor — it's a *real* drift that simply hasn't escalated past the FRESH ≤2 ceiling. If the codebase shipped v85 next sweep without operator restart, drift would jump to 2 (still FRESH); v86 would tip into STALE (>2) and trigger an escalation. The carry counter (9 rounds) is the early-warning instrument that lets the evaluator-conductor loop surface the operator dependency long before the drift threshold breaches. This is the system working as designed; the only theater risk is allowing the carry to mute itself, which the existing `make-restart-live-gateway-detection-gap-coverage` brief explicitly guards against.

The unread-handoffs script's 2-entry "ghost-stamp" report is also non-theater: the script intentionally surfaces conductor entries whose timestamps are >300s from the nearest conductor.json commit (a heuristic for "stamp written without a heartbeat commit"). One is a 12-round-old benign carry, the other is the most-recent conductor tick mis-stamped by the scheduled-task wall clock — informational, not actionable.

## Gap / follow-up brief candidates

None this round.

Track for R108:

- If gateway carry crosses 12 rounds without `make restart-live-gateway`, surface explicitly as an operator-action item in heartbeat headline (parity with the R106 operator-action surfacing for the supervisor restart, which did get actioned within ~9 h).
- Optional defensive: a one-shot `ops-gateway-auto-restart-on-schema-mtime-change` brief mirroring the unauthored supervisor self-restart brief — both surfaces share the same "long-running daemon never picks up new binary" failure mode. Hold pending operator decision; do not author preemptively.
