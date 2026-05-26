# Scene 4 + Scene 5 (Claims 4 + 5 — Skills shared + learned + Any-PC MCP+CU) — capability UX audit

**Date**: 2026-05-26 20:07 AEST (Melbourne local)
**Auditor**: e2e-ux-journey-test-agent (scope: Scene 4 `/skills` + Scene 5 `/workflows` + `/workflows/build` + `/workflows/templates`)
**Method**: live SPA walk via Claude Preview MCP (Model Context Protocol — Anthropic's standard tool-integration channel) against the running Next.js dev server on `:3000` and the gateway on `127.0.0.1:7400` (both 200 on `/health`); source-code reads where the live page wouldn't hold still; direct `curl` to confirm gateway endpoint shapes. Prior audit at `docs/skycot/ux-audits/2026-05-25-1100-scene-4-and-5-claim-4-and-5.md` used as delta baseline.

---

## TL;DR

Substantial progress since the 2026-05-25 audit — **6 of the 11 prior findings are RESOLVED**. The `WORKFLOWS · MOCK` eyebrow is gone (now reads `WORKFLOWS · v0.1`), the non-canon Rio / Sam author names are gone (now Mariah / Jamal / Nori), `/workflows/build` now ships a real blank-canvas entry point, `/workflows/templates` ships 50 templates faceted by vertical + risk, the SubWorkflowPicker now reads the gateway hook, the Run-now button label was aligned with the runbook, and the skills client now hits `/api/v1/skills` so the CORS (Cross-Origin Resource Sharing — browser rule that lets a webpage talk to a server on a different web address) preflight no longer 404s.

What remains is **3 P0s** and **5 P1/P2s**. The two demo-killers: (1) the `customer_research_followup` MCP+CU combo workflow is defined in the `argo-demo-seed` crate but the running gateway DB doesn't contain it (no `argo demo seed --tenant <demo>` was run) AND the SPA's workflow-catalog only fans out to verticals derived from the local fixture list (`sales / support / gtm-engineer / people-ops / finance`) — so even if the seed ran the card still wouldn't render because the workflow lives under the `marketplace` vertical which the SPA never queries; (2) the `/skills/<id>` detail page has zero Install affordance — the runbook's "Click Install" beat in Scene 4 is impossible there. The runtime path to install a skill exists (`POST /api/v1/workers/<id>/skills` → `[Try it]` button on `/workers/<id>/skills`), but it's buried two clicks away from where the runbook narrates the action.

For Claim 5, the SPA workflow builder canvas cannot express the MCP-substrate or computer-use step kinds at all — `StepKind` enum has 12 variants but no `mcp_invoke`, no `cu_action`, no `computer_use`. The seed crate works around this by emitting a `tool_call` with `tool = "cu.run_macro"`, but the SPA builder has no way to author a `cu.run_macro` step from scratch.

---

## Headline blockers (Scene 4 + Scene 5 unrecordable as scripted)

| # | Severity | Finding | Scene impact |
|---|---|---|---|
| F1 | P0 | `customer_research_followup` defined in seed crate but absent from running gateway DB; SPA wouldn't render it anyway (queries the wrong verticals + falls back to hardcoded fixtures that don't include it) | Scene 5 — the canonical MCP+CU combo workflow is invisible |
| F2 | P0 | "competitor pricing scrape" search beat still returns 0 results | Scene 4 step 2 narrated search beat |
| F3 | P0 | `/skills/<id>` has no Install affordance — runtime path is on `/workers/<id>/skills` `[Try it]` instead, two clicks away | Scene 4 step 3 "Click Install" beat |
| F4 | P0 | `Run now` button on workflow detail page is `disabled` for every fixture workflow (`!isLive`) | Scene 5 — operator cannot click run on any workflow that the SPA actually surfaces |

---

## All findings

### F1 [P0] — Scene 5 MCP+CU combo workflow `customer_research_followup` is functionally orphaned

The combo workflow exists as TOML in `crates/argo-demo-seed/src/workflow.rs:41-64` with three steps: two `skill_invoke` (`zendesk fetch thread`, `summarize for followup`) and one `tool_call` against `cu.run_macro` for Notes.app. The vertical id is `"marketplace"`. The runtime wire is real.

But three independent gaps make it invisible to a Scene 5 recording:

