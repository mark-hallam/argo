# Round 30 — process drift sweep

- **Date**: 2026-05-23
- **Round**: 30
- **Pillar/Claim**: Process / operations
- **Verdict**: 🟡 **PARTIAL** — 1 P0 visible, fix-by tracking misses it

## Setup

Per scheduled-task Priority Axis 4: re-run all process-gate scripts
and surface any drift the conductor would normally catch.

```
bash scripts/check-unread-handoffs.sh
bash scripts/check-findings-only-fix-by.sh
bash scripts/check-gateway-freshness.sh
git log --oneline origin/main --since="2 hours ago"
```

## Observed

### Unread handoffs

`scripts/check-unread-handoffs.sh` returns a long list of evaluator
handoffs from rounds 18-29. The script flags any non-conductor
entries since the last conductor heartbeat-flip as "unread." Last
conductor tick was tick-97 at `9eab9fc81`. The script's
`ACTION REQUIRED` summary block fires correctly. **Not drift** — this
is the protocol's normal handoff-detection mechanism working as
intended. The conductor will action these on the next tick.

### Fix-by deadlines

```
🟢 eval-cron-duty-fires-agent-runner-end-to-end           7 days remaining
🟢 eval-workflow-execution-runs-real-step                 7 days remaining
🟢 mission-hire-audit-payload-shape-validation            7 days remaining
🟢 mission-cu-macos-cross-tenant-capability-test          7 days remaining
🟢 mission-cross-tenant-test-audit-export                 2 days remaining

Tally: 5 within-budget · 0 due-today · 0 overdue
```

All five tracked deadlines green. **HOWEVER** —
`fix-cu-macos-permission-revoke-test-add-tenant-worker-fields`
(authored by conductor tick-96 at 2026-05-23T06:55Z, P0, workspace
test-sweep blocker) is **NOT in the tracking sweep**. Either:

- The brief was authored without a `fix_by:` field, OR
- The brief was authored but never landed in the tracked stream
  directory, OR
- The `check-findings-only-fix-by.sh` script's matcher misses this
  brief's filename pattern

The brief is real (round-26 evaluator surfaced it; tick-96 conductor
noted authoring it; my round-30 scenario-1 verified the failure mode
still exists). It just isn't being tracked. See
`round-30-p0-cu-macos-blocker-still-open.md` for the full evidence.

**P0 IMPLICATION**: The process gate that *would* have escalated
this 14h+ unfixed P0 is silent. No fix-by deadline = no
"AWAITING-REBASE escalation" pressure on the operator.

### Gateway freshness

```
STATUS: UNREACHABLE (no response from http://127.0.0.1:7400/health
— gateway not running?)
```

Informational; no Chrome MCP / browser scenarios attempted this
round. Consistent with previous rounds 19+ — gateway has been DOWN
across most recent rounds. Not a regression.

### Recent merges (last 2h)

```
9eab9fc81 conductor: tick-97 heartbeat — 2 stale-DIRTY PRs,
          queue healthy at 33 TODO, no replenish
a2888e801 status: tick-92 — promote 5 LIVE-test briefs to top of
          Phase A (P0 OPERATOR-LOCKED)
```

Two commits since round 29. Tick-92 promoted 5 LIVE-test briefs to
the top of the queue — operator-locked P0s. Tick-97 is a heartbeat
with no work shipped (no PRs merged, queue stable). No new mission
code surface to verify this round.

The five operator-locked LIVE-test briefs presumably include:

- `eval-cron-duty-fires-agent-runner-end-to-end` (fix-by 2026-05-30)
- `eval-workflow-execution-runs-real-step` (fix-by 2026-05-30)
- `mission-hire-audit-payload-shape-validation` (fix-by 2026-05-30)
- `mission-cu-macos-cross-tenant-capability-test` (fix-by 2026-05-30)
- One more not in the fix-by tracker

Worth confirming in a future round which briefs are in the LIVE-test
top-5 vs the fix-by tracker — the alignment may have drifted.

## Anti-theater check

Process-drift scripts return either canned output or shell-level
truth (file lists, grep counts). I am not relying on any test that
could rewrite itself green — these are direct grep/find/SQL outputs.
The P0 cu-macos failure was independently verified by `cargo check
-p argo-executor-computer-use-macos --tests` in
`round-30-p0-cu-macos-blocker-still-open.md`.

## Gap / follow-up brief candidate

**Brief name**: `ops-fix-by-tracker-add-p0-cu-macos-blocker`

**Priority**: P1 (process gate is silent on a real P0 — protocol
defense)

**Scope** (~10-30 LOC, single-file edit):

Either:
1. Add a `fix_by: 2026-05-25` field to the existing
   `streams/fix-cu-macos-permission-revoke-test-add-tenant-worker-fields.md`
   brief so `check-findings-only-fix-by.sh` picks it up, OR
2. Audit the `check-findings-only-fix-by.sh` matcher to ensure all
   P0-tagged briefs are surfaced regardless of fix-by-field
   presence (P0 should be ungovernable-by-absence).

Option 2 is the structurally-better fix — "any P0 unfixed > 24h
should escalate" is a more defensible invariant than "P0s only
escalate if someone remembered to tag fix-by." But option 1 is the
2-line workaround the conductor can ship today.

## Operational

- 33 TODO queue
- 2 stale-DIRTY PRs (per tick-97; not investigated this round)
- Gateway DOWN
- 5 fix-by deadlines all green (does NOT include the silent P0)
