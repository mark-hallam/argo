# mission-workflow-rollback-audit-detail-diff-view

**Authored against**: `origin/main` `842a6e2e4` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-workflow-rollback-audit-detail-diff-view`.

## Summary

#1980 landed `AuditKind::WorkflowRolledBack`. When an operator clicks into the audit row at `/audit/action/<id>` for a rollback event, they currently see a generic action summary. Surface a workflow-rollback-specific diff view that shows: from_version → to_version + the steps that changed (the same diff #1967's CLI produces, but in the UI).

Pre-condition: **#1980 MERGED ✅** + **#1967 MERGED ✅**.

## Acceptance

- [ ] When `AuditRow.kind === "workflow_rolled_back"`, `ActionSummaryHero` renders a custom payload:
  - "Rolled back v5 → v3 by Admin Mark" (consumes `decider_role` from the pending tick-26 reauthored brief, if landed; gracefully degrades if not)
  - 1-line summary: "12 steps changed, 3 added, 5 modified, 4 removed"
- [ ] Expandable "Show diff" section consumes the LCS line-diff from #1967's CLI logic (extract into a shared helper if needed).
- [ ] Diff is read-only — no inline rollback button (the rollback page #1980 owns that).
- [ ] No regression on other audit-kind rendering at `/audit/action/<id>`.
- [ ] One component test covering the rollback-detail render.

## May touch

- `web/components/skycot/audit/ActionSummaryHero.tsx` (extend with rollback-specific branch)
- `web/components/skycot/audit/WorkflowRollbackDiff.tsx` (new)
- `web/lib/diff/workflow-version-diff.ts` (new shared helper if #1967's logic doesn't already export one)
- Test files
- `web/lib/api/audit.ts` (extend if rollback payload needs new fields)

## Must NOT touch

- The `audit_record` schema (read at projection only).
- The audit-page register (locked enterprise-restrained).
- The rollback UI from #1980 or CLI from #1967.

## Notes

- Pre-flight: read #1967's diff implementation in `crates/argo-cli/src/handlers/workflow_def.rs` — see if the LCS helper is reusable.
- Pre-flight: `rg -ln 'ActionSummaryHero' web/components/` for canonical path.
- Sizing: 200-400 LOC.
- Pairs with `web-approvals-decider-role-on-timeline` (tick-26 reauthor) — both consume decider_role from the same audit-feed projection.
- High demo leverage: rollback story becomes audible end-to-end.
