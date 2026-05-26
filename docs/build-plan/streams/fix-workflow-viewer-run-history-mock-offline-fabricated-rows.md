# fix-workflow-viewer-run-history-mock-offline-fabricated-rows

**Owner**:
**Authored against**: origin/main `354de50a0c5fdf561125c7e931699fe94daa326d` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-workflow-viewer-run-history-mock-offline-fabricated-rows`.
**Source**: e2e-ux-audit T0739 — `docs/skycot/ux-audits/2026-05-27-0739/scene-4-and-5-claim-4-and-5.md` F8.

🟡 **P1 — `/workflows/[v]/[n]` viewer's run-history table renders fabricated rows whenever the gateway 404s on the workflow row (which it does in demo-mode today). The page renders a "DEMO RUN HISTORY" pill (honest dressing) but the rows themselves are theatre. Per the function-vs-dressing rule: if the camera drills into the run history during Scene 5 audit pass-through, the camera reads fabricated rows. Currently rated P1 because Scene 5 narrates `/audit` (not the workflow-local run-history table) as the audit destination — but the table sits on the same page the camera DOES sweep.**

## Root cause

`web/app/(app)/workflows/[verticalId]/[name]/page.tsx:104-106`:

```ts
const isDemoRunHistory = source === "mock-offline" || source === "mock-bypass";
```

When the gateway returns no workflow row, the SPA falls through to seed and the run-history table renders fabricated rows (from `web/lib/sample-workflows.ts` or similar). The "DEMO RUN HISTORY" pill IS honest about the data source, but the table contents themselves are pure dressing — synthetic timestamps, synthetic durations, synthetic outcomes.

This compounds with the F3-cluster gateway-empty problem: as long as the gateway is empty, the run-history table is always falling back. After the gateway seed lands, the rows become real — but until then, every workflow detail page surfaces theatre.

The `/audit` feed has the cleaner pattern: when gateway returns 0 rows, the SPA renders the "Quiet day" empty state — no fabricated rows. The workflow-viewer should mirror that pattern.

## Findings this brief closes

- scene-4-and-5-claim-4-and-5 F8 [P1] — `/workflows/[v]/[n]` viewer's run history pulls from `mock-offline` source whenever gateway 404s

## Fix recipe

1. **Suppress the run-history table when the source is `mock-offline` or `mock-bypass`.** In `web/app/(app)/workflows/[verticalId]/[name]/page.tsx` near line 104-106, when `isDemoRunHistory === true`, replace the table render with an empty-state component reading:
   ```
   No runs yet · This workflow hasn't been triggered in this workspace.
   ```
   Mirror the visual treatment of the `/audit` "Quiet day" empty-state — same component if possible (`AuditEmptyState` or equivalent).
2. **Keep the "DEMO RUN HISTORY" pill behaviour** — it's honest dressing that signals data-source state. But don't render fabricated rows under it.
3. **Add a `Run now →` CTA** in the empty-state to keep the user moving forward. Routes to the existing Run-now affordance at `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:263` (the `<Play size={13} /> Run now` button).

## Acceptance criteria

- [ ] When the gateway returns no workflow row for `/workflows/sales/qualify_lead`, the page renders an empty-state for the run-history section (NOT fabricated rows).
- [ ] The empty-state copy reads "No runs yet · This workflow hasn't been triggered in this workspace." (or operator-approved equivalent).
- [ ] A `Run now →` CTA appears in the empty-state.
- [ ] When the gateway returns real workflow rows, the table renders the real rows (no regression).
- [ ] Playwright pin: walk to a workflow detail page in mock-offline mode, assert the run-history table is absent AND the empty-state is present.

## May-touch

- `web/app/(app)/workflows/[verticalId]/[name]/page.tsx` (render branch)
- `web/components/skycot/workflow/WorkflowRunsTable.tsx` (no change expected; just don't mount)
- `web/components/skycot/EmptyState.tsx` or audit-equivalent (no change expected; just consume)
- `web/app/(app)/workflows/[verticalId]/[name]/__tests__/page.test.tsx` (pin)

## Out of scope

- The workflows API empty list (covered by `fix-gateway-skills-and-verticals-boot-seed-tenant-resolution.md`)
- The Scene 5 customer_research_followup seed (covered by `fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds.md`)
- The Run-now CSRF / 403 risk (covered by `fix-workflow-execute-csrf-token-wiring-verify-before-demo.md` + `fix-workflow-execute-demo-principal-capability-grant.md`)

## Effort

S — ~40-60 LOC + pin. ~1 hour. P1 because the camera sweeps this surface during Scene 5.
