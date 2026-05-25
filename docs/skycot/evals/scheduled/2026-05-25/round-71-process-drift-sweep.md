# Round 71 — Scenario 3 — Process-drift sweep + ω-r68 supervisor-watchdog RECURRENCE-4

- **Date**: 2026-05-25 (UTC)
- **Round**: 71
- **Pillar/Claim**: Process / operational hygiene
- **Verdict**: 🟡 **PARTIAL** — halt/findings/drift all 🟢 (**15th** consecutive round r57→r71), BUT ω-r68 supervisor-watchdog escalation now at RECURRENCE-4 (was 3 at r70) + 6 hung scheduled-task processes carry from r70 (still unswept) + z-r66 6th-round ghost-handoff cosmetic skip

## Setup

```bash
$ python3 scripts/find-stale-claim-drift.py --quiet ; echo $?
Found 0 drift rows (none)
0

$ test -f /tmp/argo-stop && echo PRESENT || echo ABSENT
ABSENT
$ test -f ~/.argo/stop && echo PRESENT || echo ABSENT
ABSENT

$ bash scripts/check-findings-only-fix-by.sh
🟢 eval-cron-duty-fires-agent-runner-end-to-end  within budget (2026-05-30, 5 days remaining)
🟢 eval-workflow-execution-runs-real-step  within budget (2026-05-30, 5 days remaining)
🟢 mission-hire-audit-payload-shape-validation  within budget (2026-05-30, 5 days remaining)
🟢 mission-cu-macos-cross-tenant-capability-test  within budget (2026-05-30, 5 days remaining)

Tally: 4 within-budget · 0 due-today · 0 overdue · 0 informational · 4 total.
```

All baseline sweeps clean. **15th consecutive clean round** for halt-sentinel + drift + findings-budget (r57→r71).

## Observed — ω-r68 watchdog escalation

`docs/skycot/agents/SHARED.md` last 4 watchdog entries (20:06:06Z):

```
[supervisor-watchdog] sup=PID:68295/rc:0 slots=2/3 s1=MISSING-5h s2=ACTIVE-noclaim
                      pulses=0/0-stale claude_pids=0/0hang remediated=none
                      drift=0 streak=4 quota_probe=skipped
[supervisor-watchdog] 🚨 ALERT slot-1 MISSING: last worker rc=0 at 01:02:33Z
                      (~5h ago) and supervisor has not relaunched. PID 68347
                      only manages slot-2 (worker-loop PID 16795). Expected
                      supervisor tree: parent+2 children; actual: parent+1 child.
                      Operator: tail /tmp/supervisor.log and check whether slot-1
                      subshell exited or supervisor needs reload.
[supervisor-watchdog] 🚨 ALERT 6 prior scheduled-task watchdog runs HUNG in step 4b
                      eval blocks (zsh PIDs 2373/14319/47534/58362/86635/93418,
                      ages 1h-6.9h). Outside this watchdog's remediation allowlist.
                      Operator can sweep with:
                      kill -KILL 2373 14319 47534 58362 86635 93418.
[supervisor-watchdog] 🚨 ALERT 4 consecutive degraded watchdog runs — operator
                      intervention requested
```

`/tmp/argo-watchdog-state`:
```
degraded 2026-05-25T17:08:13Z
degraded 2026-05-25T18:06:01Z
degraded 2026-05-25T19:07:55Z
degraded 2026-05-25T20:06:06Z
```

4 consecutive degraded watchdog runs. The watchdog has now requested operator-intervention 4 times. No conductor tick has authored `fix-worker-supervisor-respawn-after-rc-0` (the ω-r68 brief) and no operator action has been taken. **RECURRENCE-4** (was 3 at r70, 2 at r69, 1 at r68).

Supervisor.log corroborates: slot-1's last activity was `[01:02:33] slot-1: worker exited rc=0 after 1991s` — successful 33-min run on 2026-05-26 01:02 local = 15:02:33Z. After that, NO further slot-1 launch lines. The bug is that the supervisor doesn't respawn after rc=0 (intended exit) — only after rc=1+ (failure). Slot-1 has been MISSING ~5h.

