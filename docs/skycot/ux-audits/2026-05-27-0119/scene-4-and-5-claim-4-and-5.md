# Scene 4 + 5 e2e UX audit — Skills + Workflows (claims 4 + 5)

**Auditor**: SKYCOT e2e UX agent
**Run timestamp**: 2026-05-27T01:19 local Melbourne
**Scope**: `/skills` marketplace + install flow, `/workflows` `customer_research_followup` template + Run, `/workflows/build` canvas. Verified via curl against gateway + DOM/source inspection. Read-only.
**Environment**: SPA on `:3000`, gateway on `:7400` (schema v83), `GET /api/v1/skills` returns `{skills: [], total_count: 0}`, `GET /api/v1/verticals` returns `[]`.

---

## TL;DR

Two demo-blocking realities collide for Scene 4 + Scene 5:

1. **Scene 4 install flow does not exist.** `/skills` and `/skills/[skillId]` have no "Install" button anywhere in the marketplace surface. The "install a skill" action is actually a per-worker `Try it` button buried inside `/workers/{id}/skills` → Library tab — the runbook's Scene 4 narration ("Click Install on the verified result") cannot be performed as written.
2. **Scene 5 cannot run any workflow.** The `customer_research_followup` workflow the audit prompt names does not exist in seed data OR templates. The closest seed-data workflow with both an MCP-like step and a CU-like step does not exist either. And the `Run now` button on every workflow detail page is hard-disabled when the source is "mock-offline" (which it always is in this dev environment because gateway returns 0 workflows), so the demo cannot click Run at all without first publishing a workflow through the canvas.

Finding count: 4 P0, 5 P1, 3 P2, 2 P3. The biggest blockers are Scene-4-has-no-install-button and Scene-5-has-no-runnable-workflow.

---

## Headline blockers

1. **F1 [P0] — `/skills` and `/skills/[skillId]` have no Install button.** The runbook's "Click Install on the verified result" step cannot be performed. Scene 4 narration breaks.
2. **F2 [P0] — Run-now is disabled on every seed workflow.** `disabled={running || !isLive}` blocks clicks unless the workflow lives in the live gateway. Gateway has zero workflows so every workflow card on `/workflows` has Run disabled, even for the operator. Scene 5 narration ("Hit Run") fails.
3. **F3 [P0] — `customer_research_followup` workflow does not exist.** Not in `web/lib/sample-workflows.ts`, not in `web/lib/workflow-templates.ts`. The runbook spec for Scene 5 (MCP+CU combo) has no implementation. The 50+ templates have no MCP step kind and no computer-use step kind at all.
4. **F4 [P0] — Builder canvas has no MCP step and no computer-use step in the palette.** `SIDEBAR_PALETTE_GROUPS` exposes triggers / skills / logic / approval-gates / tools (HTTP / Browser-Playwright / Code) / notify. There is no way for the operator to build the runbook's "step 1 = MCP call, step 2 = computer-use" workflow on the canvas. Even if the operator wanted to assemble Scene 5 manually, the primitives aren't there.

---

## All findings, in order

### F1 [P0] — `/skills/[skillId]` has no Install button; runbook Scene 4 has no install path

The runbook (`docs/skycot/demos/v1-mission-mvp-runbook.md` line 227) reads: *"Click **Install** on the verified result. Progress strip: 'Verifying provenance… Installing skill… Available to workers.'"* That UI does not exist. `web/app/(app)/skills/[skillId]/page.tsx` renders Provenance + Workers-using sections only; the only buttons on the page are the back link. No "Install" text appears in either file. The real install path is **per-worker** — `Try it` on `/workers/{id}/skills` Library tab calls `POST /api/v1/workers/{workerId}/skills` (`web/lib/api/worker-skills.ts:186`). This is correct architecture (skills attach to workers, not to the workspace), but the runbook narration is wrong.

- **Reproduce**:
  - `open http://127.0.0.1:3000/skills`
  - Click any seed skill card.
  - Land on `/skills/approval.route`. Scan the page for an Install button. Confirm none exists.
- **File**: `web/app/(app)/skills/[skillId]/page.tsx:38-122`
- **Fix**: Either (a) rewrite the runbook to "Install" actually meaning "go to Mariah's worker page, open Library tab, Try it" — and pre-warm the operator that this is the path; or (b) add an `Install on a worker → [worker picker]` button to the skill-detail page.

