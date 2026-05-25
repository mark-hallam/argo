# fix-workflow-substrate-pill-on-cu-workflows

**Owner**: claude.
**Authored against**: origin/main `5c4abb23e1f6` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-workflow-substrate-pill-on-cu-workflows`.
**Source**: e2e-ux-audit (scene-4-and-5-claim-4-and-5) finding F6 — `docs/skycot/ux-audits/2026-05-26-0820/scene-4-and-5-claim-4-and-5.md`.

🟡 **P1 — No visible computer-use or macOS substrate indicator anywhere in the SPA. Scene 5 narrates "right now you're watching the macOS executor drive the Notes app" with no on-screen reinforcement.**

## Root cause

`rg "cu_macos|computer-use-macos|MacosCu|substrate" web/` returns 0 hits outside node_modules / .next. The workflow detail page at `web/app/(app)/workflows/[verticalId]/[name]/page.tsx` has:
- No substrate chip
- No macOS-executor pill
- No "running on your Mac" indicator
- No live-session preview

Scene 5 talk-track says "you're watching the macOS executor drive the Notes app" — the only on-screen reinforcement is the visible Notes app itself. For an investor demo this is P1 (Notes appearing IS the load-bearing visual), but the absence of a "macOS executor · live" chip on the SPA leaves a gap: the viewer can't connect the SPA they were just on to the desktop substrate. It looks like two unrelated demos rather than one orchestrated story.

For a public release demo this would escalate to P0 — a customer evaluating Skycot's "any-PC execution" claim sees zero in-app affirmation of which substrate is running.

## Findings this brief closes

- scene-4-and-5-claim-4-and-5 F6 [P1] — No visible computer-use / macOS substrate indicator in the SPA

## Fix recipe

1. **Add a `SubstrateChip` component** at `web/components/skycot/workflow/SubstrateChip.tsx`. Props:
   ```ts
   interface Props {
     substrate: "macos" | "linux" | "windows" | "e2b" | "browser" | "headless";
     status: "idle" | "running" | "completed" | "failed";
   }
   ```
   Renders a small pill with an icon (Apple logo for macOS, Linux tux for Linux, Windows logo for Windows, cloud for E2B, globe for browser, no icon for headless) + label + a live-pulse animation when `status === "running"`.

2. **Surface substrate in the workflow detail header** (`web/app/(app)/workflows/[verticalId]/[name]/page.tsx`):
   - When the workflow contains any `cu_action` step (after `fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds` lands), render the chip in the page header
   - Substrate value derived from the workflow's `default_substrate` field on the workflow definition (or computed from the step kinds — macos if any `cu_action` step, headless otherwise)

3. **Surface substrate on the workflow card** (`web/components/skycot/workflows/WorkflowCard.tsx` or similar):
   - Small chip variant in the card meta-row (next to the author + version chips)
   - Same icon + label set, no pulse animation (cards are static)

4. **Surface during live runs** (`web/components/skycot/workflows/WorkflowRunStatus.tsx` if it exists, or in the run-detail view):
   - Live chip with pulse animation while the run is in-flight
   - Per-step substrate badge in the timeline (e.g. "Step 2 · cu_action on macOS · 1.4s")

5. **Add a `substrate` field to the workflow definition schema** at `crates/argo-workflow/src/definition.rs` (or wherever workflow defs live):
   - Default `headless` for non-CU workflows
   - `macos` when any step is `cu_action` on macOS substrate
   - Surface in the gateway response on `GET /api/v1/workflows/<vertical>/<name>`

6. **Storybook variants** at `web/stories/SubstrateChip.stories.tsx`: all 6 substrate values × 4 status values = 24 variants.

## Acceptance criteria

- [ ] New `SubstrateChip` component with 6 substrate variants × 4 status variants.
- [ ] Workflow detail page renders the chip in the header when the workflow contains any cu_action step.
- [ ] Workflow card on `/workflows` index renders a small substrate chip in the meta row.
- [ ] Live-run status view renders the chip with a pulse animation while running.
- [ ] Gateway workflow definition response includes a `substrate` field.
- [ ] For Scene 5 demo: visit `/workflows/sales/customer_research_followup` → header shows "macOS · idle" chip; click Test Run → chip becomes "macOS · running" with pulse.
- [ ] Storybook variants visible.
- [ ] Component test covering the 24 variants.

## May-touch

- `web/components/skycot/workflow/SubstrateChip.tsx` (NEW)
- `web/components/skycot/workflow/SubstrateChip.test.tsx` (NEW)
- `web/app/(app)/workflows/[verticalId]/[name]/page.tsx` (header)
- `web/components/skycot/workflows/WorkflowCard.tsx` (or wherever workflow cards render)
- `web/components/skycot/workflows/WorkflowRunStatus.tsx` (run-detail timeline)
- `web/lib/api/workflows.ts` (type for substrate field on workflow definition)
- `crates/argo-workflow/src/definition.rs` (substrate field on schema)
- `crates/argo-cli/src/handlers/gateway/routes_workflows*.rs` (emit substrate)
- `web/stories/SubstrateChip.stories.tsx` (NEW)

## Out of scope

- Substrate-switcher UI (E2B vs macOS vs Linux toggle) — Phase 2 per `streams/claude-defer-non-macos-computer-use.md`
- Live-cu-session preview (separate scope, larger UI)
- macOS Accessibility / Screen Recording permission grant flow (operator-only, off-SPA)

## Effort

~150-250 LOC including SPA component + gateway schema field. ~3-4 hours.
