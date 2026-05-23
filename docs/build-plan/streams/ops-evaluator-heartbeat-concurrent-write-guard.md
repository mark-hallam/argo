# ops-evaluator-heartbeat-concurrent-write-guard

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `d3929941e` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh ops-evaluator-heartbeat-concurrent-write-guard`.

🟢 **P3 ops hygiene — concurrent evaluator sessions race on heartbeat file.** Source: evaluator round-31 SHARED.md handoff 2026-05-23T08:45:00Z.

> "🟡 P3 ops-evaluator-heartbeat-concurrent-write-guard — concurrent evaluator sessions race on evaluator.json (my session 08:30 timestamp was reverted to 22:30 mid-round by another evaluator session, per system-reminder confirming 'intentional')."

Two evaluator sessions (a scheduled-task one and an interactive one) can run concurrently. Both write `docs/skycot/agents/heartbeats/evaluator.json`. The last-writer-wins race overwrites the newer timestamp with an older one, making the heartbeat appear stale to the supervisor.

## Scope

The heartbeat file at `docs/skycot/agents/heartbeats/evaluator.json` needs either:

(a) **Per-agent suffix** — split into `evaluator-interactive.json` and `evaluator-scheduled.json`, both honoured by the supervisor's freshness check.

(b) **CAS guard** — refuse to write a stale-timestamp; if the on-disk timestamp is newer than the one we're about to write, abort with a warning.

(c) **Append-only log** — write each heartbeat as a new line in `evaluator.log`, supervisor reads the tail.

Recommend (a) — least invasive, least new code. Update `scripts/check-*-freshness.sh` to honour both files.

## Acceptance

- Choose pattern (a) per recommendation above.
- Update the evaluator role doc / startup checklist to write to per-session file.
- Update `scripts/check-evaluator-freshness.sh` (or equivalent) to consider the newest of the two files when computing staleness.
- Add a brief comment in the heartbeat file explaining the split.
- May-touch: `scripts/check-*-freshness.sh`, `docs/skycot/agents/heartbeats/`, evaluator role doc.

## Parallelism
None.

## Done when
PR opens, conductor admin-merges. Optional regression: a shell test asserting both files are read.
