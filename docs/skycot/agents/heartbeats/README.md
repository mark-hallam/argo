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
