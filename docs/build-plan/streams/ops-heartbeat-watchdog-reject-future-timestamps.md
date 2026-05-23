# ops-heartbeat-watchdog-reject-future-timestamps

- **Priority**: 🟡 P2 (operational defence; without it, a wrong-time agent monopolises the freshness verdict forever)
- **Owner**: claude
- **Type**: ops / shell-script harden (~10-20 LOC)
- **Fast-mode candidate**: YES (single-file bash edit + 1-2 new test cases in an existing bats/shell test if one exists, else manual verification doc)
**Authored against**: origin/main `ccdff2958` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh ops-heartbeat-watchdog-reject-future-timestamps`.

## Premise

Round-36 evaluator (2026-05-23T13:27Z) observed a live race on `evaluator.json`: a concurrent interactive evaluator session stamped Melbourne local time as `Z` (UTC), producing a `last_run_utc: "2026-05-23T23:30:00Z"` when real UTC at the time was 13:27. The watchdog's per-session-file dedupe picks the **lexicographically-largest** ISO-8601 timestamp (`scripts/check-agent-heartbeats.sh:83-87`):

```bash
# Sort by canonical asc, then by last_iso desc (lexicographic on
# the ISO-8601 timestamp is correct because the format is fixed-
# width). awk's `!seen[$1]++` keeps the first line per canonical
# — which after this sort is the freshest.
sort -k1,1 -k2,2r "$collect_file" | awk -F'\t' '!seen[$1]++' > "$collect_file.dedup"
```

A future-dated timestamp wins forever. The `AGE` column displays `-35735s` (~-10h) and the agent shows ✅ alive even though no real heartbeat has fired in the last hour. **Pre-fix behaviour:** last-writer-wins, the wrong timestamp eventually got corrected by the next real write. **Post-fix behaviour:** dedupe survives the next real write because the future-dated row remains lexicographically-largest, so wrong-time mode is now **silently permanent** until a human notices and edits the file.

This is the same risk class as the round-3 audit-export bug — a contract that "self-corrects under last-writer-wins" stops self-correcting once dedupe lands, but the dedupe assumed input is honest.

## What to do

Add a future-dated-timestamp filter to the dedupe pre-pass at `scripts/check-agent-heartbeats.sh:70-87`. Refuse to dedupe-prefer a `last_iso` more than **5 minutes ahead of `now_iso`** (5min is generous enough for clock drift between writer + reader, tight enough to catch tz mistakes). Label rejected rows `⚠ FUTURE-DATED` in the report so they're visible, then drop them, falling back to the next-freshest sibling.

Suggested approach (sketch — adapt to your style):

1. In the per-file collection loop (lines 72-81), before `printf`'ing into `$collect_file`, compute `cand_epoch` from the candidate ISO (the same BSD/GNU `date -u -j` pattern already used at lines 106-107). If `cand_epoch - now_epoch > 300`, write the row to a `$reject_file` instead of `$collect_file`, with a separate column noting it was future-dated.
2. After the dedupe sort+awk (line 87), iterate `$reject_file` to emit a `⚠ FUTURE-DATED` line for each rejected row in the report (so the operator can see what was dropped).
3. If `$collect_file.dedup` ends up empty for an agent because ALL its sibling files were future-dated, the agent should appear as `🔴 STALE — only future-dated heartbeats available`, NOT ✅ alive on a single future-dated row.

## May-touch

- `scripts/check-agent-heartbeats.sh` — the watchdog script itself.
- `docs/skycot/agents/heartbeats/README.md` — add a 2-3 line note documenting the 5-minute future-dated grace + rejection behaviour (so the operator doesn't think their wall-clock is broken).

Optional (recommended):

- A new shell-test under `tests/` (if a bats/shell-test rig exists; check before adding) that constructs a fake heartbeat dir with one future-dated + one healthy sibling and asserts the healthy one wins. If no test rig exists, document the manual verification recipe inline in the script via a `# Test:` comment block.

## Hands-off

- Any agent's heartbeat-write surface — fixing the write side is `ops-evaluator-scheduled-task-prompt-use-flavoured-heartbeat-path` (operator-actionable, in `~/.claude/scheduled-tasks/`, NOT this brief's scope).
- The cadence semantics (`expected_cadence_seconds`, `5/4` grace window) — unchanged.

## Verification

- Hand-craft a `/tmp/heartbeats-test/evaluator-future.json` with `last_run_utc: "2099-01-01T00:00:00Z"` + a `/tmp/heartbeats-test/evaluator-real.json` with a sane recent timestamp.
- Run the watchdog against the temp dir (or with `HEARTBEAT_DIR` env override if it accepts one — otherwise add one).
- Confirm: report shows ⚠ FUTURE-DATED for the bad row + ✅ alive (real-timestamp) for the canonical evaluator.
- Confirm: a dir with ONLY the future-dated row reports 🔴 STALE for that agent (not ✅ alive on the bogus row).

## Acceptance

- A future-dated `last_run_utc` (>5 min ahead of `now_iso`) is dropped from the dedupe pre-pass.
- The dropped row is surfaced in the report as `⚠ FUTURE-DATED` (operator-visible, not silently swallowed).
- An agent with only future-dated heartbeats reports STALE.
- README gets a 2-3 line note about the new behaviour.

## Provenance

Round-36 evaluator finding (2026-05-23T13:27Z); race observed 6+ consecutive rounds. The dedupe pin already shipped via PR #2139 closes the contract in isolation, but the underlying race still fires because the convention isn't enforced at write-time. This brief closes the read-side gap (reject malformed input); the write-side gap requires operator action on `~/.claude/scheduled-tasks/skycot-evaluator-hourly/SKILL.md` (NOT repo-tracked, so no swarm brief possible).

Related shipped: [[ops-evaluator-heartbeat-concurrent-write-guard]] (PR #2139, dedupe pre-pass, MERGED).