1. **Gateway DB is empty.** `curl -H 'X-Argo-Principal: ...' http://127.0.0.1:7400/api/v1/verticals/marketplace/workflows` returns `{"count":0,"items":[],"vertical_id":"marketplace"}` — `argo demo seed --tenant <id>` was never run against this gateway. The seed crate doesn't auto-fire on gateway boot.
2. **SPA never queries the `marketplace` vertical.** `web/lib/api/workflows.ts:125-131` derives the per-vertical fan-out from `SAMPLE_WORKFLOW_SUMMARIES.vertical` only — and the fixture has 5 verticals (`sales / support / gtm-engineer / people-ops / finance`). `marketplace` is not in that set, so even if `argo demo seed` ran, the SPA call to `/api/v1/verticals/marketplace/workflows` never happens.
3. **SPA fixture fallback doesn't include the combo workflow.** `web/lib/sample-workflows.ts:64-141` ships 7 hardcoded `WorkflowSummary` rows; none of them is `customer_research_followup` and none surfaces an `mcp_invoke` or `cu_action` step kind.

- Reproduction:
  - `grep -n 'customer_research_followup' web/lib/sample-workflows.ts` → 0 matches
  - `grep -n 'marketplace' web/lib/sample-workflows.ts` → 0 matches
  - `curl http://localhost:3000/workflows` and scan all 7 card headings — no Zendesk/Notes/research-followup card present
- File paths:
  - Seed (real): `~/code/argo/crates/argo-demo-seed/src/workflow.rs:30,44`
  - SPA vertical-fanout (orphaned): `~/code/argo/web/lib/api/workflows.ts:125-131`
  - SPA fixture (incomplete): `~/code/argo/web/lib/sample-workflows.ts:64-141`
- Suggested fix: one of (a) author the combo workflow into `SAMPLE_WORKFLOW_SUMMARIES + SAMPLE_WORKFLOW_SPECS` so the SPA renders it via fixture (theater the camera sees, **violates no-theater rule**), or (b) wire `argo demo seed --tenant <id>` into the gateway boot script when `ARGO_DEMO_MODE=1` so the marketplace workflow lands in the DB AND add `marketplace` to `discoverVerticalIds()` so the SPA queries for it (the real fix per the function-vs-dressing rule). Option (b) is also the only path that lets the runbook's "Step 1 fires through the MCP pool" voiceover be true on camera.

### F2 [P0] — "competitor pricing scrape" search still returns 0 results

No regression, no fix from prior audit. Same 6 SEED_SKILLS hardcoded at `web/lib/api/skills.ts:63-134`: `approval.route / invoice.extract / candidate.screen / deal.qualify / support.triage / lead.enrich`. None matches the runbook narration "competitor pricing scrape". Search yields zero. Page shows the count pill `6 skills · 0 shown` (cosmetic mismatch — see F8).

The gateway endpoint `/api/v1/skills` returns `{"skills":[],"tenant_id":"...","total_count":0}` so the SPA fallback to SEED_SKILLS triggers correctly — but the seed catalogue itself is the gap.

- Reproduction:
  - `open http://localhost:3000/skills` → type "competitor pricing scrape" → empty state
- File path: `~/code/argo/web/lib/api/skills.ts:63-134` (SEED_SKILLS)
- Suggested fix: add a `pricing.competitor_scrape` (or `market.scrape`) seed row to `SEED_SKILLS` AND `argo-demo-seed`'s skill seeder (currently absent — see workflow.rs as the only thing the seed crate inserts). For function-side correctness this skill should also be a real `argo skill` library entry, not just a fixture string.

### F3 [P0] — `/skills/<id>` has no Install button — the runtime path is on a different page

The skill detail at `/skills/approval.route` renders Provenance + Workers-using + a back link — nothing else. Zero `<button>` elements in `<main>`. Source: `web/app/(app)/skills/[skillId]/page.tsx` (122 lines, no install/onClick at all).

The runtime install path is real: `POST /api/v1/workers/<id>/skills` exists at `crates/argo-cli/src/handlers/gateway/routes_worker_skills.rs:138-176`, the SPA wires it via `enableSkill()` at `web/lib/api/worker-skills.ts:181-188`, and it's bound to a `[Try it]` button on the `SkillsLibraryCard` at `web/components/skycot/worker/SkillsLibraryCard.tsx:73`. The flow is: navigate to `/workers/<id>/skills`, scroll to the Library section, click `[Try it]` next to the skill.

