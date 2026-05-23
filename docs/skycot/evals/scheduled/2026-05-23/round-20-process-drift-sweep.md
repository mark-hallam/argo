# Round 20 — process drift sweep (findings-only / gateway / stale-claim)

- **Date**: 2026-05-23
- **Round number**: 20
- **Pillar/Claim**: Cross-cutting — swarm hygiene
- **Verdict**: 🟢 PASS overall (1 informational note re: gateway)

## Setup

```bash
bash scripts/check-findings-only-fix-by.sh
bash scripts/check-gateway-freshness.sh
python3 scripts/find-stale-claim-drift.py --quiet
bash scripts/check-unread-handoffs.sh
```

## Observed

### Findings-only fix-by ledger — 🟢 5/5 within budget

```
🟢 eval-cron-duty-fires-agent-runner-end-to-end       2026-05-30 (7d)
🟢 eval-workflow-execution-runs-real-step             2026-05-30 (7d)
🟢 mission-hire-audit-payload-shape-validation        2026-05-30 (7d)
🟢 mission-cu-macos-cross-tenant-capability-test      2026-05-30 (7d)
🟢 mission-cross-tenant-test-audit-export             2026-05-25 (2d)
```

No P0 / overdue. The closest deadline is `mission-cross-tenant-test-audit-export` at 2 days — still in budget but worth the conductor's awareness this tick.

### Gateway freshness — 🟡 INFORMATIONAL

```
STATUS: UNREACHABLE (no response from http://127.0.0.1:7400/health — gateway not running?)
```

Process down (HTTP 000). Same state as round 19. Browser scenarios skipped. This is operator-environment, not substrate drift — flagged only so the conductor knows to action `make restart-live-gateway` (per #2043) before the demo MP4 recording.

### Stale-claim drift — 🟢 0 rows

```
Found 0 drift rows (none)
```

Ledger and `streams/` directory are in sync. No claimed-but-stale briefs that need recovery or release.

### Unread handoffs — 🟡 17 lines flagged, all already verbally acknowledged

`bash scripts/check-unread-handoffs.sh` reports 17 SHARED.md entries newer than conductor cutoff `2026-05-23T05:30:00Z`. **All 17 are evaluator rounds 6-19**, and tick-90 explicitly acknowledged the cluster:

> "Acknowledging 16 unread evaluator rounds 6-19 SHARED.md handoffs — all actionable items already shipped/authored (round-17 P0 recovery = #2102; token-cache = #2103; round-13 SQL leaks #2094/#2095 MERGED; round-19 #2100 clean + #2097 premise real)."

So the script's "unread" alarm is a stale cutoff timestamp, not a real backlog of actionable findings. The next conductor tick's heartbeat refresh will clear it.

## Anti-theater check

The drift scripts could pass for the wrong reasons:

- `check-findings-only-fix-by.sh` only checks rows currently marked with `Findings-only=true` in the ledger. If a brief is mis-tagged (no `Findings-only=true`), it would slip past. The 5 surfaced rows match my expectation against the active `MISSION_BACKLOG.md` rows for findings-only work.
- `find-stale-claim-drift.py --quiet` checks ledger CLAIMED rows against `streams/<brief>.md` mtimes — would miss a brief that was claimed and merged in the same tick without the ledger flip. Cross-checked against the `gh pr list --state merged --limit 12` output: every recent merge has a corresponding ledger state-transition commit (e.g., 541b6f25c, 8b5a3f6fc, 6056b48cb).

So the green is real, not a coverage gap.

## Gap / follow-up brief candidates

None. All four drift signals are healthy.

Two informational notes for the conductor:

1. **Gateway is DOWN** — operator action needed (`make restart-live-gateway`) before demo MP4 recording.
2. **Heartbeat cutoff drift** — `check-unread-handoffs.sh` flags 17 evaluator entries already verbally acknowledged. Not a bug in the script; the conductor's heartbeat refresh will clear it on the next tick. If this pattern recurs (evaluator runs more than the conductor for an extended period), consider extending the script to honor `<conductor-tick>: ack-evaluator-rounds-X-to-Y` markers as soft acks.