### F2 [P0] — Run-now button is hard-disabled when gateway has no workflows

`web/app/(app)/workflows/[verticalId]/[name]/page.tsx:105` sets `isLive = source === "gateway"`, and line 256 wires `disabled={running || !isLive}` on the Run-now button. The `useWorkflow` hook falls through to `sample-workflows.ts` whenever the gateway has zero workflows (which is the current state — `curl http://127.0.0.1:7400/api/v1/verticals` returns `{verticals: []}`). Net effect: every workflow surface in the current dev env shows Run-now disabled with the tooltip "Available once this workflow is published to the gateway." Scene 5 ("Hit Run. Step 1 fires through the MCP pool…") is impossible to record without first authoring + publishing the workflow through the canvas.

- **Reproduce**:
  - `open http://127.0.0.1:3000/workflows`
  - Click any workflow card (e.g. `qualify_lead`).
  - Hover the Run-now button — disabled, with tooltip "Available once this workflow is published to the gateway."
- **File**: `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:256`, plus the underlying state at `web/lib/hooks/use-workflows.ts` + the seed-data fallback in `web/lib/api/workflows.ts`.
- **Fix**: For the demo, the operator must seed at least one runnable workflow via the canvas's Save + Publish flow (`POST /api/v1/verticals/{v}/workflows/{n}` + `PUT .../head`) before the recording starts. Document this in the runbook pre-flight, or have `argo demo seed` create a published `customer_research_followup` workflow row.

### F3 [P0] — `customer_research_followup` workflow does not exist in seed or templates

The audit prompt names this workflow explicitly. `grep -rn "customer_research_followup" web/` returns one hit, in a test file (`SubWorkflowPicker.test.tsx`), and nothing in either of the two seed sources (`web/lib/sample-workflows.ts`, `web/lib/workflow-templates.ts`). The closest analogue in the 50-template catalog is `sales-competitor-intel` (`competitor_intel_digest`) which crawls competitor pages and synthesizes a digest — useful as a stand-in only.

- **Reproduce**:
  - `rg "customer_research_followup" web/ | grep -v test`
  - Returns: nothing
- **File**: missing from `web/lib/workflow-templates.ts` (50+ templates), missing from `web/lib/sample-workflows.ts` (7 seed workflows).
- **Fix**: Author a `customer_research_followup` template with one MCP step (e.g. HubSpot search) + one computer-use step (e.g. Notes.app draft) — would also resolve F4. Or rename the Scene 5 workflow in the runbook to one that exists.

### F4 [P0] — Builder palette has no MCP step kind and no computer-use step kind

`web/app/(app)/workflows/[verticalId]/[name]/build/types.ts:236-286` defines the canvas palette. The Tools group exposes only HTTP / Browser-action (Playwright) / Code. There is no `mcp_invoke` kind, no `computer_use` kind, no `macos_notes`-style action. The runbook's Scene 5 promise ("step 1 = MCP call (`zendesk:search_tickets`), step 2 = computer-use action (`macos_notes:create_note`)") cannot be authored on this canvas. The substrate chip on workflow cards has a "cu" / "mcp" surface (see `SubstrateChip`), but no actual step kind backs it.

- **Reproduce**:
  - `open http://127.0.0.1:3000/workflows/sales/untitled/build`
  - Open the left palette → Tools group has 3 items: HTTP request, Browser action (Playwright), Code. No MCP, no Notes.app, no any-Mac-app step.
- **File**: `web/app/(app)/workflows/[verticalId]/[name]/build/types.ts:271-276` (palette item list); `web/app/(app)/workflows/[verticalId]/[name]/build/types.ts:175-188` (step-kind enum).
- **Fix**: Add `mcp_invoke` + `computer_use` to the StepKind union and to the Tools palette group. Block scope is bigger than this audit — flag as a build-plan brief.

### F5 [P1] — Searching "competitor pricing scrape" on `/skills` returns 0 results

The seed catalog `web/lib/api/skills.ts:63-134` has 6 skills (`approval.route`, `invoice.extract`, `candidate.screen`, `deal.qualify`, `support.triage`, `lead.enrich`). None contain the words "competitor", "pricing", or "scrape". The runbook (line 226) says: "Operator searches **competitor pricing scrape**. Three results." On record day the search returns zero, the "Skills my workers learned this month" filter is the only thing populated.

