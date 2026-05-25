# fix-workflow-builder-url-token-and-palette-cu-split

**Owner**: claude.
**Authored against**: origin/main `5c4abb23e1f6` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-workflow-builder-url-token-and-palette-cu-split`.
**Source**: e2e-ux-audit (scene-4-and-5-claim-4-and-5) findings F10 + F11 — `docs/skycot/ux-audits/2026-05-26-0820/scene-4-and-5-claim-4-and-5.md`.

🟢 **P2 — Workflow builder shows `__new__` token in the URL when creating a new workflow + the palette's "Browser action" entry conflates browser automation with desktop control (macOS Notes app per Scene 5 talk track is desktop, not browser).**

## Root cause

Two small workflow-builder polish gaps:

**F10 — `__new__` token visible in address bar**:
`web/app/(app)/workflows/build/page.tsx:44` — clicking "Sales" navigates to `/workflows/sales/__new__/build`. The `__new__` segment is visible in the address bar. The camera doesn't typically record the URL bar for in-app scenes, but if any frame catches it, the engineering-plumbing token reads poorly.

**F11 — palette "Browser action" mislabels desktop control**:
`web/app/(app)/workflows/[verticalId]/[name]/build/types.ts:271-276`:
```ts
{ id: "tool-browser", kind: "tool_call", label: "Browser action", description: "Drive a real browser" }
```

Scene 5's narrated truth is that macOS computer-use drives the **Notes app** (not a browser). The single palette entry conflates browser automation (Playwright / headless) with desktop control (macOS / Windows / Linux substrate). Scene 5 doesn't navigate into the builder during recording — but if the camera ever follows an "Edit this workflow" link, the mismatch is visible.

## Findings this brief closes

- scene-4-and-5-claim-4-and-5 F10 [P2] — Builder canvas blank-vertical entry route uses `__new__` token visible in URL
- scene-4-and-5-claim-4-and-5 F11 [P2] — Workflow-builder "Browser action" palette entry mislabels desktop / macOS control

## Fix recipe

**For F10** (URL token):
1. Touch `web/app/(app)/workflows/build/page.tsx:44` and the route file at `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx`.
2. Options:
   - **Option A**: Route to `/workflows/build/<vertical>` for new-workflow creation (clean — keeps the segment in the URL human-readable). Requires a new route file at `web/app/(app)/workflows/build/[verticalId]/page.tsx` that mirrors the existing builder page but treats name as `undefined`.
   - **Option B**: Use a query param: `/workflows/<vertical>/build?new=1`. Reuses existing route shape.
   - **Option C**: Replace `__new__` with `untitled` (clean enough that it doesn't read as plumbing — operator can later rename inline).
   
   Pick Option C for the smallest patch. `untitled` reads as "a workflow you haven't named yet" — natural enough for the camera if it ever catches it.

3. Update `web/app/(app)/workflows/build/page.tsx:44` to navigate to `/workflows/<vertical>/untitled/build`.
4. Update the route handler at `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx` to treat `name === "untitled"` as the new-workflow state (no preload, blank canvas).
5. When the user saves, rename the URL via `router.replace(`/workflows/${vertical}/${chosenName}/build`)`.

**For F11** (palette CU split):
1. Touch `web/app/(app)/workflows/[verticalId]/[name]/build/types.ts:271-276`.
2. Replace the single `tool-browser` entry with two distinct palette items:
   ```ts
   { id: "tool-cu-desktop", kind: "cu_action", label: "Desktop action", description: "Drive a desktop app (macOS Notes, Excel, Photoshop...)" },
   { id: "tool-cu-browser", kind: "tool_call", label: "Browser action", description: "Drive a real browser (Playwright)" },
   ```
3. Use distinct icons (an app-window icon for desktop, a browser icon for browser).
4. Backing step kinds:
   - `cu_action` — new step kind ships under `fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds`. If that brief hasn't landed yet, scope this brief to renaming only (single entry but with clearer label "Browser action (Playwright)") and defer the desktop entry to the cu-step-kinds brief.
   - `tool_call` — existing
5. Add a small "substrate" picker on the desktop entry when added to the canvas: `macOS · Linux · Windows` toggle (default macOS per `streams/claude-defer-non-macos-computer-use.md`).

## Acceptance criteria

**F10**:
- [ ] `/workflows/build` → click "Sales" → URL bar reads `/workflows/sales/untitled/build` (not `__new__`).
- [ ] After save, URL replaces to `/workflows/sales/<chosen-name>/build`.
- [ ] Existing edit flow (clicking an existing workflow → Edit) unchanged.

**F11**:
- [ ] Palette shows two distinct entries: "Desktop action" + "Browser action" (with separate icons).
- [ ] Desktop entry backs onto `cu_action` step kind (depends on `fix-scene-5-...` brief — if that hasn't landed, scope this part out and document in PR body).
- [ ] Browser entry stays as `tool_call`.
- [ ] Adding a Desktop step to the canvas opens a small substrate picker (macOS / Linux / Windows).

## May-touch

- `web/app/(app)/workflows/build/page.tsx` (route token rename)
- `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx` (handle `untitled` name)
- `web/app/(app)/workflows/[verticalId]/[name]/build/types.ts` (palette split)
- `web/components/skycot/workflow-builder/PaletteItem.tsx` (icons for desktop vs browser)
- `web/components/skycot/workflow-builder/StepPropertiesPanel.tsx` (substrate picker for cu_action)

## Out of scope

- New step-kind authoring for `cu_action` / `mcp_invoke` (covered by `fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds`)
- Multi-substrate support beyond macOS (Phase 2 per `streams/claude-defer-non-macos-computer-use.md`)
- Browser substrate variants (Chrome vs Firefox vs WebKit) — out of scope

## Effort

~50-100 LOC across builder route + palette. ~1-2 hours. Defer the desktop palette wire until `fix-scene-5-...` lands (scope notes in PR body).
