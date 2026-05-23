# ops-evaluator-scheduled-task-prompt-write-side-runbook

**Authored against**: origin/main `9d9f8fd40` on `2026-05-24`.
**Validate before claim**: bash scripts/check-brief-freshness.sh ops-evaluator-scheduled-task-prompt-write-side-runbook

## Why this brief exists

PR #2150 (`ops-heartbeat-watchdog-reject-future-timestamps`) shipped the *read-side* defence: any heartbeat row stamped >5 minutes ahead of `now_iso` is rejected from dedupe and labelled `⚠️ FUTURE-DATED`. That closes the silent-permanent-wrong-time class of bug at the watchdog.

The *write-side* gap is still open: the evaluator's scheduled task at `~/.claude/scheduled-tasks/skycot-evaluator-hourly/SKILL.md` (operator-actionable, NOT repo-tracked) wrote `last_run_utc: "2026-05-23T23:30:00Z"` when real UTC was `13:27Z` — stamping Melbourne local time as `Z`. PR #2150's body explicitly flagged this as out of scope:

> Per brief's hands-off: any agent's write-side surface. The scheduled-task prompt at `~/.claude/scheduled-tasks/skycot-evaluator-hourly/SKILL.md` is operator-actionable (NOT repo-tracked), so a separate `ops-evaluator-scheduled-task-prompt-use-flavoured-heartbeat-path` brief covers that. This PR is purely the read-side defence.

Evaluator round-37 (commit `9b07a2ce6` on 2026-05-24) confirmed: "heartbeat WRITE-side gap STILL OPEN (round-36's 2 P1 briefs unauthored, race observed 7th consecutive round)". Seven rounds in a row is a process-drift signal — operator may not know the gap was promised but the brief was never written.

The fix is documentation: this brief produces a `docs/skycot/agents/heartbeats/WRITE_SIDE_RUNBOOK.md` that documents the gap, the operator-actionable remediation (use `date -u +"%Y-%m-%dT%H:%M:%SZ"` everywhere — never `date +"%Y-%m-%dT%H:%M:%SZ"`), the failure mode it produces (negative AGE, silently-permanent post-#2139), and the read-side rejection now in place (post-#2150) so the operator knows the wrong-time mode is now self-detecting at the next watchdog run.

## Acceptance criteria

1. Create `docs/skycot/agents/heartbeats/WRITE_SIDE_RUNBOOK.md` (~30-80 LOC).
2. Document explicitly:
   - The gap: scheduled-task prompts that stamp local time as Z produce future-dated heartbeats.
   - The fix every operator-edited prompt must apply: use `date -u +"%Y-%m-%dT%H:%M:%SZ"` (the `-u` flag is the critical bit).
   - Concrete examples of right vs wrong: paste a one-liner that produces wrong-time mode and the one-liner that fixes it.
   - The read-side defence now in place (#2150) — any wrong-time row is now rejected within 5 minutes of the next watchdog run.
   - The list of operator-actionable scheduled-task prompt files at `~/.claude/scheduled-tasks/skycot-{evaluator,conductor,auditor,marketing,personal-site}-*/SKILL.md` (these are NOT repo-tracked but the runbook can name them so the operator knows where to look).
3. Cross-link from `docs/skycot/agents/heartbeats/README.md` (the existing README that #2150 already extended with "Future-dated rejection").
4. Cross-link from `docs/build-plan/HOST_SETUP.md` (the runbook shipped by #2135 / `ops-host-setup-runbook-path-canary`).
5. Add a short note to the conductor's tick checklist (`docs/build-plan/PROTOCOL_LOOP_PROMPT.md` step 4.6 heartbeat refresh) reminding that any scheduled-task SKILL.md the conductor authors or edits must use the `-u` flag.

## Parallel-surface declarations

- No new code, no new pub surface. Docs-only.
- Related docs already shipped: `docs/skycot/agents/heartbeats/README.md` (PR #2150), `docs/build-plan/HOST_SETUP.md` (PR #2135). This brief extends the operator-runbook chain.

## May-touch

- `docs/skycot/agents/heartbeats/WRITE_SIDE_RUNBOOK.md` (NEW)
- `docs/skycot/agents/heartbeats/README.md` (cross-link only)
- `docs/build-plan/HOST_SETUP.md` (cross-link only)
- `docs/build-plan/PROTOCOL_LOOP_PROMPT.md` (one-line reminder addition near step 4.6)

## Must NOT touch

- `~/.claude/scheduled-tasks/*/SKILL.md` — these are operator-actionable, NOT repo-tracked. Document the fix; don't apply it.
- `scripts/check-agent-heartbeats.sh` — read-side defence already in place via PR #2150.
- Any Rust code.

## LOC budget

~30-80 LOC docs (fast-mode candidate — pure documentation).

## Priority

🟢 P2 OPS docs. Seven evaluator rounds carrying this gap as "still open" is a process-drift signal worth closing.

## Source

PR #2150 (`ops-heartbeat-watchdog-reject-future-timestamps`) body, "Out of scope" section. Re-flagged in evaluator round-37 (`9b07a2ce6`) "heartbeat WRITE-side gap STILL OPEN".
