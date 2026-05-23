# Write-side runbook — heartbeat timestamps

Companion to `README.md` "Future-dated rejection (5-min grace)"
section (read-side defence, shipped via PR #2150). This document
covers the **operator-actionable** write side: stop the agents
from emitting wrong-time stamps in the first place.

If you only do one thing: confirm every scheduled-task `SKILL.md`
on this host stamps `date -u +"%Y-%m-%dT%H:%M:%SZ"` — never
`date +"%Y-%m-%dT%H:%M:%SZ"`. The `-u` flag is the critical bit.

## The bug

A scheduled-task prompt that calls `date +"%Y-%m-%dT%H:%M:%SZ"`
(no `-u`) interpolates the host's **local** time but stamps it
with a literal `Z` suffix that asserts UTC. On a host in Melbourne
(AEST = UTC+10), 13:27 local gets written as
`2026-05-23T23:30:00Z` — 10 hours in the actual future.

Evaluator round-36 (2026-05-23) observed this race live:

- A concurrent operator-interactive evaluator session wrote
  `last_run_utc: "2026-05-23T23:30:00Z"` when real UTC was 13:27.
- The watchdog's per-session dedupe (from PR #2139) picks the
  lexicographically-largest timestamp.
- The future-dated row won the verdict; the agent showed
  ✅ alive with `AGE = -35735s` (~-10h).

By round-37 the gap had been observed for **seven consecutive
evaluator rounds** without remediation — a process-drift signal
the operator may not have known about because the brief that
should have remediated it (`ops-evaluator-scheduled-task-prompt-
use-flavoured-heartbeat-path`) was promised but never authored.
This runbook closes that chain.

## Right vs wrong (one-liner each)

WRONG — interpolates Melbourne local, stamps `Z`:

```bash
TIMESTAMP=$(date +"%Y-%m-%dT%H:%M:%SZ")
```

RIGHT — `-u` forces UTC, stamp matches the suffix:

```bash
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
```

Single character (`-u`). Catches every tz mismatch
regardless of host timezone.

## Where to apply the fix

The scheduled-task prompts live at:

```
~/.claude/scheduled-tasks/skycot-evaluator-hourly/SKILL.md
~/.claude/scheduled-tasks/skycot-conductor-hourly/SKILL.md
~/.claude/scheduled-tasks/skycot-auditor-3x-daily/SKILL.md
~/.claude/scheduled-tasks/skycot-marketing-weekday-morning/SKILL.md
~/.claude/scheduled-tasks/skycot-personal-site-weekday-morning/SKILL.md
```

These files are **operator-actionable** — they're not tracked in
the repo. The audit + fix is a one-time operator pass:

```bash
grep -lrn 'date +"%Y-%m-%dT%H:%M:%SZ"' ~/.claude/scheduled-tasks/ \
  | xargs -I{} sed -i.bak 's/date +"%Y-%m-%dT%H:%M:%SZ"/date -u +"%Y-%m-%dT%H:%M:%SZ"/g' {}
```

Confirm with `grep -rn 'date -u +' ~/.claude/scheduled-tasks/` —
every match should have the `-u` flag.

## Read-side defence already in place (PR #2150)

The watchdog `scripts/check-agent-heartbeats.sh` rejects any
heartbeat row stamped more than 5 minutes ahead of `now_iso` and
labels it `⚠️ FUTURE-DATED (rejected, >5min ahead of now)`.
Rejected rows count as STALE so the script's exit code reflects
the anomaly.

Behaviour with both defences in place:

| Layer | What happens if a wrong-time stamp lands |
|---|---|
| Write side (this runbook applied) | Stamp written correctly in UTC. No future-dated row. |
| Read side (PR #2150) | If a wrong-time stamp DOES land (rogue script, fresh agent not yet patched), the watchdog rejects it on the next sweep. Wrong-time mode auto-detects within 5 minutes. |

Apply the write-side fix once; the read-side keeps you safe
between writes.

## Conductor tick reminder

Step 4.6 of `docs/build-plan/PROTOCOL_LOOP_PROMPT.md` is the
conductor's "write own heartbeat" point. Any new scheduled-task
SKILL.md the conductor authors or edits MUST stamp
`date -u +"%Y-%m-%dT%H:%M:%SZ"`. The reminder is now inlined at
step 4.6 so the conductor can't author a new agent that
re-introduces this gap.

## Related

- **Read-side defence**: `docs/skycot/agents/heartbeats/README.md`,
  "Future-dated rejection (5-min grace)" section (PR #2150).
- **Host-setup canary**: `docs/build-plan/HOST_SETUP.md` (PR
  #2135) — covers the related case of a host whose PATH lacks
  `claude`. Different gap, similar shape.
- **Split-agent dedupe**: `README.md` "Split-agent convention"
  (PR #2139) — what set up the lexicographic-largest race that
  made wrong-time mode silently permanent.

Author: `ops-evaluator-scheduled-task-prompt-write-side-runbook`,
closing PR #2150 → round-37 follow-up chain.
