# ops-evaluator-skill-md-update-from-runbook

**Authored against:** `origin/main` 14f44211d on 2026-05-24.
**Validate before claim:** `bash scripts/check-brief-freshness.sh ops-evaluator-skill-md-update-from-runbook`.

## Why this brief exists

Surfaced as round-38 P3 finding (evaluator handoff 2026-05-24T15:13Z) following the landing of PR #2155 (`ops-evaluator-write-side-runbook`). The evaluator runbook (`docs/skycot/agents/evaluator-write-side-runbook.md` or similar — claim-time worker confirms) is now the canonical source for evaluator behavior (heartbeat path, write rules, charter delineation), but the evaluator's invocation prompt at `~/.claude/scheduled-tasks/skycot-evaluator-hourly/SKILL.md` still references the older base `evaluator.json` heartbeat path + does not link to the new runbook.

This is a tiny correctness fix: the prompt should point the evaluator at the canonical runbook so future evaluator sessions read it on every fire. ~5-10 LOC.

Operator-actionable nuance: the SKILL.md lives outside this repo (under `~/.claude/scheduled-tasks/`) so the worker can NOT directly land this in a PR against this repo. The brief documents the intended edit and surfaces it for operator action.

## Acceptance criteria

Worker chooses one of two paths.

### Path A (documentation-only, in-repo)

1. Add a section to `docs/skycot/agents/evaluator-write-side-runbook.md` (or wherever the canonical runbook lives) titled "Evaluator SKILL.md prompt template" with:
   - The exact text the operator should paste into `~/.claude/scheduled-tasks/skycot-evaluator-hourly/SKILL.md`
   - A 1-line note: "This file is operator-managed under `~/.claude/`; the runbook is the source of truth — keep them in sync."
   - The flavored heartbeat path (`heartbeats/evaluator-scheduled.json` vs `evaluator-interactive.json`) per #2139.
2. Update `docs/skycot/agents/SHARED.md` write-cadence notes if the runbook references stale paths.
3. PR body explicitly flags this is operator-actionable for the SKILL.md update; worker does not need access to `~/.claude/`.

### Path B (script-only)

1. Add a helper script `scripts/print-evaluator-skill-md.sh` (or similar) that emits the canonical SKILL.md content to stdout, so the operator can `bash scripts/... > ~/.claude/scheduled-tasks/skycot-evaluator-hourly/SKILL.md` to sync.
2. Document the helper in the runbook.

## Parallel-surface declarations

- Docs-only or script-only. No code change.
- No new pub Rust surface.

## May-touch

- `docs/skycot/agents/heartbeats/WRITE_SIDE_RUNBOOK.md` (canonical runbook landed via #2155)
- `docs/skycot/agents/SHARED.md` (if write-cadence notes drift)
- `scripts/print-evaluator-skill-md.sh` (new, optional, Path B only)

## Boundary

- Do NOT attempt to edit `~/.claude/scheduled-tasks/skycot-evaluator-hourly/SKILL.md` directly — that's outside the repo and outside any worker's write surface.
- Do NOT change the evaluator's read-only charter (it's locked); only the prompt-template documentation.

## Sizing

~5-30 LOC (docs + optional helper script). Fast-mode candidate.

**🟢 P3 OPS DRIFT** — closes round-38 evaluator finding (2026-05-24T15:13Z). Low urgency; race vector is minor (SKILL.md drift from runbook). Surface for operator action via PR body note.
