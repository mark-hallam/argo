# fix-workflow-detail-connector-preflight-status-block

**Owner:** claude
**Authored against:** origin/main `2a9911c6828c4b37e9313b32dd024457e31d15b8` on 2026-05-28
**Validate before claim:** `bash scripts/check-brief-freshness.sh fix-workflow-detail-connector-preflight-status-block`
**Source:** UX e2e audit 2026-05-28-0252 (scene-4-and-5 F18)

🟡 **P2 — Workflow detail page has no connector-status block. Workflow steps reference connectors (e.g. `skill: "zendesk fetch thread"`) but the viewer doesn't surface whether the connectors are authenticated for THIS tenant. To know "will Run now actually work?", the operator has to navigate to `/settings/connectors` and check manually. For Claim 5 (Any-PC execution) to feel real, the viewer should preflight: "Zendesk ✅ connected · Notes (cu macOS) ✅ macOS-permissions granted" and gate Run now if anything is missing.**

## Root cause

`web/app/(app)/workflows/[verticalId]/[name]/page.tsx:205-260` renders the workflow meta row (vertical · risk · version · substrate) but does not enumerate the workflow's connector dependencies or check tenant-side auth state. The data is available — `/api/v1/tenant/connectors` (used by `/settings/connectors`) returns the per-tenant connector list — but the workflow viewer doesn't read it.

For Scene 5, the camera lingers on a workflow detail page before clicking Run now. Currently the viewer offers no signal about which connectors the workflow needs or whether they're ready. Run now fires, and if a connector isn't connected, it 5xxs deep in the runtime — the buyer sees a generic error without context.

Sibling `fix-workflow-detail-run-now-disabled-when-source-not-gateway` (IN_REVIEW #2358) handles the "source isn't gateway" disable case. This brief is the additive connector-preflight surface for the live-gateway case.

## Findings this brief closes

- scene-4-and-5 F18 [P2] — Workflow detail viewer has no connector-status preflight block; "Run now" fires without knowing connectors are ready

## Fix recipe

1. **Walk the workflow's steps** and enumerate the connectors referenced. For each step kind:
   - `mcp_invoke` step → connector is the `server` field (e.g. `zendesk`, `notion`).
   - `cu_action` step → connector is the executor app (e.g. `macos_notes`).
   - `skill_invoke` step → may reference an MCP server transitively; resolve via the skill's manifest.
   - `tool_call` step → may reference a connector via the tool's manifest.
2. **Fetch the tenant's connector status** via `GET /api/v1/tenant/connectors` (already used by `/settings/connectors`).
3. **Render a `<WorkflowConnectorPreflightCard>`** between the meta row and the steps list:
   - One row per connector the workflow uses.
   - Each row: connector icon + name + status chip (`✅ connected` / `⚠️ needs reconnect` / `🔴 not connected` / `🔵 permissions pending` for cu_action).
   - Click-through to `/settings/connectors/<id>` for any unhealthy entry.
4. **Gate Run now** if any required connector is unhealthy. Disable the button with a tooltip "Connect Zendesk to run this workflow → /settings/connectors/zendesk".
5. **Empty case**: if the workflow uses zero connectors (pure skill-only with no external dependencies), don't render the card at all.
6. **Vitest + storybook**:
   - `WorkflowConnectorPreflightCard.test.tsx` — covers 3 health states + the no-connectors empty case.
   - Storybook variants: all-green, one-amber, all-red, no-connectors.
7. **Loading state**: while the connector fetch is pending, render a skeleton row per detected connector (no flicker).

## Acceptance criteria

- [ ] Workflow detail page renders a connector preflight card listing every connector the workflow uses.
- [ ] Each connector shows real tenant-side status (✅ / ⚠️ / 🔴 / 🔵).
- [ ] Run now is disabled with a helpful tooltip if any required connector is unhealthy.
- [ ] Workflows with zero connector dependencies render no card (no empty placeholder).
- [ ] vitest + storybook pass.
- [ ] PR body cites scene-4-and-5 F18.

## May-touch

- `web/app/(app)/workflows/[verticalId]/[name]/page.tsx` (~20 LOC card insertion + gate)
- `web/components/skycot/workflow/WorkflowConnectorPreflightCard.tsx` (NEW, ~200 LOC)
- `web/components/skycot/workflow/WorkflowConnectorPreflightCard.test.tsx` (NEW, ~100 LOC)
- `web/components/skycot/workflow/WorkflowConnectorPreflightCard.stories.tsx` (NEW)
- `web/lib/api/connectors.ts` (extend if needed, ~10 LOC)
- `web/lib/workflow/extract-connectors.ts` (NEW helper, ~80 LOC)

## Out of scope

- Adding `mcp_invoke` + `cu_action` to the viewer's step-kind enum (covered by `fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds`)
- Run-now silent-failure fix (covered by `fix-workflow-run-now-silent-failure-no-toast-or-error-state`)
- CSRF wiring on Run-now (covered by `fix-workflow-execute-csrf-token-wiring-verify-before-demo`)
- Connector list itself (covered by /settings/connectors briefs)

## Effort

M — ~200 LOC card + ~80 LOC extractor helper + ~100 LOC tests + storybook + ~20 LOC integration. ~5-7 hours.
