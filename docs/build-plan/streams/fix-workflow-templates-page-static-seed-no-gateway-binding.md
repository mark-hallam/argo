# fix-workflow-templates-page-static-seed-no-gateway-binding

**Owner**: claude.
**Authored against**: origin/main `80e149df55f6` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-workflow-templates-page-static-seed-no-gateway-binding`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-1028/whole-site-mvp-claims-coverage.md` F15 [P1] + F16 [P2].

🟠 **P1 — `/workflows/templates` renders from `web/lib/workflow-templates.ts` — a static seed. There is no `useWorkflowTemplates` hook fetching from the gateway. Operators who build a new template in their tenant cannot surface it here. Page comment admits the gap.**

## Root cause

`web/app/(app)/workflows/templates/page.tsx:1-30` imports `WORKFLOW_TEMPLATES` from `@/lib/workflow-templates` as a 50+ entry static array. Line 26-28 comment: "Real binding to gateway-stored templates lands in a follow-up brief." Until then:
- Tenant-authored templates are invisible.
- The seed list is fixed (cannot add the demo-runbook `customer_research_followup` template without a code change).
- For a public-release product this would be misleading; for POC it's acceptable dressing-side but no progress indicator tells the viewer that.

## Code evidence

`web/app/(app)/workflows/templates/page.tsx:1-50` — fully static import.

## Findings this brief closes

- whole-site-mvp-claims-coverage F15 [P1] — workflow templates 100% static seed, no gateway binding
- whole-site-mvp-claims-coverage F16 [P2] — seed missing `customer_research_followup` (subsumed; new seed entry added)

## Fix recipe

1. **Add gateway endpoint** `GET /api/v1/workflows/templates` that returns the registered template catalogue:
   ```json
   {
     "templates": [
       {"id":"customer_research_followup","display_name":"...","vertical_id":"...","steps_summary":[...]}
     ]
   }
   ```
   File: `crates/argo-cli/src/handlers/gateway/routes_workflow_templates.rs` (verify path).
2. **Add `useWorkflowTemplates()` hook** at `web/lib/hooks/use-workflow-templates.ts` that fetches from the gateway. Merge with the existing static seed: gateway-source first, then any seed-only entries not yet promoted. Tag each card with its source.
3. **In `web/app/(app)/workflows/templates/page.tsx`**, replace static import with the hook. On fetch error / empty, fall through to the static seed (preserving current behavior).
4. **Add the `customer_research_followup` template** to the static seed (`web/lib/workflow-templates.ts`) so even mock-offline lands the demo template. Steps: `[trigger, mcp_call(zendesk:search_tickets), computer_use(macos_notes:create_note), notify]`. Depends on `fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds` landing the step-kind support first.
5. **Surface a "Curated · v1 seed" pill** in the page header when the gateway returns 0 templates so the operator knows it's not tenant-bound yet.
6. **Web test** asserts the page renders gateway-fetched templates when the response is non-empty.

## Acceptance criteria

- [ ] `GET /api/v1/workflows/templates` returns the registered catalogue.
- [ ] `/workflows/templates` page hydrates from the gateway when available.
- [ ] Static seed remains as the mock-offline fallback.
- [ ] `customer_research_followup` is in the seed.
- [ ] "Curated · v1 seed" pill shows when gateway returns empty.
- [ ] Web test passes.

## May-touch

- `crates/argo-cli/src/handlers/gateway/routes_workflow_templates.rs` (NEW or extend)
- `web/lib/hooks/use-workflow-templates.ts` (NEW)
- `web/app/(app)/workflows/templates/page.tsx` (consume hook)
- `web/lib/workflow-templates.ts` (add `customer_research_followup`)
- `web/tests/workflow-templates-page.test.ts` (NEW)

## Out of scope

- MCP / computer-use step-kind support (separate brief: `fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds`)
- Per-tenant authoring UI for templates (post-v1.0)
- Template publish flow / versioning (post-v1.0)

## Effort

~120-180 LOC across gateway + hook + page + seed + test. ~3-4 hours.
