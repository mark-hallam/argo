# Round 31 — Process drift sweep

**Date**: 2026-05-23
**Round**: 31 (scheduled-task hourly fire)
**Pillar / Claim**: Process gates (`check-unread-handoffs.sh`,
`check-findings-only-fix-by.sh`, `check-gateway-freshness.sh`).
**Verdict**: 🟡 PARTIAL — gateway gate fires correctly (UNREACHABLE);
unread-handoffs gate carries 13 stale entries from a heartbeat-clock
quirk; fix-by tracker gap from round 30 **persists** but is on the
conductor's radar per tick-101.

## Setup

Re-ran the three process gates the round-30 sweep examined:

```
bash scripts/check-unread-handoffs.sh
bash scripts/check-findings-only-fix-by.sh
bash scripts/check-gateway-freshness.sh
```

Also read `docs/skycot/agents/heartbeats/conductor.json` to compare what
the conductor knows vs what the gates see.

## Observed

### Gate 1 — check-unread-handoffs.sh

Reported 13 unread entries spanning evaluator rounds 18–30 + the
supervisor-watchdog 07:14Z + 08:14Z entries. BUT conductor tick-100
(08:11Z) heartbeat says: *"Cleared 26 unread SHARED.md handoffs
(evaluator rounds 9-30 + my ticks 87-99 + supervisor-watchdog)."* And
conductor tick-101 (08:35Z) re-confirms: *"STEP 4.5b cleared 26 unread
handoffs from previous window (rounds 9-30 + supervisor-watchdog)."*

Diagnosis: the script reads the **evaluator** heartbeat
`docs/skycot/agents/heartbeats/evaluator.json`, which was reverted by a
concurrent process during this round (system reminder confirmed the
revert was intentional; another evaluator session owns the file). The
script is checking for entries newer than the heartbeat's
`last_run_utc`, which currently shows `2026-05-23T22:30:00Z` (a
"PM-of-previous-day" timestamp that's inherently odd given the conductor
chain ran 06:55Z → 08:35Z this morning).

Not a real gap — this is a heartbeat-clock quirk between concurrent
evaluator sessions. The conductor isn't being misled; tick-101 saw the
right thing.

### Gate 2 — check-findings-only-fix-by.sh

```
🟢 eval-cron-duty-fires-agent-runner-end-to-end       within budget (2026-05-30, 7 days)
🟢 eval-workflow-execution-runs-real-step             within budget (2026-05-30, 7 days)
🟢 mission-hire-audit-payload-shape-validation        within budget (2026-05-30, 7 days)
🟢 mission-cu-macos-cross-tenant-capability-test      within budget (2026-05-30, 7 days)
🟢 mission-cross-tenant-test-audit-export             within budget (2026-05-25, 2 days)
Tally: 5 within-budget · 0 due-today · 0 overdue · 0 informational · 5 total.
```

All 5 deadlines green. **But the cu-macos P0 blocker is NOT among them**
— it lives at BUILD_STATUS.md line 152 as TODO/zombie, and the tracker
only walks `MERGED-FINDINGS-ONLY` rows (per the script's own docstring:
*"Sweep BUILD_STATUS.md for MERGED-FINDINGS-ONLY rows…"*). The P0 has
been open ~15h with no escalation gate firing.

This is round 30's finding — still unfixed. Conductor tick-101 mentions
the brief candidate `ops-fix-by-tracker-add-p0-cu-macos-blocker`, but
hasn't authored.

**Reframing of the round-30 brief shape**: round 30 suggested either
"add a fix_by field to the existing unblock brief" or "harden the
matcher so any P0 unfixed >24h escalates." Looking at the actual script
this round, the simplest fix is option (c) — **a new
`scripts/check-todo-aging.sh`** that walks TODO rows for `P0 ` tokens
in their status notes and flags any unfixed >24h. The fix-by tracker
should keep doing what it does today; the gap is "P0 TODO that won't
ship soon" not "MERGED-FINDINGS-ONLY past deadline."

### Gate 3 — check-gateway-freshness.sh

```
STATUS: UNREACHABLE  (no response from http://127.0.0.1:7400/health — gateway not running?)
```

Working as intended. Gateway has been DOWN since at least round 25 per
the historical record. Informational — not blocking.

## Anti-theater check

The three gates *are* doing their advertised job:

- Unread-handoffs gate **does** catch new SHARED.md entries; the false
  alarm this round is concurrent-evaluator clock-skew, not a bug in the
  script.
- Fix-by tracker **does** sweep MERGED-FINDINGS-ONLY rows; it correctly
  reports 0 overdue. The cu-macos blocker isn't a tracker false-negative
  — it's an out-of-scope row the tracker was never designed to see.
- Gateway-freshness gate **does** report status correctly.

The system is **not theater** — the gap is in the *coverage of the gate
set*, not in any single gate. A `check-todo-aging.sh` would close the
gap.

## Gap / follow-up brief candidates

1. **`ops-add-todo-aging-gate-script`** (P1, ~30-50 LOC) — author a new
   `scripts/check-todo-aging.sh` that walks `BUILD_STATUS.md` for TODO
   rows whose status note contains `P0`, parses each row's `Authored:`
   or `Claimed:` timestamp, and exits 1 if any P0 has been TODO >24h.
   Conductor's per-tick STEP 4.x sequence calls this script and
   surfaces output identically to the existing fix-by tracker.
   **Supersedes** round-30's `ops-fix-by-tracker-add-p0-cu-macos-blocker`
   brief recommendation — same intent, cleaner shape.

2. **`ops-evaluator-heartbeat-concurrent-write-guard`** (P3, ~20-40 LOC)
   — when two evaluator sessions race on `evaluator.json` (an hourly
   scheduled-task and an interactive session), the older session's
   heartbeat keeps winning the revert. Either (a) use a file lock + a
   `pid` field so the newer session can detect ownership and bail
   cleanly, or (b) move to per-session heartbeats keyed by
   `session_label` and have `check-unread-handoffs.sh` walk the union.
   Cosmetic — doesn't block anything load-bearing.

## Cross-references

- Round-30 process-drift report:
  `docs/skycot/evals/scheduled/2026-05-23/round-30-process-drift-sweep.md`.
- Conductor tick-101 heartbeat acknowledging the brief candidates:
  `docs/skycot/agents/heartbeats/conductor.json`.
- Round-31 P0 cu-macos status (companion report):
  `docs/skycot/evals/scheduled/2026-05-23/round-31-p0-cu-macos-still-red-but-claimed.md`.
