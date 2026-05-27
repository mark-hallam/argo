# fix-workflow-templates-add-mcp-and-computer-use-examples

**Owner:** claude
**Authored against:** origin/main `2a9911c6828c4b37e9313b32dd024457e31d15b8` on 2026-05-28
**Validate before claim:** `bash scripts/check-brief-freshness.sh fix-workflow-templates-add-mcp-and-computer-use-examples`
**Source:** UX e2e audit 2026-05-28-0252 (scene-4-and-5 F16)

🟠 **P1 — `/workflows/templates` ships 50 templates and ZERO of them reference MCP connectors or computer-use steps. `grep "MCP\|mcp_invoke\|cu_action\|computer-use\|cu.run_macro" web/lib/workflow-templates.ts` returns 0 matches across 654 lines. The two step kinds that distinguish Skycot from n8n / Zapier are absent from the template gallery — Claim 5 (Any-PC execution) has no marketing surface on the template marketplace.**

## Root cause

`web/lib/workflow-templates.ts:97-654` defines 50 templates via the `t(...)` constructor. None of the templates uses `mcp_invoke` or `cu_action` step kinds. The 50-template gallery is the operator-facing "showcase of what you can build" surface — and the showcase doesn't surface the differentiator.

Sibling brief `fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds` (BLOCKED) authors the canonical `customer_research_followup` workflow with both step kinds. THIS brief expands the same coverage into the templates gallery so the differentiator appears in 3+ "ready to clone" examples (not just the headline workflow).

The viewer-side ability to RENDER `mcp_invoke` + `cu_action` is the blocker on that sibling brief; once it lands here, the templates render correctly. If we ship template content before the viewer types are extended, the cards land but the detail page can't render the steps — coordinate sequencing.

## Findings this brief closes

- scene-4-and-5 F16 [P1] — Workflow templates have 0 MCP/cu_action examples in 50-template gallery; differentiator absent from template marketing surface

## Fix recipe

1. **Sequencing check**: confirm `fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds` (BLOCKED) is unblocked OR ship this brief gated behind a feature flag that hides the new templates until viewer-side rendering lands. Coordinate via brief comment.
2. **Add 3 net-new templates** to `web/lib/workflow-templates.ts`:
   - **`notion-to-slack-digest`** (MCP connector example): `mcp_invoke notion:search_pages` → `mcp_invoke slack:post_message`. Tagged `MCP · Notion · Slack`. Vertical: marketing.
   - **`linear-to-github-mirror`** (MCP connector example): `mcp_invoke linear:list_issues` → `mcp_invoke github:create_issue`. Tagged `MCP · Linear · GitHub`. Vertical: gtm-engineer.
   - **`notes-summary-handoff`** (computer-use example, macOS-flavoured): `skill_invoke summarise.weekly_runs` → `cu_action macos_notes:create_note`. Tagged `Computer-use · macOS · Notes`. Vertical: project-coordination.
3. **Add the new "MCP" and "Computer-use" filter chips** to `/workflows/templates` filter row so visitors can narrow by differentiator step kind. Wire into the existing filter substrate.
4. **Update the template detail page** (`web/app/(app)/workflows/templates/[templateId]/page.tsx`) to render the new step kinds with their distinctive iconography (Plug icon for MCP, Monitor icon for CU). Coordinate via shared `STEP_KIND_META` if sibling brief landed.
5. **Bump the "50 templates" count to 53** on `/workflows` hero ("Browse 50+ templates" → "Browse 53+ templates" or generalise the count via a `WORKFLOW_TEMPLATES.length` reference).
6. **Vitest**:
   - Extend `web/lib/workflow-templates.test.ts` to assert ≥2 templates with `mcp_invoke` steps and ≥1 with `cu_action`.
   - Grep guard: a CI test that fails if `mcp_invoke` count drops back to 0.

## Acceptance criteria

- [ ] `grep -c mcp_invoke web/lib/workflow-templates.ts` returns ≥2.
- [ ] `grep -c cu_action web/lib/workflow-templates.ts` returns ≥1.
- [ ] `/workflows/templates` renders the 3 new templates with their step-kind chips visible on the card.
- [ ] Template detail page for each new template renders the `mcp_invoke` / `cu_action` step icons + labels (depends on sibling brief landing first).
- [ ] "MCP" and "Computer-use" filter chips work — clicking filters down to the matching templates.
- [ ] vitest + grep-guard pass.
- [ ] PR body cites scene-4-and-5 F16 and notes the dependency on the sibling brief.

## May-touch

- `web/lib/workflow-templates.ts` (+~120 LOC, 3 new templates)
- `web/app/(app)/workflows/templates/page.tsx` (filter chips, ~20 LOC)
- `web/app/(app)/workflows/templates/[templateId]/page.tsx` (step-kind rendering, ~10 LOC)
- `web/lib/workflow-templates.test.ts` (extend)
- `web/app/(app)/workflows/page.tsx` (template-count copy)

## Out of scope

- Authoring the canonical `customer_research_followup` workflow + extending viewer step types (covered by `fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds` BLOCKED)
- Wiring templates to gateway storage (covered by `fix-workflow-templates-page-static-seed-no-gateway-binding` BLOCKED)
- Other template polish — title-suffix, card-author-chip — covered by their own briefs.

## Effort

S/M — ~120 LOC template authoring + ~30 LOC filter chips + tests. ~3-4 hours. Sequence after sibling viewer brief.
