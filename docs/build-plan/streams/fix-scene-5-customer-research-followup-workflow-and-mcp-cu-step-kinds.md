# fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds

**Owner**: claude.
**Authored against**: origin/main `0653c478d` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds`.
**Source**: e2e-ux-audit Agent 3 finding F3 (`docs/skycot/ux-audits/2026-05-25-1100-scene-4-and-5-claim-4-and-5.md`).

🔴 **P0 DEMO-GATE — Scene 5's narrated workflow doesn't exist. MCP + computer-use first-class step kinds aren't authored.**

## Root cause

Scene 5 narration requires a workflow named `customer_research_followup` with:
- **Step 1** = MCP call `zendesk:search_tickets`
- **Step 2** = computer-use action `macos_notes:create_note`

Agent 3 verification: `grep -rn "customer_research_followup\|zendesk:search_tickets\|macos_notes:create_note" crates/ web/` → **0 matches**. The 7 workflows that DO render (`post_listing`, `screen_candidate`, `approve_refund`, `qualify_lead`, `triage_ticket`, `morning_pipeline_digest`, `clean_customer_list`) use only step kinds `trigger / skill_invoke / decision / approval / tool_call` — never `mcp_invoke` and never `cu_action`.

The runbook narration ("Step 1 fires through the MCP pool — the activity feed shows the dispatcher receiving `(tenant_id, 'zendesk', 'search_tickets')`") cannot be performed against the current build.

This is the load-bearing Scene 5 evidence beat for Claim 5 (Any-PC execution). Without it, the MCP-first / computer-use story has no demo path.

## Findings this brief closes

- Agent 3 F3 [P0] — `customer_research_followup` workflow + `zendesk:search_tickets` + `macos_notes:create_note` step refs do not exist anywhere in the codebase

## Fix recipe

Three layers; ship as one coordinated brief:

1. **Add `mcp_invoke` + `cu_action` as first-class workflow step kinds** in `crates/argo-workflow/src/step.rs` (or wherever the step-kind enum lives):
   - `StepKind::McpInvoke { server: String, tool: String, args: Map<String, Value> }` — dispatches through the MCP pool with tenant scoping.
   - `StepKind::CuAction { app: String, action: String, args: Map<String, Value> }` — dispatches through the computer-use executor (per `argo-executor-computer-use-macos`).
   - Verify the runtime dispatcher in `crates/argo-runtime/src/workflow_runner.rs` (or equivalent) routes both new kinds.

2. **Author the `customer_research_followup` workflow definition**:
   - Manifest at `crates/argo-vertical-pack/manifests/sales/customer_research_followup.yaml` (or equivalent — verify the pack location).
   - Mirror in `web/lib/sample-workflows.ts` so it renders on `/workflows` even before the gateway seed lands.
   - Author chip: pick a canon roster member (`Mariah` or `Peta` — see `fix-demo-seed-cli-for-real-roster` for the canon list).

3. **Activity-feed rendering**: the SPA's workflow-execute activity panel should display the MCP dispatcher tuple `(tenant_id, 'zendesk', 'search_tickets')` and the CU action call when the workflow runs. Verify `web/components/skycot/workflows/WorkflowRunActivity.tsx` (or similar) handles the new step kinds visually.

## Acceptance criteria

- [ ] `grep -rn customer_research_followup crates/ web/` returns ≥4 matches (manifest + sample-workflow + step impl + test).
- [ ] `grep -rn zendesk:search_tickets crates/ web/` and `macos_notes:create_note` each return ≥1 match.
- [ ] `/workflows` renders a card for `customer_research_followup` (sales vertical, canon author).
- [ ] Clicking Run on the workflow detail page dispatches step 1 through the MCP pool and step 2 through the CU executor.
- [ ] The activity feed shows `(tenant_id, 'zendesk', 'search_tickets')` for step 1 and the CU call signature for step 2.
- [ ] Audit row written for each step with the real worker_id + tenant_id.
- [ ] Integration test pinning the two-step dispatch end-to-end (mock MCP server + mock CU executor are acceptable for CI).

## May-touch

- `crates/argo-workflow/src/step.rs` (new step kinds)
- `crates/argo-workflow/src/runner.rs` (dispatch)
- `crates/argo-runtime/src/workflow_runner.rs` (runtime integration)
- `crates/argo-vertical-pack/manifests/sales/customer_research_followup.yaml` (NEW)
- `web/lib/sample-workflows.ts` (SPA mirror)
- `web/components/skycot/workflows/WorkflowRunActivity.tsx` (activity feed)
- Integration test under `crates/argo-cli/tests/`.

## Out of scope

- MCP pool itself (assumed live per memory `project_v1_strategic_decisions_2026_05_14` — MCP-first integration)
- Computer-use executor itself (assumed live per the macOS-first decision)
- "Run now" label vs "Run workflow" mismatch (separate brief: `fix-workflow-run-button-label-alignment`)

## Effort

~300-500 LOC across crates + manifest + SPA + test. ~4-6 hours.
