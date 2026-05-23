# Round 39 — process drift sweep

**Date**: 2026-05-23 (real UTC)
**Round**: 39 [scheduled-task hourly fire, read-only]
**Pillar / Claim**: Process integrity / swarm protocol
**Verdict**: 🟢 PASS on substrate gates; 🟡 P1 heartbeat-write-side gap STILL OPEN (8th-consecutive-round observation)

## Setup

Standard process drift sweep:

- `bash scripts/check-unread-handoffs.sh`
- `bash scripts/check-findings-only-fix-by.sh`
- `bash scripts/check-todo-aging.sh`
- `bash scripts/check-agent-heartbeats.sh`
- `bash scripts/check-gateway-freshness.sh`

## Observed

### Substrate gates — all green

```
✅ No unread handoffs from other agents (cutoff 2026-05-24T15:48:00Z).
🟢 5/5 findings-only fix-by deadlines within budget
🟢 3/3 todo-aging within budget (16h on the 3 brief candidates)
gateway: STATUS: UNREACHABLE (gateway not running — informational)
```

`mission-cross-tenant-test-audit-export` is the closest fix-by deadline — 2026-05-25, **2 days remaining**. Flag for tick-119 conductor sweep.

### Heartbeat watchdog — read-side defence working

```
AGENT            LAST RUN (UTC)          AGE           STATUS
─────  ─────────────────────  ────────────  ──────
auditor          2026-05-23T09:40:00Z    6h54m         ✅ alive
evaluator        2026-05-23T16:30:00Z    4m            ✅ alive
marketing        2026-05-21T20:14:59Z    1d20h         ☀️  WEEKEND (silence expected)
personal-site    2026-05-21T23:24:00Z    1d17h         🔴 STALE (cadence 86400s, grace 25%)
conductor        2026-05-24T15:48:00Z    —           ⚠️  FUTURE-DATED (rejected, >5min ahead of now)

Summary: 3 alive, 2 stale (now=2026-05-23T16:34:01Z)
```

PR #2150's future-dated rejection (5-min grace) is working as designed:
- **My round-39 heartbeat** correctly UTC-stamped via `date -u +"%Y-%m-%dT%H:%M:%SZ"` → `2026-05-23T16:30:00Z` → AGE 4m → ✅ alive.
- **Conductor's last_run_utc** `2026-05-24T15:48:00Z` (Melbourne local Sun May 24 01:48 AEST written as Z) is ~23h in the actual future → correctly labelled `⚠️ FUTURE-DATED (rejected, >5min ahead of now)`.

This means the same Melbourne-local-as-Z write-side bug from round-36 is STILL active on the conductor's scheduled-task SKILL.md prompt — operator hasn't applied the runbook's one-line `sed` fix from `docs/skycot/agents/heartbeats/WRITE_SIDE_RUNBOOK.md`.

## Anti-theater check

The future-dated rejection ISN'T a config-error label; it's a real-state contradiction:
- Real UTC at scan time: `2026-05-23T16:34:01Z` (from `now_iso` in the script output).
- Conductor's stamp: `2026-05-24T15:48:00Z` — ~23 hours ahead.
- 5-min grace is 5 min, not 23 hours — the rejection is correct.

My round-39 stamp `2026-05-23T16:30:00Z` is ~4m BEFORE `now_iso=2026-05-23T16:34:01Z` — within cadence + grace, so labelled `✅ alive` (4m). This proves the watchdog accepts past-dated rows in budget and rejects future-dated ones; it isn't a "newest wins" lexicographic-max bug — it's a real-time-aware verdict.

## Gap / follow-up briefs

**P1 STILL OPEN — heartbeat write-side fix not applied to scheduled-task SKILL.md prompts (8th consecutive round):**

The runbook from PR #2155 (`docs/skycot/agents/heartbeats/WRITE_SIDE_RUNBOOK.md`) prescribes the operator-actionable one-liner:

```bash
grep -lrn 'date +"%Y-%m-%dT%H:%M:%SZ"' ~/.claude/scheduled-tasks/ \
  | xargs -I{} sed -i.bak 's/date +"%Y-%m-%dT%H:%M:%SZ"/date -u +"%Y-%m-%dT%H:%M:%SZ"/g' {}
```

The 5 SKILL.md files listed in the runbook live OUTSIDE the repo (`~/.claude/scheduled-tasks/`), so this is operator-actionable, not conductor-actionable. Round 36 surfaced two evaluator-side P1 briefs (`ops-evaluator-scheduled-task-prompt-use-flavoured-heartbeat-path` + `ops-evaluator-interactive-launch-template-use-flavoured-heartbeat-path`) — both still not applied per round 37's grep verification.

**This round's prompt observation**: my SKILL.md prompt does NOT prescribe `date -u` — it leaves the choice to the model. I used `date -u` because I read the runbook earlier in this session. A less careful model would re-introduce the bug. So even with the runbook landed, the SKILL.md prompts need explicit `date -u` instruction.

Brief candidates (re-surface, no new):

1. **`ops-scheduled-task-prompt-add-explicit-date-u-instruction`** (P1, ~5-10 LOC per SKILL.md file, 5 files) — operator-actionable. Add a 2-line block to each scheduled-task SKILL.md: *"Stamp timestamps with `date -u +"%Y-%m-%dT%H:%M:%SZ"` — the `-u` flag is mandatory; without it, Melbourne local time gets written with a Z suffix and the heartbeat lands ~10h in the future, triggering FUTURE-DATED rejection."* Closes round-36/37/38/39 observation.

2. **`mission-cross-tenant-test-audit-export`** (P0, fix-by 2026-05-25, **2 days remaining**) — re-surface for conductor visibility. Closest fix-by deadline.

3. **`eval-gateway-uuid-bound-routes-tenant-ownership-sweep`** (P1, ~150-200 LOC) — round-33 + 35 + 37 + 38 unauthored. With `mission-approval-decide-request-tenant-pinned` (#2146) and `mission-gateway-session-export-tenant-bind` (queued) now-shipped or pending, a sweep brief would catch any third instance pre-ship.

Gateway DOWN throughout this round — couldn't run browser-driven scenario via Chrome MCP or Playwright. Noted in Operational section: scenario priority axis 3 skipped.

## Operational

- Gateway at :7400 is DOWN per `check-gateway-freshness.sh` UNREACHABLE → no Playwright / Chrome MCP scenarios this round.
- 25th green substrate/ops contract on main this week (counting #2157 + #2156 + #2160 from rounds 38-39).
