# SKYCOT agent overview — what's running, where it writes, when it ran last

**Authored**: 2026-05-21 by conductor.
**Purpose**: single page operator can `open` to see what's running and whether each agent is alive. Replaces "is the marketing agent still going?" anxiety.

## Currently active agents

| Agent | Charter | Cadence | Write dir | Heartbeat |
|---|---|---|---|---|
| **Conductor** | `docs/build-plan/PROTOCOL_LOOP_PROMPT.md` | hourly (top of hour) | `docs/build-plan/BUILD_STATUS.md` + brief files + PR merges | `docs/skycot/agents/heartbeats/conductor.json` |
| **Auditor** | `docs/skycot/audit/AUDITOR_CHARTER.md` | 3×/day Melbourne (07:30 / 13:30 / 19:30) | `docs/skycot/audit/scheduled/` | `docs/skycot/agents/heartbeats/auditor.json` |
| **Marketing** | `docs/skycot/marketing/MARKETING_AGENT_CHARTER.md` | 1×/day Mon-Fri 06:00 Melbourne | `docs/skycot/marketing/drafts/` | `docs/skycot/agents/heartbeats/marketing.json` |
| **personal-site** | `docs/skycot/personal-site/CHARTER.md` | 1×/day Mon-Fri 06:30 Melbourne | `docs/skycot/personal-site/drafts/` | `docs/skycot/agents/heartbeats/personal-site.json` |

## Liveness check (Gap 3 — agents can't notice when they die)

Every agent writes a heartbeat file at the start of each run. A single command tells you which are alive:

```bash
bash scripts/check-agent-heartbeats.sh
```

Reports each agent's last-seen timestamp + whether it's overdue (using each agent's expected cadence + 25% grace).

Hooked into the **conductor's hourly tick** at Step 4.6 — if any agent is overdue, the conductor surfaces it at the top of its next summary. So you don't need to manually run the check unless you suspect something.

## Cross-coordination (Gap 2 — agents don't talk to each other)

Each agent reads the OTHER agents' `LATEST.md` files at the start of its run:

- **Conductor** reads `docs/skycot/audit/scheduled/LATEST.md` (already does — Step 4.5).
- **Auditor** reads `docs/skycot/marketing/drafts/LATEST.md` — checks for marketing drafts claiming work that hasn't actually shipped (anti-hype guardrail).
- **Marketing** reads `docs/skycot/audit/scheduled/LATEST.md` (already does, per charter) — sources timely content from the auditor's strategic reads.

Plus a shared 1-line ledger:

- `docs/skycot/agents/SHARED.md` — each agent appends one line per run with format `<UTC timestamp> [<agent>] <one-line what-i-did>`. Append-only. Other agents read the tail (last 50 lines) for context. Acts as a low-friction cross-comm bus without complex inter-agent plumbing.

## Operator feedback loop (Gap 1 — agents over-produce without feedback)

Two signal channels:

1. **Implicit (auto-detected)**: each agent reads its previous outputs at start of run, then greps git log + recent merges for evidence the recommendations / drafts got actioned. The auditor's "Resolved since last" section is this signal. The marketing agent will eventually read a `posted.md` log (see below).

2. **Explicit (operator-updated)**: `docs/skycot/agents/posted.md` — operator-maintained log of what got posted / actioned. Recommended format:

   ```
   ## 2026-05-21
   - posted: drafts/2026-05-21/linkedin-personal-process.md (LinkedIn, 14:30)
   - skipped: drafts/2026-05-21/x-personal-process.md (too dev-jargon-y)
   - actioned: auditor LATEST.md P0 #2 (cookie middleware fix landed)
   - declined: auditor LATEST.md P1 #4 (deferred — not ready)
   ```

   Don't worry about discipline — even sporadic entries give the agents signal. Empty file is fine for week 1.

Both signal channels are **read-only inputs** for the agents — only the operator writes to `posted.md`. The implicit channel auto-corrects without operator effort; the explicit channel sharpens it.

## Cost summary

| Agent | Token estimate per run | Runs/day | $/day |
|---|---|---|---|
| Conductor | ~10K | 24 (hourly) | ~$1-2 |
| Auditor | ~30-60K | 3 | ~$5-15 |
| Marketing | ~15-30K | 1 (weekdays) | ~$0.50-1.50 |
| **Total** | — | — | **~$7-20/day** |

Worth every dollar at pre-launch stage. Budget review post-v1.0-launch.

## Killing an agent

Per each charter's killswitch section. Universal pattern: close the Claude Code session running that agent, or `CronDelete <job-id>` from inside the session.

## Future agents (deferred)

Per the operator's 2026-05-21 strategic conversation:

- **Demo-runtime smoke watcher** (Tier 1A) — daily smoke health report. To set up next if/when operator confirms.
- **PR-review-agent autonomous** (Tier 1B) — currently spawned by conductor on-demand. Could become autonomous if operator wants.
- Customer-success, bug-triage, board-update, investor-update — all deferred until they have signal to feed on.
