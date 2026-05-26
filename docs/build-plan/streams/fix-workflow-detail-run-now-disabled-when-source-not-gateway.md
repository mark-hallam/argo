# fix-workflow-detail-run-now-disabled-when-source-not-gateway

**Owner**: claude.
**Authored against**: origin/main `80e149df55f6` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-workflow-detail-run-now-disabled-when-source-not-gateway`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-1028/scene-4-and-5-claim-4-and-5.md` F9 [P0].

🔴 **P0 DEMO-BLOCKER — Workflow detail "Run now" button is hard-disabled whenever `source !== "gateway"`. Gateway returns 0 workflows → every workflow falls to seed-source → button always greyed out with "Available once this workflow is published to the gateway." Scene 5 step 2 ("Hit Run") cannot land.**

## Root cause

`web/app/(app)/workflows/[verticalId]/[name]/page.tsx:268-281` renders the run button with `disabled={running || !isLive}` and tooltip "Available once this workflow is published to the gateway." `isLive` is `source === "gateway"` (line 103). With `/api/v1/verticals` returning `{verticals: []}` and `/api/v1/workflows` returning empty, every workflow falls through to the seed source. `isLive` is therefore always false in the recording environment, the Run button is permanently greyed out.

The build-canvas page has its own test-run path (`useTestRun`) but the **detail page** Run button is the one the runbook describes — the natural "Run" entry-point for an operator looking at the canonical workflow. The Operator-locked No-Theater rule (`feedback_no_theater_in_demo`) means the run must actually fire the two backends, so just relaxing the disable gate isn't enough; the seed-source path needs to actually invoke a real handler.

## DOM evidence

- `/workflows/sales/qualify_lead` → "Run now" button greyed out
- Hover tooltip: "Available once this workflow is published to the gateway."
- `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:103` `isLive = source === "gateway"`
- `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:273-280` button disabled binding

## Findings this brief closes

- scene-4-and-5-claim-4-and-5 F9 [P0] — workflow detail "Run now" hard-disabled when source !== gateway

## Fix recipe

This brief depends on `fix-demo-seed-cli-for-real-roster` shipping the gateway-side workflow seed (so `source === "gateway"` becomes true). It also depends on `fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds` ensuring the workflow types exist in the seed.

1. **Confirm the seed CLI populates the gateway with at least the 7 demo workflows** (qualify_lead, customer_research_followup, etc.). If `fix-demo-seed-cli-for-real-roster` is already in flight, no work here.
2. **In `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:268-281`**, relax the disable condition: keep `disabled={running}` always, but allow seed-source workflows to fire a "publish-then-run" mutation:
   - Click "Run now" on a seed-source workflow → silently POST `/api/v1/workflows/<vertical>/<name>` (publish) → on 201, immediately POST `/api/v1/workflows/<vertical>/<name>/runs` (run).
   - Update tooltip when disabled to be more specific: "Run requires a connected provider" rather than "Available once this workflow is published to the gateway."
3. **Show a small "Publishing…" pill during the publish step** before the run-overlay opens.
4. **Audit row from the publish + run must thread the operator's principal** (not the gateway's bypass principal) so the camera-drill-in lands on a real attribution.
5. **Test**: web-side Playwright (or jest) test that clicking Run on a seed-source workflow lands the run-overlay within 2s without showing the "publish to gateway first" tooltip.

## Acceptance criteria

- [ ] On the demo seed (or fresh tenant immediately after seed-CLI run), the Run-now button on every workflow detail page is enabled.
- [ ] Clicking Run-now triggers a real backend round-trip (publish if necessary, then run) and the run-overlay opens with real step state.
- [ ] An audit row is written that the camera can drill into.
- [ ] Web test asserts the Run-now → run-overlay flow against seeded workflows.
- [ ] Tooltip on disabled state (provider missing) reads accurately.

## May-touch

- `web/app/(app)/workflows/[verticalId]/[name]/page.tsx` (relax disable + auto-publish)
- `web/lib/api/workflows.ts` (verify publish + run endpoints exist)
- `web/app/(app)/workflows/[verticalId]/[name]/build/RunOverlay.tsx` (no change expected)
- `web/tests/workflows-run-now.test.ts` (NEW)

## Out of scope

- The seed CLI itself (separate brief `fix-demo-seed-cli-for-real-roster`)
- MCP / computer-use step kinds (`fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds`)
- Run-overlay MCP dispatcher tuple display (covered by Scene 4/5 F12 → folded into the scene-5 brief)

## Effort

~80-120 LOC + 1 test. ~2-3 hours, gated on seed-CLI landing first.
