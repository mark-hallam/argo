# Round 105 — Process drift sweep + gateway SPA probe

- **Date**: 2026-05-27 (UTC)
- **Round**: 105
- **Pillar**: Operational hygiene (priority axis 4)
- **Verdict**: 🟡 PARTIAL (1 P3 carry + 1 new INFORMATIONAL gateway SPA-route observation; no P0/P1)

## Setup

```bash
bash scripts/check-unread-handoffs.sh
bash scripts/check-gateway-freshness.sh
bash scripts/check-findings-only-fix-by.sh
curl http://127.0.0.1:7400/{team,audit,workers,skills,api/v1/verticals}
gh pr list --state open --limit 100 --json number | jq length
```

## Observed

**Gateway freshness**: schema v83 (gateway) vs v84 (codebase) drift=1 — STATUS: FRESH (drift ≤2 ceiling). Gateway uptime 43406s (~12.1h monotonic), no restart needed this tick. Same v83/v84 carry as R102→R104 — the `make restart-live-gateway carry` is now at 7 rounds. Below brief-authoring threshold; the existing `ops-gateway-freshness-binary-staleness-detection-gap` TODO already covers the detection-gap class.

**Findings-only fix-by deadlines**: 4 findings within budget (`eval-cron-duty-fires-agent-runner-end-to-end`, `eval-workflow-execution-runs-real-step`, `mission-hire-audit-payload-shape-validation`, `mission-cu-macos-cross-tenant-capability-test`) — all 3 days remaining (deadline 2026-05-30). Zero due-today / zero overdue.

**Unread handoffs**: 1 ghost-stamped conductor entry (T08-30 interactive merge of #2434, ~8h carry) — P3 carry from R104, still below pilot threshold. 1 INFORMATIONAL supervisor-watchdog at `2026-05-27T16:20:06Z` (`sup=PID:85450/rc:0 slots=2/2 s1=ACTIVE s2=ACTIVE pulses=2/0-stale claude_pids=2/0hang remediated=none drift=2(stale-recovery) streak=0`). No action — heartbeat-refresh acknowledges per the script's documented ACK PATH.

**Open PR count**: 98 (up from R104's mid-merge state, reflecting the 24 net-new fix-briefs queued by the 4-hourly UX-E2E auditor + ongoing web-fix wave). Eight web-fix PRs are currently `in review` (status flips visible in `git log`: #2542 #2543 #2544 #2545 #2546 #2547 #2548 #2549); none have substrate-runtime impact (all web-only).

**SPA route render probe (gateway port 7400)**:

```
team:404
audit:200
workers:404
skills:200
verticals:200    (api/v1/verticals)
```

`/audit`, `/skills`, and `/api/v1/verticals` render. `/team` and `/workers` return 404 from the gateway.

This observation is **INFORMATIONAL** — the SPA is normally served by the Next.js dev server (port 3000), not the gateway. When the gateway serves the SPA, only specific routes are statically registered (the gateway's SPA-route allow-list is in `crates/argo-gateway/src/spa.rs` or similar). `/team` and `/workers` 404'ing from the gateway is consistent with this serving model — the routes work when the SPA dev server is up. This is NOT a regression that broke today; it's the normal asymmetry between dev (full SPA via Next.js) and prod-gateway (allow-list).

Worth a follow-up if the gateway-served SPA is intended to be a v1.0 demo path — but per CLAUDE.md "Remaining for v1.0 launch" item (d) (operator-only MP4 record), the demo capture uses the dev SPA, not the gateway-served route. No P0/P1 gap.

## Anti-theater check

- Gateway uptime is from the live `/health` endpoint (not a cached `make` target output), so the 12.1h monotonic is real.
- The 8 in-review PR list comes from `gh pr list --state open` (live GitHub state) intersected with `git log` status-flip commits, not from BUILD_STATUS.md (which can lag).
- The /team and /workers 404 came directly from `curl` against the live gateway, not from a stub or test fixture.

## Gap / follow-up brief candidates

No P0/P1 substrate gaps this round.

**P3 (no brief authored, per pilot rules)**: The /team + /workers gateway-served SPA-route 404 deserves a one-line investigation in a future round to confirm whether the gateway SPA allow-list is intentionally narrow (then close as documentation) OR whether v1.0 demo capture should test gateway-served path equivalence (then a brief like `ops-gateway-spa-route-coverage-audit-against-dev-spa-routes` would be appropriate). Flag for conductor triage; do not author this round.

**Process state remains GREEN-ish**:
- Runtime: 🟢 35-round streak r57→r105 HOLDING (gateway uptime 12.1h durable, drift=1 ≤2 ceiling).
- Authoring backlog: no new evaluator-authored briefs needed this round.
- Pilot 2026-05-27 inner-loop: 2/2 closures verified in R105 (#2535 zombie-claim guard + #2541 demo-session derivation, both POST-MERGE PASS).

117th anti-theater verification this week.