So Claim 4 IS functionally live (PR #1682 wired `LayeredLookup::with_library` at `crates/argo-agent-runner/src/turn_helpers.rs:71` — verified). What's missing is **the marketplace-side affordance the runbook narrates**: from `/skills/<id>` the operator cannot install with one click — they have to remember which worker they want to install into, navigate to that worker's skills tab, then find the library section. Scene 4 step 3 ("Operator clicks Install on the verified result; progress strip reads Verifying provenance… Installing skill… Available to workers.") cannot be recorded on `/skills/<id>`.

- Reproduction:
  - `open http://localhost:3000/skills/approval.route` → no Install / Add / Try button visible
  - Compare with `/workers/<workerId>/skills` Library section — `[Try it]` exists there
- File paths:
  - Missing affordance: `~/code/argo/web/app/(app)/skills/[skillId]/page.tsx`
  - Wired-up runtime: `~/code/argo/web/lib/api/worker-skills.ts:181-188` + `~/code/argo/web/components/skycot/worker/SkillsLibraryCard.tsx:73`
- Suggested fix: add an `<InstallToWorker>` picker on `/skills/<id>` — dropdown of the current tenant's workers, calls `enableSkill(workerId, skillName)`, shows the three-line progress strip from the runbook. The endpoint and hook already exist; this is purely a missing UI surface (~80 LOC component + 10 LOC of wiring).

### F4 [P0] — `Run now` button is `disabled` for every fixture workflow

On `/workflows/sales/morning_pipeline_digest` (and every other fixture workflow) the `<button data-testid="wfd-run-workflow">` is disabled. Source at `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:256` reads `disabled={running || !isLive}`, where `isLive` is true only when the workflow came from the gateway, not the fixture. Tooltip: "Available once this workflow is published to the gateway".

Since the gateway DB is empty (see F1), no fixture workflow is `isLive` — so EVERY Run now button on the SPA is disabled in demo mode. Scene 5 narration "Operator hits Run now and watches the audit chain" is impossible without seeding the gateway first.

- Reproduction:
  - `open http://localhost:3000/workflows/sales/morning_pipeline_digest`
  - DOM: `document.querySelector('[data-testid=wfd-run-workflow]').disabled` → `true`
- File path: `~/code/argo/web/app/(app)/workflows/[verticalId]/[name]/page.tsx:251-264`
- Suggested fix: either (a) seed at least one fixture workflow into the gateway DB so `isLive` flips true for the demo, or (b) for demo-mode (`NEXT_PUBLIC_SKYCOT_DEMO_MODE=1`) allow the Run now button to fire even on fixture workflows and route to a recorded-execution playback (theater — only acceptable if the camera explicitly DOES NOT drill into the resulting audit row). Option (a) is the no-theater path.

### F5 [P1] — Workflow card author chip doesn't distinguish worker / human / template-pack

The scope brief asks: "Does each workflow card show who authored it (worker / human / template-pack)?" Per `web/app/(app)/workflows/page.tsx:267` and `web/lib/sample-workflows.ts:34-50`, the field is just `lastEditor: string` rendering as `by Mariah` / `by Priya` / etc. No icon, no type chip, no `authorType: "worker" | "human" | "template"` discriminant.

Mariah is a worker, "Priya" could be a worker OR a human teammate (the persona name is shared), and there's no way to tell from the card. For Scene 5 this is a polish gap — the camera will read "by Mariah" but the voiceover claim "workflows can be edited by the AI teammate themself" doesn't have a visual anchor.

- File path: `~/code/argo/web/lib/sample-workflows.ts:34-50` (schema) + `web/app/(app)/workflows/page.tsx:267` (render)
- Suggested fix: extend `WorkflowSummary` with `authorType: "worker" | "human" | "template_pack"` and render an inline icon (Bot / User / Layers) next to the name. ~20 LOC.

### F6 [P1] — All 7 workflow cards show "HEADLESS · IDLE" — no MCP / CU / browser substrate is on display

Per `web/lib/api/workflows.ts:331-351`, the substrate chip is text-inferred from the workflow's name + description + step text. None of the 7 fixture workflows mentions Notes / browser / Playwright / macOS / Linux, so all collapse to `headless`. For Scene 5's "any-PC execution" voiceover the camera needs to see at least one workflow with a `macOS` or `Browser` or `E2B` chip — currently zero.

- Reproduction:
  - `open http://localhost:3000/workflows` → every card's substrate chip reads `HEADLESS`
- File path: `~/code/argo/web/lib/sample-workflows.ts` — needs at least one fixture (or seeded workflow) with `substrate: "macos"` or a step body mentioning the macOS Notes app
- Suggested fix: piggyback on the F1 fix — when `customer_research_followup` lands in the SPA-visible set, it will infer `macos` from `cu.run_macro` tool + macOS Notes wording. Without that workflow visible, the Scene 5 "any-PC" beat has no on-camera proof.

### F7 [P1] — `/workflows/build` blank-canvas builder lacks MCP and computer-use step kinds

The new `/workflows/build` page (resolves prior F8) lets the operator pick a vertical and lands on `/workflows/<vertical>/untitled/build` — a real blank canvas. Good. But the available `StepKind` set at `web/app/(app)/workflows/[verticalId]/[name]/build/types.ts:22-34` is:

```
trigger | skill_invoke | approval | tool_call | notify | decision |
data | loop | wait | sub_workflow | http_request | code
```

There's no `mcp_invoke`, no `cu_action`, no `computer_use`. So a Scene 5 demo of "compose a workflow that combines an MCP tool + a computer-use tool from scratch" on the live builder is not directly authorable — you have to use the generic `tool_call` step and type the right tool name (`cu.run_macro`) by hand, with no inspector help, no MCP-connector picker, no autocomplete of available tools.

For a builder canvas this is a meaningful capability gap given Claim 5 is the headline.

- Reproduction:
  - `open http://localhost:3000/workflows/build` → pick "Sales" → on the canvas, open the step palette → notice no MCP / computer-use category
- File path: `~/code/argo/web/app/(app)/workflows/[verticalId]/[name]/build/types.ts:22-34` (enum); `web/app/(app)/workflows/[verticalId]/[name]/build/builder-handlers.ts:98-105` (defaults)
- Suggested fix: extend `StepKind` with `mcp_invoke` and `cu_action`, add palette entries, add inspector forms for `server / tool / args` (MCP) and `app / action / target` (CU). 2-3 day brief.

### F8 [P2] — "Untitled" workflow build URL redirects to onboarding instead of opening a blank canvas

`/workflows/build` → click "Sales" tile → navigates to `/workflows/sales/untitled/build` → the SPA hydrates, finds no workflow named "untitled" in the gateway, and **redirects to `/onboarding/workspace`** instead of showing the blank-canvas mode. The redirect appears to be a side effect of demo-mode flow combined with the build page's workflow-resolution logic. Source-side, `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx:69` does define `NEW_WORKFLOW_SENTINEL = "untitled"` and the `isNewWorkflow` branch should render the blank-canvas state — but in the live SPA something nudges the route to onboarding instead.

- Reproduction:
  - `open http://localhost:3000/workflows/build`
  - Click "Sales" tile
  - Land on `/onboarding/workspace` ("Name your workspace") instead of the builder canvas
- File path: `~/code/argo/web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx:69,87,105-109,111-129`
- Suggested fix: trace the route divergence — likely an `onError` / `fallback` path on the builder page redirecting when `useWorkflow` returns `null` AND a `useEffect` somewhere overrides the `isNewWorkflow` branch. Add a Playwright test `clicks_sales_tile_lands_on_blank_canvas` to lock the behavior.

### F9 [P2] — Skills page empty-state copy still says "6 skills · 0 shown" instead of "0 of 6"

Same cosmetic mismatch as prior F11. The count pill at `web/app/(app)/skills/page.tsx:113-114` reads `{skills.length} skills · {filtered.length} shown` which becomes `6 skills · 0 shown` during a no-match search. The empty-state body below says `No skills match "<query>"`. Two redundant phrasings.

- File path: `~/code/argo/web/app/(app)/skills/page.tsx:113-114`
- Suggested fix: when `filtered.length === 0 && query !== ""`, render `0 of {skills.length}` instead.

### F10 [P2] — Workflow `cohort_label` on the cohorts page is irrelevant; the demo banner is on-screen in the recording

The `DemoBanner` at `web/components/skycot/DemoBanner.tsx` pins to the top of every `(app)` page when `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1`. Says "Demo mode — showing sample data. Sign in to see your own workspace." It IS dismissible (persists to localStorage via `skycot:demo-banner-dismissed`) so the operator can dismiss it once and it stays gone for the rest of the recording session. Operator workaround = real, but worth noting as a pre-record checklist item.

- File path: `~/code/argo/web/components/skycot/DemoBanner.tsx:39-66`
- Suggested fix: add the dismiss step to the v1-mission-mvp-runbook pre-record checklist ("dismiss DemoBanner before clip 1 takes start"). Also consider auto-hiding the banner when `ARGO_DEMO_RECORDING=1` env var is set, distinct from `ARGO_DEMO_MODE`.

---

## Resolved since prior audit (2026-05-25)

| Prior F# | Resolution | Evidence |
|---|---|---|
| F1 (MOCK pill) | RESOLVED | `web/app/(app)/workflows/page.tsx:69` now hardcodes `WORKFLOWS · v0.1`; source-pill diagnostic relegated to `data-source` attribute only. Live snapshot confirms. |
| F4 (Rio / Sam non-canon authors) | RESOLVED | `web/lib/sample-workflows.ts:103, 124, 135` now read `Mariah / Jamal / Nori` (all canon). Live snapshot of `/workflows` confirms "BY MARIAH / BY JAMAL / BY NORI" on the previously-non-canon cards. |
| F6 (CORS preflight 404 on `/skills`) | RESOLVED on the client side | `web/lib/api/skills.ts:169` now hits `/api/v1/skills` (prefixed). The gateway's unprefixed `OPTIONS /skills` is still 404 (verified: `curl -X OPTIONS http://127.0.0.1:7400/skills` → 404) but the SPA no longer calls that path so the preflight regression is moot. |
| F7 (Run workflow vs Run now) | RESOLVED | Button label is now `Run now` (matches runbook). See `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:263`. NOTE: button is now `disabled` for fixture workflows — see new F4 above. |
| F8 (no `/workflows/build` entry point) | RESOLVED | `web/app/(app)/workflows/build/page.tsx` now ships a 5-vertical picker (Sales / Support / GTM / People Ops / Finance). NOTE: clicking through has a routing bug — see new F8 above. |
| F9 (SubWorkflowPicker hardcoded SEED_WORKFLOWS) | RESOLVED | `web/app/(app)/workflows/[verticalId]/[name]/build/SubWorkflowPicker.tsx:89-95` now uses `useWorkflows()` and falls back to `SEED_WORKFLOWS` only when `isLoading` or empty. |

Newly observed positives (not in prior audit):
- `/workflows/templates` ships 50 templates with vertical + risk facets — exceeds runbook expectation.
- `enableSkill` / `disableSkill` / `promoteLearned` are fully wired between `/workers/<id>/skills` UI and the gateway (`POST /api/v1/workers/<id>/skills`); Claim 4's runtime path is end-to-end live (just buried — see F3).
- `WorkflowProposeModal` exists with a sonner-toast confirmation; not a stub, just lightweight.

---

## Claim verdicts

- **Claim 4 (Skills shared + learned) — LIVE backend, BROKEN front-door affordance.** Runtime: `LayeredLookup::with_library` confirmed wired at `crates/argo-agent-runner/src/turn_helpers.rs:71`. Gateway: `POST /api/v1/workers/<id>/skills` real (200 path, 401 without principal). SPA: `enableSkill()` real, wired to `[Try it]` on `/workers/<id>/skills`. **But** the marketplace front door at `/skills/<id>` has no install affordance — the operator has to know to navigate to a worker's skills tab. Scene 4 step 3 unrecordable on the surface the runbook names.
- **Claim 5 (Any-PC execution via MCP + CU) — LIKELY-LIVE substrate, NOT-EXPOSED in the SPA.** The seed crate has a real `customer_research_followup` workflow that combines `skill_invoke` + `cu.run_macro` (computer-use Notes.app). But the gateway DB is empty, the SPA never queries the `marketplace` vertical, the fixture catalog doesn't include the workflow, the `Run now` button is `disabled` on all fixture workflows, the builder canvas has no `mcp_invoke` / `cu_action` step kinds, and no card on `/workflows` shows a `macOS` / `Browser` / `E2B` substrate chip. The SPA does not surface the MCP+CU combination capability today.
- **Workflow card author chip (Q3 in scope brief)** — does NOT distinguish worker / human / template-pack (see F5).

---

## Orientation score: 56 / 100

Up from 38 / 100 in the prior audit — substantial structural progress. The MOCK pill is dead, the non-canon worker names are corrected, the builder has a blank-canvas entry, templates surface 50 options, the worker skills tab has a real install flow, and the CORS bug is fixed. The remaining gaps are still demo-critical though: Scene 5 has no on-camera proof of the MCP+CU combination, Scene 4's install beat can't be recorded on `/skills/<id>` as scripted, and the workflow seed is sitting in a crate that isn't being run against the live gateway. All four P0s are 1-2 file fixes — the work is short, but it's blocking the recording, not optional polish.

Per the function-vs-dressing rule: F1 (combo workflow orphaned) is FUNCTION-side (the camera will narrate "and here's MCP+CU together" with no card to point at); F3 (no install button on `/skills/<id>`) is FUNCTION-side (the camera will narrate "click install" with no button to click); F4 (Run now disabled) is FUNCTION-side (the runbook narrates the click). F2 (search returns 0) is FUNCTION-side (the search beat is in the script). None of the four P0s are dressing — all four break the recording as scripted.
