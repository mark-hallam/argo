# Round 41 — process drift sweep

**Date**: 2026-05-23
**Round**: 41
**Pillar/Claim**: Process gate / drift defense
**Verdict**: 🟡 PARTIAL — fix-by tracker + handoffs gate green; gateway DOWN (informational); a more severe drift surfaced from scenario 1 (pre-merge gate is masking RED tests, not just clippy)

## Setup

- `bash scripts/check-unread-handoffs.sh`
- `bash scripts/check-findings-only-fix-by.sh`
- `bash scripts/check-gateway-freshness.sh`

## Observed

### `check-unread-handoffs.sh`: 🟡 11 unread entries above conductor cutoff `2026-05-23T18:15:00Z`

All 11 are previous evaluator entries (rounds 23-31, 37, 38)
visible to the script because they bear timestamps after
`2026-05-23T18:15:00Z`. Several are **legitimately future-dated**
by the Melbourne-local-as-Z heartbeat-WRITE-side bug observed for
the **10th-consecutive round** (e.g. `2026-05-24T15:13:00Z` entries
are ~21h in the literal future).

This is the known WRITE-side gap. PR #2150 closed the READ side
(watchdog rejects future-dated heartbeats); the WRITE side still
needs operator action to update the scheduled-task SKILL.md prompts
outside the repo. Re-surfacing for visibility — not actionable by
the swarm.

### `check-findings-only-fix-by.sh`: 🟢 5/5 within budget

```
🟢 eval-cron-duty-fires-agent-runner-end-to-end        2026-05-30, 7 days remaining
🟢 eval-workflow-execution-runs-real-step              2026-05-30, 7 days remaining
🟢 mission-hire-audit-payload-shape-validation         2026-05-30, 7 days remaining
🟢 mission-cu-macos-cross-tenant-capability-test       2026-05-30, 7 days remaining
🟢 mission-cross-tenant-test-audit-export              2026-05-25, 2 days remaining  ⚠ FLAG
```

**`mission-cross-tenant-test-audit-export` is 2 days from deadline**
and has been flagged for 5 consecutive evaluator + conductor rounds
without a claim. If it doesn't claim by tomorrow it will be the
first findings-only-fix-by deadline breach since the tracker
shipped. Conductor: this needs explicit escalation in tick-123.

### `check-gateway-freshness.sh`: 🔴 UNREACHABLE (informational)

```
STATUS: UNREACHABLE  (no response from http://127.0.0.1:7400/health — gateway not running?)
```

Informational; swarm-compile load makes this typical during active
worker-supervisor sessions. Browser-only verification scenarios
were not attempted this round.

## Anti-theater check

The 3 scripts are read-only and report against `origin/main` HEAD
ledger state — not against my evaluator-private view. The
fix-by tracker reads the ledger's FINDINGS-ONLY headers directly.
The unread-handoffs script reads SHARED.md timestamp-by-timestamp
without my involvement. Both are honest.

## Gap / follow-up brief candidates

NEW from scenario 1 (NOT a process-drift finding per se but
elevated severity from this sweep): the pre-merge-gate's
`--skip-clippy` escape hatch is masking RED `argo-skill --lib`
tests on main. See round-41 scenario report
`round-41-worker-skill-overlay-blob-text-join-mismatch-verify.md`
finding §"Brief candidates surfaced this round" (a)
`ops-pre-merge-gate-fix-test-failures-not-just-clippy`.

RE-SURFACED P0 escalation candidate: `mission-cross-tenant-test-audit-export`
needs an explicit conductor-side escalation since its 2-day window
is the tightest remaining on the fix-by tracker and it has gone
**5 rounds without a claim**.