- **Reproduce**:
  - `open http://127.0.0.1:3000/skills`
  - Type "competitor pricing scrape" in the search bar.
  - See "No skills match" empty state.
- **File**: `web/lib/api/skills.ts:63-134`
- **Fix**: Either add a seed `marketplace.competitor-pricing-scrape` skill to SEED_SKILLS or rename the runbook search query to "approval routing" or "invoice extraction" — both of which exist in seed and as TRY chips.

### F6 [P1] — Initial SSR shows `0 skills · 0 shown` then hydrates to 6

`/skills` is a `"use client"` page. Initial HTML returned by `curl http://127.0.0.1:3000/skills` shows the loading state with `0 skills · 0 shown` in the count pill (verified in /tmp/skills snapshot). Once the client query resolves the gateway empty-array response and falls back to `SEED_SKILLS`, the count pill snaps to `6 skills · 6 shown`. The flash is visible on a fresh page load — could be screen-recorded mid-skip on Scene 4 narration.

- **Reproduce**:
  - `curl -s http://127.0.0.1:3000/skills | grep "skills-count"` — confirms `0` SSR initial value
  - Open in browser — see flash from `0` to `6` ~200ms in.
- **File**: `web/app/(app)/skills/page.tsx:113-117` (count pill) + `web/lib/hooks/use-skills.ts:21` (useQuery)
- **Fix**: Either prefetch the seed during build (`getServerSideProps`-equivalent) or hide the count pill until the query is no longer pending.

### F7 [P1] — `GET /api/v1/skills/learned-this-month` returns 404

The hook `useSkillsLearnedThisMonth` calls this endpoint. Gateway responds `{"error":"not found"}` with HTTP 404. The client falls back to `MOCK_LEARNED_SKILL_IDS` = `["support.triage", "lead.enrich"]`, so the filter chip "Skills my workers learned this month" still works visually — but the gateway never serves a real answer for this endpoint. If demo-recording day strict-prod env was set, this would surface as a network error in the console.

- **Reproduce**:
  - `curl -s http://127.0.0.1:7400/api/v1/skills/learned-this-month` → `{"error":"not found"}` HTTP 404
- **File**: gateway-side, not in this scope; fallback path at `web/lib/api/skills.ts:283-295`
- **Fix**: Gateway endpoint needs to land before strict-prod. Out of scope for SPA-only fix.

### F8 [P1] — Workflow run-history shows "Demo workflow · no run history" even on workflows the operator could plausibly run

When `source !== "gateway"`, `isDemoRunHistory` is true and the entire run-history surface is replaced with a static "Demo workflow · no run history. Live run history appears after this workflow is served by the gateway." dashed-border panel (`page.tsx:395-410`). For Scene 5 narration this is fine — but it sits below the disabled Run button, doubling-down on the "this isn't really runnable" signal the camera will see.

- **Reproduce**:
  - `open http://127.0.0.1:3000/workflows/sales/qualify_lead`
  - Scroll past the step list to "RUN HISTORY". See the demo-runs-empty card.