Slot-2 IS active (last launch `[03:17:27] slot-2 → /tmp/claude-worker-supervised-slot2-20260526T031727.log` = 17:17:27Z; child worker PID 16795 still alive at start-of-round).

## Anti-theater check

1. **Drift sweep prove-it**: ran `find-stale-claim-drift.py --quiet` directly with exit-code check — did NOT trust any heartbeat's text claim of "drift=0." Output `Found 0 drift rows (none)` is the script's own assertion against the live ledger.

2. **Halt sentinel prove-it**: `test -f` on both paths, not by reading any agent's claim of "ACTIVE." Direct filesystem check.

3. **Supervisor.log prove-it**: read raw `/tmp/supervisor.log` tail directly (not via watchdog summary) — confirmed (a) slot-1 last activity 01:02:33 local, (b) slot-1 in extended respawn-loop pattern with `rc=1` flapping prior to the rc=0 termination, (c) slot-2 still respawning in 03:14-03:17 range. The watchdog's "slot-1 MISSING-5h" claim is grounded in the supervisor's own log.

4. **Findings budget prove-it**: ran the script live; output shows 4 within-budget items with explicit `2026-05-30` deadlines, not memorized from prior rounds.

5. **Recurrence-N tracking**: confirmed by comparing to prior heartbeats — r68 was 1 (initial finding), r69 was 2, r70 was 3, r71 is 4. Brief still UNAUTHORED.

## Gap / follow-up

- **🟢→🟡 RECURRENCE-4 ω-r68 carry — author `fix-worker-supervisor-respawn-after-rc-0`**: 4-consecutive-degraded watchdog runs is the loudest "operator intervention required" signal the supervisor produces. The carry has been escalating for 4 rounds with no brief authored. **Recommend conductor T20-30 (or next interactive operator presence) ACTIONS one of**:
  - (a) Author the brief — supervisor respawn-on-rc=0 should be a ~30-LOC fix in `scripts/worker-supervisor.sh` (add `|| true; sleep 30; relaunch` after the rc=0 path).
  - (b) Reload the supervisor LaunchAgent (operator-only lever) to relight slot-1 immediately — `launchctl kickstart -k gui/$(id -u)/com.skycot.worker-supervisor`. This is a one-time fix that doesn't prevent recurrence but unblocks now.
  - **Severity P2** (one worker slot lost ≠ swarm-blocking; slot-2 still active).
- **6 hung scheduled-task processes carry from r70**: still unswept; the watchdog explicitly excludes itself from killing peer scheduled-task processes. Operator-only sweep: `kill -KILL 2373 14319 47534 58362 86635 93418`. **Severity P2** — incremental memory leak, not blocking.
- **z-r66 6th-round ghost-handoff cosmetic skip**: still skipped per r70's call (cosmetic, no behavior impact).
- **θ-r70 ghost-stamped-tick detection**: still UNAUTHORED, but RECURRENCE-1 (no new occurrence this round — T19-30 was honest in its heartbeat). **DOWNGRADE to P3** unless it recurs.

## Recommendations for next conductor tick

In priority order:

1. **🟢 P1 MERGE #2334** (5-round carry-closer: prevents future schema-version drift; PRE-MERGE PASS this round)
2. **🟢 P1 MERGE #2335** (5-round carry-closer: distinguishes 503 from UNREACHABLE in `check-gateway-freshness.sh`; PRE-MERGE PASS this round)
3. **🟢 P1 OBSERVE restart-live-gateway.sh outcome** — if next /health probe returns 200, MERGE #2336 + #2337 too. If still 503/UNREACHABLE, escalate.
4. **🟡 P2 NEW ω-r68 author `fix-worker-supervisor-respawn-after-rc-0`** (RECURRENCE-4 — escalate to brief authoring)
5. **🟡 P3 NEW ops brief `ops-document-or-remove-legacy-sql-migrations-directory`** (clarify the two-migration-system pattern observed in S1)
6. **🟡 P4 skip z-r66 ghost-handoff cosmetic** (7th-round cosmetic skip)
7. **🟡 P4 skip θ-r70 ghost-tick** (no recurrence; downgrade)

## Cost

~3K tokens for S3 (script invocations + log tail + watchdog state read).
