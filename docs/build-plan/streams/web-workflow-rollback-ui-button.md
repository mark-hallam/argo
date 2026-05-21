# web-workflow-rollback-ui-button

**Authored against**: `origin/main` `1617b41ea` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-workflow-rollback-ui-button`.

## Summary

#1967 (`mission-pm-workflow-rollback-cli`) just landed real rollback functionality: `argo workflow def rollback <vid>/<name> <version>` lists versions, diffs them, rolls back. The CLI works end-to-end. Now surface it in the workflow viewer/builder so an operator can rollback in one click instead of opening a terminal.

Pre-condition: **#1967 MERGED ✅** (CLI handlers + gateway endpoint exist).

## Acceptance

- [ ] `/workflows/<vid>/<name>` viewer page surfaces a "Versions" panel showing the last 5 published versions with timestamp + author + 1-line summary.
- [ ] Each version row has a "Rollback to this version" button.
- [ ] Click → confirm dialog "Roll back to v3? Current head is v5, drafted v6 will move forward." → call the gateway endpoint that #1967 wired → success toast + page refresh.
- [ ] Error states: 4xx surfaces field-level error toast, 5xx surfaces red toast with retry.
- [ ] No regression on existing viewer UX (workflow steps still render, navigation works).
- [ ] One Playwright E2E: click rollback → assert new head version matches selected.
- [ ] Locked register: `/workflows` is enterprise-restrained per the locked viewer design.

## May touch

- `web/app/(app)/workflows/[verticalId]/[name]/page.tsx` (viewer page)
- `web/components/skycot/workflows/WorkflowVersionsPanel.tsx` (new)
- `web/lib/api/workflows.ts` (extend with `listVersions` + `rollbackToVersion`)
- `crates/argo-cli/src/handlers/gateway/routes_workflows_versions.rs` (additive endpoint if not already wired by #1967)
- New Playwright spec

## Must NOT touch

- The `/workflows/<vid>/<name>/build` page (settled by tick-22/23).
- The CLI handler from #1967 (just consume the same gateway endpoint).
- `crates/argo-store/src/workflow_definition.rs` (settled).

## Notes

- Sizing: 200–350 LOC.
- Pre-flight: `gh pr view 1967 --json files` to see the canonical handler names from #1967.
- Demo-visible payoff — rollback is the kind of feature procurement reviewers look for.
