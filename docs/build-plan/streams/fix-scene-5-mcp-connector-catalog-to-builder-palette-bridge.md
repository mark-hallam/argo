# fix-scene-5-mcp-connector-catalog-to-builder-palette-bridge

**Owner**: claude.
**Authored against**: origin/main `78f162a5` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-scene-5-mcp-connector-catalog-to-builder-palette-bridge`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2152/scene-4-and-5-claim-4-and-5.md` F5 [P0].

🔴 **P0 DEMO-BLOCKER for Track P + Scene 5 — `/settings/connectors` ships a real 29-connector MCP catalog (`web/app/(app)/settings/connectors/connector-catalog.ts`, 280 LOC, `data-testid="connector-card-airtable"` etc.) but it is NOT wired into the workflow builder. The builder palette at `web/app/(app)/workflows/[verticalId]/[name]/build/types.ts:236-286` lists only `tool-http / tool-browser / tool-code` — no MCP connectors, no connector-picker, no Computer-use action. Scene 5's "operator picks an MCP server and a computer-use action" beat cannot be assembled from the canvas.**

## Root cause

Two layers, both broken:

1. **Builder `StepKind` enum is incomplete.** `web/app/(app)/workflows/[verticalId]/[name]/build/types.ts:22-34` enumerates `trigger | skill_invoke | approval | tool_call | notify | decision | data | loop | wait | sub_workflow | http_request | code`. There is no `mcp_invoke` and no `cu_action`. The closest match is `tool_call`, which the existing palette item labels "Browser action (Playwright)" — there's no inspector form that lets the operator pick `cu.run_macro` and target `Notes.app`.

2. **The 29-connector catalog is settings-only.** `web/app/(app)/settings/connectors/connector-catalog.ts` exports real connectors (Airtable / Asana / GitHub / Gmail / HubSpot / Jira / Linear / Notion / Slack / etc.) with `data-testid="connector-card-<slug>"`. None of these are reachable from the builder palette. There is no `tool-mcp-airtable` / `tool-mcp-github` entry. The `SIDEBAR_PALETTE_GROUPS` (`types.ts:236-286`) has no "MCP connectors" group.

Together: an operator cannot author a workflow from scratch that uses any of the 29 MCP connectors the platform supports. The builder pretends MCP doesn't exist.

This is a Track P / Scene 5 hard blocker — `project_track_p_v1_mvp_scope` lists Track P inside v1.0 demo scope; P-02 narrates "operator drags an MCP server onto the canvas". Today there's no MCP server to drag.

## Findings this brief closes

- scene-4-and-5-claim-4-and-5 F5 [P0] — Builder canvas has no MCP / computer-use step kinds; the 29-connector MCP catalog is disconnected from the builder

## Fix recipe

1. **Extend `StepKind`** at `web/app/(app)/workflows/[verticalId]/[name]/build/types.ts:22-34`:
   ```ts
   | "mcp_invoke"
   | "cu_action"
   ```
   Mirror in any sibling enum in `crates/argo-workflow/src/step.rs` (verify; may already be done by sister brief `fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds`).

2. **Add a new palette group "MCP connectors"** in `SIDEBAR_PALETTE_GROUPS` at the same file (`types.ts:236-286`):
   - Import `CONNECTOR_CATALOG` from `web/app/(app)/settings/connectors/connector-catalog.ts`.
   - For each connector, emit a palette item with `id: "tool-mcp-<slug>"`, label = `<DisplayName>`, step kind = `mcp_invoke`, default `tool.args` pre-filled with `{ server: "<slug>", tool: "", args: {} }`.
   - Render the connector icon on the palette card.

3. **Add "Computer-use action" to the Tools palette group**:
   - Single palette entry `tool-cu-action` with step kind `cu_action`.
   - Substrate auto-set to `macos`.
   - Inspector form fields: `app`, `action`, `target`.

4. **Build inspector forms** at `web/app/(app)/workflows/[verticalId]/[name]/build/inspectors/`:
   - `McpInvokeInspector.tsx` — `server` (read-only from palette pick), `tool` (free text with autocomplete from connector's tool list if available), `args` (JSON editor).
   - `CuActionInspector.tsx` — `app` (default `Notes.app`), `action` (`run_macro` etc.), `target` (free text).

5. **Persist + emit TOML** — extend `build/proposeToml.ts` (or equivalent) so `mcp_invoke` + `cu_action` steps emit the right TOML stanza matching the seed in `crates/argo-demo-seed/src/workflow.rs`.

6. **Add a Playwright spec** `web/playwright/workflows/builder-mcp-and-cu-steps.spec.ts`:
   - Open `/workflows/{vertical}/untitled/build`.
   - Drag an MCP connector (e.g. Airtable) onto the canvas.
   - Assert a step appears with kind=`mcp_invoke` + server=`airtable`.
   - Drag a Computer-use action; assert step kind=`cu_action` and substrate auto-set to `macos`.
   - Click "Save as draft"; assert the saved workflow round-trips.

## Acceptance criteria

- [ ] Builder palette includes a "MCP connectors" group with at least 10 connectors visible.
- [ ] Builder palette includes a "Computer-use action" item under Tools.
- [ ] Dropping an MCP connector creates a step with `kind: "mcp_invoke"` and the connector slug pre-populated.
- [ ] Dropping a CU action creates a step with `kind: "cu_action"` and `substrate: macos`.
- [ ] Save + publish round-trip the new step kinds without dropping data.
- [ ] New Playwright spec passes.
- [ ] No regression on existing `skill_invoke` / `tool_call` step authoring.

## May-touch

- `web/app/(app)/workflows/[verticalId]/[name]/build/types.ts` (enum + palette)
- `web/app/(app)/workflows/[verticalId]/[name]/build/inspectors/McpInvokeInspector.tsx` (NEW)
- `web/app/(app)/workflows/[verticalId]/[name]/build/inspectors/CuActionInspector.tsx` (NEW)
- `web/app/(app)/workflows/[verticalId]/[name]/build/proposeToml.ts` (TOML emit)
- `web/app/(app)/settings/connectors/connector-catalog.ts` (read-only — verify shape includes the fields needed for palette rendering)
- `web/playwright/workflows/builder-mcp-and-cu-steps.spec.ts` (NEW)
- `crates/argo-workflow/src/step.rs` (verify enum has both kinds; sister brief may already add)

## Out of scope

- Adding the `customer_research_followup` workflow fixture (covered by `fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds`).
- macOS perms preflight wizard (covered by `fix-spa-no-cu-macos-executor-status-surface`).
- Run-now enablement (covered by `fix-workflow-detail-run-now-disabled-when-source-not-gateway`).

## Effort

~600-900 LOC (palette + 2 inspectors + TOML emitter + spec). ~3-5 days. P0 because Track P is in v1.0 scope and Scene 5's "compose MCP + CU from the canvas" beat depends on this.
