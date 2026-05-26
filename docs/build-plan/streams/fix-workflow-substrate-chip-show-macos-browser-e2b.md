# fix-workflow-substrate-chip-show-macos-browser-e2b

**Owner**: claude.
**Authored against**: origin/main `ddf44dfb4` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-workflow-substrate-chip-show-macos-browser-e2b`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2007/scene-4-and-5-claim-4-and-5.md` F6 [P1].

🟠 **P1 — All 7 workflow cards on `/workflows` show "HEADLESS · IDLE" substrate chip. None of the 7 fixture workflows mentions Notes / browser / Playwright / macOS / Linux. For Scene 5's "any-PC execution" voiceover the camera needs to see at least one workflow with a `macOS` or `Browser` or `E2B` chip — currently zero on-camera proof of the multi-substrate capability.**

## Root cause

`web/lib/api/workflows.ts:331-351` text-infers the substrate chip from the workflow's name + description + step text. None of the 7 fixture workflows in `web/lib/sample-workflows.ts:64-141` mentions Notes / browser / Playwright / macOS — all collapse to `headless`. So even though the runtime supports MCP + computer-use substrates, the camera sees a uniform wall of HEADLESS chips.

## Findings this brief closes

- scene-4-and-5-claim-4-and-5 F6 [P1] — All workflow cards show HEADLESS·IDLE; no MCP/CU/browser substrate on display

## Fix recipe

This brief depends on `fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds` landing — once `customer_research_followup` (with `cu.run_macro` tool + macOS Notes wording) is in the SPA-visible workflow set, its substrate chip will infer `macos` from the existing text-matcher.

Until then, the immediate fix:

1. **Add explicit `substrate` field to `WorkflowSummary`** at `web/lib/sample-workflows.ts:34-50`:
   ```ts
   export type WorkflowSummary = {
     // ... existing fields ...
     substrate: "headless" | "macos" | "browser" | "e2b" | "linux" | "windows";
   };
   ```
   Replace text-inference with explicit declaration. Text-inference stays as fallback only for legacy data.

2. **Populate substrate per fixture** with at least one non-headless workflow per substrate kind:
   - `customer_research_followup` → `macos` (Notes.app via cu.run_macro)
   - One new fixture: `competitor_pricing_scrape` → `browser` (Playwright on Chrome)
   - One new fixture: `pdf_invoice_extraction` → `e2b` (sandboxed Python via E2B substrate)
   - Existing 7 → `headless` (no change)

3. **Update gateway response** at `crates/argo-cli/src/handlers/gateway/routes_workflows_list.rs` to include `substrate` in the workflow row (so real-data also carries this; SPA stops guessing).

4. **Update substrate-chip render** at `web/components/skycot/workflows/WorkflowCard.tsx` to use the explicit field with proper colors per substrate (e.g. macOS = silver, browser = blue, e2b = green, headless = gray).

5. **Add a unit test** at `web/components/skycot/workflows/WorkflowCard.test.tsx`:
   - For each substrate value, the chip renders the correct label + color
   - Snapshot test on each substrate appearance

## Acceptance criteria

- [ ] `/workflows` shows ≥3 distinct substrate values across visible cards (not all HEADLESS)
- [ ] At least one card shows `MACOS` substrate (proves Claim 5 cu)
- [ ] At least one card shows `BROWSER` or `E2B` substrate (proves multi-substrate)
- [ ] Gateway response carries substrate field
- [ ] Unit test passes

## May-touch

- `web/lib/sample-workflows.ts` (extend schema + add 2-3 new fixtures)
- `web/lib/api/workflows.ts:331-351` (use explicit field, keep text-infer as fallback)
- `web/components/skycot/workflows/WorkflowCard.tsx` (render with substrate-specific style)
- `crates/argo-cli/src/handlers/gateway/routes_workflows_list.rs` (include substrate)
- `crates/argo-workflow/src/types.rs` (schema)
- `web/components/skycot/workflows/WorkflowCard.test.tsx` (NEW)

## Out of scope

- The MCP/CU step kinds in builder (covered by `fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds`)
- Real workflow execution on the substrates (already shipped per claim-5 runtime)
- New visual treatment for substrate chips (use existing chip patterns)

## Effort

~100-200 LOC. ~3-4 hours. P1 because Scene 5 "any-PC" voiceover needs visual proof.