- **File**: `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:395-410`
- **Fix**: Either hide the entire run-history block in demo-mode (just don't render anything below the step list) or rephrase the message to omit "demo workflow".

### F9 [P1] — Workflow detail page shows "version 0.1.0" + "v3" + "32 minutes ago" but the workflow has no real run history

Workflow cards on `/workflows` and the workflow detail header carry meta — versions, last-edited timestamp, last-editor — that all come from `sample-workflows.ts`. The detail page renders the meta verbatim ("version 0.1.0 · 3 versions · 32 minutes ago · by Jamal") but the Run-now button is disabled and run-history says "no run history". A demo viewer drilling into this card sees a mismatch: "this workflow was edited 32 minutes ago and has 3 versions, but the system can't run it and has never run it." This is dressing-side theater for the workflow card itself (acceptable per function-vs-dressing rule), but the runbook DOES navigate into Scene 5 and the camera will read these meta fields.

- **Reproduce**:
  - `open http://127.0.0.1:3000/workflows`
  - Read the meta on each card. The "32 minutes ago / by Jamal" comes from seed.
- **File**: `web/lib/sample-workflows.ts:60-138`
- **Fix**: For a real demo, seed at least one workflow through the gateway (`POST .../workflows/<name>` with TOML, then `PUT .../head`) so the source becomes `gateway` and the meta is real. Document the pre-flight as part of `docs/skycot/demos/v1-mission-mvp-runbook.md`.

### F10 [P2] — Workflow card "lastEditor" cast covers 4 of 5 canonical workers; missing Peta

Seed `lastEditor` values are: Mariah ×2, Priya ×1, Jamal ×3, Nori ×1. Peta (project-ops PM cohort, locked 2026-05-19) authors no seed workflow. Probably intentional — Peta is the PM, not a workflow author — but if Scene 5 narrates "this workflow Peta drafted" the camera will not find one. Function-vs-dressing: dressing-side; not a blocker.

- **Reproduce**:
  - `grep "lastEditor:" web/lib/sample-workflows.ts`
- **File**: `web/lib/sample-workflows.ts`

### F11 [P2] — Runbook references worker "Maya" but Maya does not exist in any seed

`docs/skycot/demos/v1-mission-mvp-runbook.md:229,233,247` refers to a worker named "Maya" for Scene 4. The canonical cast is Mariah / Peta / Priya / Jamal / Nori (memory `project_pm_cohorts_peta_patrick` + `sample-personas.ts:57-129`). Maya is nowhere. Runbook drift, not SPA drift.

- **Reproduce**: `grep -rn "Maya" web/lib/sample-*.ts` — no hits.
- **File**: `docs/skycot/demos/v1-mission-mvp-runbook.md:229,233,247`
- **Fix**: Search-replace Maya → Mariah in the runbook for Scene 4.

### F12 [P2] — Templates gallery has a "Use template" button that opens the canvas but offers no Run path

`/workflows/templates/sales-competitor-intel` (closest stand-in for "competitor pricing scrape") has a Use-template CTA → `/workflows/sales/competitor_intel_digest/build?from_template=sales-competitor-intel`. The operator can author the workflow on the canvas but the chain back to "now hit Run" requires Save + Publish to gateway (a 2-button flow inside the canvas). The runbook doesn't mention this — Scene 5 narration assumes the workflow is already published.

- **Reproduce**:
  - `open http://127.0.0.1:3000/workflows/templates/sales-competitor-intel`
  - Click "Use template". Land on canvas.
  - Look for Run — not visible, only Save / Publish.
- **File**: `web/app/(app)/workflows/templates/[templateId]/page.tsx:69-97`
- **Fix**: Pre-flight script `argo demo seed --tenant <demo>` should publish at least the Scene-5 workflow so the operator never sees the canvas during recording.

### F13 [P3] — TRY chips on `/skills` don't include "competitor pricing scrape"

The TRY chips (`approval routing`, `invoice extraction`, `candidate screening`, `deal qualification`, `support triage`, `lead enrichment`) match the 6 seed skills 1:1. None say "competitor pricing scrape". If the operator goes off-script and clicks a TRY chip, the filter works correctly. Nit — listed for completeness only.

- **File**: `web/app/(app)/skills/page.tsx:23-30`

### F14 [P3] — `/workflows/build` is a vertical-picker landing, not the canvas

The audit prompt says "is the canvas at `/workflows/build` functional or stub". The path `/workflows/build` is intentionally a vertical-picker landing (`web/app/(app)/workflows/build/page.tsx:12-64`); the real canvas lives at `/workflows/[verticalId]/[name]/build`. The canvas IS functional (palette + node DnD + builder canvas + run-overlay + pin-data modal + sub-workflow picker + AI proposal). Not a finding — flagging only because the prompt's wording could be misread as expecting `/workflows/build` to be the canvas itself.

- **File**: `web/app/(app)/workflows/build/page.tsx`
- **Verdict**: Working as intended.

---

## Resolved since last audit

The prior Scene 4+5 audit (`docs/skycot/ux-audits/2026-05-25-1100-scene-4-and-5-claim-4-and-5.md`) was not consulted line-by-line for this run. No explicit "resolved-since" claims.

---

## Orientation score

**42 / 100**

Scene 4 narration cannot be performed as written (no Install button). Scene 5 narration cannot be performed at all without first publishing a workflow to gateway (Run button hard-disabled) AND no MCP-or-CU step type exists in the canvas to author one. The marketplace itself is a clean, brand-loud surface; the workflow detail + canvas surfaces are well-architected. The blocker is data + missing primitives, not visual UX. If the operator pre-seeds a real Scene-5 workflow via the gateway (and rewrites the Scene 4 narration to point at the per-worker "Try it" Library flow), the rest of the surface is fit to record.
