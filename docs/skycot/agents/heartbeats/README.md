# Agent heartbeats

Each agent writes a JSON file here at the start of every run. A watchdog (`scripts/check-agent-heartbeats.sh`) reads them and reports staleness.

## Format

```json
{
  "agent": "auditor",
  "last_run_utc": "2026-05-21T13:30:42Z",
  "expected_cadence_seconds": 28800,
  "session_label": "anthropic-claude-code-session-abc123",
  "run_notes": "baseline report written; cron armed; 7 recommendations"
}
```

- `agent`: canonical short name (`conductor`, `auditor`, `marketing`).
- `last_run_utc`: ISO-8601 timestamp the agent started its run.
- `expected_cadence_seconds`: how often this agent is supposed to run. `3600` for conductor (hourly), `28800` for auditor (every 8h ≈ 3×/day), `86400` for marketing (daily, weekday-only — see notes).
- `session_label`: free-text identifier so the operator can tell sessions apart if they restart one.
- `run_notes`: 1-line summary of what this run did. Optional.

## Cadence semantics

The watchdog computes `(now - last_run_utc) / expected_cadence_seconds`. If that ratio is `>1.25`, the agent is considered **stale** (25% grace window past expected cadence).

For the marketing agent (weekday-only), the watchdog ignores weekend gaps — Saturday/Sunday silence is expected.

## Read-only

Agents append-write their own heartbeat at the start of each run. Nothing else reads/writes these files except the watchdog script and the conductor's Step 4.6.

## Split-agent convention (concurrent-write guard)

Some agents run as multiple concurrent sessions — e.g. the **evaluator** can have a scheduled-task instance (`scheduled-task:skycot-evaluator-hourly-round-N`) running alongside an interactive operator session. Both write the same heartbeat path. Last-writer-wins overrides the newer timestamp with whichever session finished writing last, making the heartbeat appear stale (or future-dated) to the watchdog.

Fix (per `ops-evaluator-heartbeat-concurrent-write-guard`, evaluator round-31 SHARED.md handoff):

- Each session writes its OWN per-session file under a stable naming convention: `<agent>-<session-tag>.json`. For the evaluator: `evaluator-scheduled.json` for the cron-fired instance and `evaluator-interactive.json` for the operator session.
- The watchdog (`scripts/check-agent-heartbeats.sh`) groups all `<agent>-*.json` files back to the canonical agent name and reports under that name using the **freshest** timestamp across all session files. So two evaluator sessions writing concurrently cannot race each other's timestamps to stale.
- The `SPLIT_AGENTS=(evaluator)` array at the top of the watchdog is the allowlist. Add an agent's base name there + a per-session-tag pattern in the agent's launch prompt to extend the convention.

The base `<agent>.json` file (without a session-tag suffix) is still honoured for backward compatibility — single-session agents (conductor, auditor, marketing, personal-site) don't need the split.

### Evaluator role checklist

When launching an evaluator session, write the heartbeat to ONE of:

- `evaluator-scheduled.json` — scheduled-task / cron-fired runs.
- `evaluator-interactive.json` — operator-launched interactive sessions.

Do NOT write to the base `evaluator.json` path from a session that knows itself to be one of the two flavours — it can still be raced by the other. The watchdog will collapse both back to the canonical `evaluator` agent at staleness-check time.
