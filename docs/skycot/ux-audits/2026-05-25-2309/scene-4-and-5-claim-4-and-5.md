# Scene 4 + Scene 5 (Claims 4 + 5 — skills marketplace + workflow execution) — re-audit

**Date**: 2026-05-25 23:09 AEST (Melbourne)
**Auditor**: e2e-ux-journey-test-agent — Scene 4 (`/skills`) + Scene 5 (`/workflows` + `/workflows/build`)
**Method**: live SPA walk via Claude Preview MCP against the in-repo dev server on `127.0.0.1:60894` (ARGO_DEMO_MODE=1, signed in as `demo@skycot.com`), gateway live on `127.0.0.1:7400` (schema v81). Direct `curl` against gateway routes. Read-only source-code spot-checks against `web/app/(app)/skills/`, `web/app/(app)/workflows/`, `web/lib/sample-workflows.ts`, `web/lib/api/skills.ts`, `web/lib/workflow-templates.ts`.
**Prior audit**: `docs/skycot/ux-audits/2026-05-25-1100-scene-4-and-5-claim-4-and-5.md` (the 11:00 sweep). This re-audit confirms which prior findings still ship on current `origin/main` and where the in-flight fix PRs are.

---

## TL;DR

Scene 4 + Scene 5 are still **unrecordable as scripted**. Of the prior audit's five P0s, **all five remain on `origin/main`** at the time of this run — the corresponding fix PRs (#2298 MOCK pill, #2300 skills CORS, #2301 SEED_WORKFLOWS leak) are all OPEN, not merged. The two most camera-visible defects are unchanged: `/workflows` still renders a literal **"WORKFLOWS · MOCK"** eyebrow pill (Scene 5 narrates this page for ~80 s), and `/skills` still falls back to a 6-skill seed list with zero matches for **"competitor pricing scrape"** (Scene 4 narrates exactly that query and expects 3 results). The `/skills/[skillId]` Install button is still entirely absent. The canonical Scene 5 workflow (`customer_research_followup`) plus its two step kinds (`mcp_invoke`, `cu_action`) still do not exist anywhere in `crates/` or `web/`. Three workflow cards still author to non-canon names (Rio x2, Sam x1).

10 findings below — 5 P0 demo-blockers, 3 P1 visible-on-camera issues, 2 P2 polish. Function-vs-dressing: every P0 is function-side or scene-vocabulary; none are dressing-side.

---

## Headline blockers

| # | Severity | Finding | Scene impact |
|---|---|---|---|
| F1 | P0 | `/workflows` renders "WORKFLOWS · MOCK" eyebrow pill on screen | Scene 5 — visible the whole scene |
| F2 | P0 | Skills search for "competitor pricing scrape" returns 0 hits, not 3 | Scene 4 — narrated hero search beat |
| F3 | P0 | `customer_research_followup` workflow + `mcp_invoke` + `cu_action` step kinds do not exist | Scene 5 — the MCP+CU combo workflow has no source code |
| F4 | P0 | Workflow cards author to non-canon names "Rio" (x2) and "Sam" (x1) | Scene 5 — vocab mismatch on-camera |
| F5 | P0 | `/skills/[skillId]` has no Install button + no install flow | Scene 4 step 3 — "Click Install" cannot be performed |

---

## All findings, in order

### F1 [P0] — `/workflows` still ships "WORKFLOWS · MOCK" eyebrow pill on screen

DOM-verified live on the running SPA: `document.querySelector('[data-testid="workflows-source-pill"]')` returns `<div data-source="mock-offline">WORKFLOWS · MOCK</div>`. The full visible eyebrow text is `WORKFLOWS · MOCK` with the brand purple accent dot. Scene 5's narrated surface labels itself MOCK to the camera.

The fix PR **#2298** (`codex-fix-workflows-page-mock-pill-eyebrow-on-camera`) is OPEN, not merged. Until it lands, the on-camera pill stays.

- Reproduction:
  - Run dev server in demo mode, sign in (`demo@skycot.com`), navigate to `/workflows`
  - Inspect: `document.querySelector('[data-testid=workflows-source-pill]').textContent` → `"WORKFLOWS · MOCK"`
  - Visual: pill is top-left, large, immediately above the H1
- File path: `web/app/(app)/workflows/page.tsx:68` (the literal `{isLive ? "LIVE" : "MOCK"}` ternary in the JSX)
- Driving condition: `web/lib/hooks/use-workflows.ts` → `source: "mock-offline"` when gateway buckets are empty
- Suggested fix: merge #2298, or drop the source suffix entirely (e.g. `WORKFLOWS · v0.1`). Per the function-vs-dressing rule this is literal on-camera theatre and is the line.

### F2 [P0] — `/skills` search for "competitor pricing scrape" returns 0 results, not 3

DOM-verified live: typed "competitor pricing scrape" into the hero search input on `/skills`; the count pill flipped to `6 skills · 0 shown`; the empty-state body shows `No skills match "competitor pricing scrape". Try a different search or clear the filter.` Zero `[data-testid=skill-card]` nodes render.

The 6 seed skills in `web/lib/api/skills.ts` are `approval.route`, `invoice.extract`, `candidate.screen`, `deal.qualify`, `support.triage`, `lead.enrich`. None of those match the tokens "competitor", "pricing", or "scrape" in name, description, or layer. Gateway live state: `GET /skills` returns `{"count":0,...,"skills":[]}` — so the SPA falls through to the seed, and the seed lacks the demo skill.

The fix brief `fix-skills-marketplace-search-seed-competitor-pricing` exists (in BUILD_STATUS) but no PR yet.

- Reproduction:
  - `/skills` → type `competitor pricing scrape` into the hero search → see "0 shown"
  - `curl -s http://127.0.0.1:7400/skills` → `{"count":0,"risk":"medium","risk_defaulted":true,"skills":[]}`
- File path: `web/lib/api/skills.ts:142` (the 6-row seed fallback)
- Suggested fix: seed 3 matching skills into both `SAMPLE_SKILLS` AND the gateway demo-seed pipeline so the live query also returns them (per the fix brief).

### F3 [P0] — `customer_research_followup` workflow + `mcp_invoke` / `cu_action` step kinds do not exist

`grep -rn "customer_research_followup\|mcp_invoke\|cu_action" web/ crates/` returns ZERO matches across the repo. The builder's authoritative `StepKind` union (`web/app/(app)/workflows/[verticalId]/[name]/build/types.ts:22-34`) is `trigger | skill_invoke | approval | tool_call | notify | decision | data | loop | wait | sub_workflow | http_request | code` — no `mcp_invoke`, no `cu_action`. The 50-row `web/lib/workflow-templates.ts` set also does not include the workflow.

Scene 5's narration ("Step 1 fires through the MCP pool…" then "Step 2 opens Notes.app via computer-use…") cannot be performed against a workflow that doesn't exist, with step kinds the builder doesn't enumerate.

The fix brief `fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds` exists, no PR yet.

- Reproduction:
  - `grep -rn "customer_research_followup\|mcp_invoke\|cu_action" ~/code/argo/web ~/code/argo/crates` → 0 lines
  - DOM walk on `/workflows` and `/workflows/templates`: scan all card names — no `customer_research_followup` row
- File path (would-touch): `web/lib/sample-workflows.ts` + `web/lib/workflow-templates.ts` + builder `types.ts:22` + a matching gateway/runtime path that actually executes MCP + CU steps end-to-end
- Suggested fix: ship the brief end-to-end (workflow seed + step-kind enum entries + runtime dispatch). If runtime dispatch is too tall an ask before recording, rewrite Scene 5 to demo an existing tool_call workflow whose dispatcher trace happens to include Zendesk/Notes.

### F4 [P0] — Workflow cards still show non-canon worker names "Rio" (x2) and "Sam" (x1)

DOM-verified live on `/workflows`: 7 cards render; their `by ` byline runs `Mariah / Priya / Jamal / Rio / Jamal / Rio / Sam`. The canon roster is `Mariah / Maya / Peta / Priya / Jamal / Nori` (operator-locked per memory + cohorts page). Three cards (`qualify_lead`, `morning_pipeline_digest`, `clean_customer_list`) attribute to non-canon names.

- Reproduction:
  - `/workflows` → walk the bottom byline of each card
- File path: `web/lib/sample-workflows.ts:103,124,135` (`lastEditor: "Rio"`, `"Rio"`, `"Sam"`)
- Suggested fix: trivial 3-line rename. `Rio → Peta`, `Sam → Nori` (or any canon name).

### F5 [P0] — `/skills/[skillId]` has no Install button and no install flow

DOM-verified live on `/skills/approval.route`: `document.querySelectorAll('main button')` returns one anonymous button (no label); `document.querySelectorAll('main a')` returns one link with text `"← Back to the marketplace"`. The body text matches the regex `/install/i` → false. Nothing on the page lets an operator install the skill.

Gateway also lacks the route the Scene 4 narration implies: `POST /skills/<id>/install` and `POST /api/v1/skills/<id>/install` both return `{"error":"not found"}`. The actual install endpoint is `POST /skills` with a raw SKILL.md body (`crates/argo-cli/src/handlers/gateway/routes_post_writes_skills.rs:13`) — different shape, not what an `<InstallSkillButton>` would call.

Fix brief `fix-skills-detail-install-button-and-flow` exists, no PR yet (zombie-claimed history per ledger).

- Reproduction:
  - `/skills/approval.route` → inspect `main button, main a` → only the back-link affordance
  - `curl -X POST http://127.0.0.1:7400/skills/approval.route/install` → 404
- File path: `web/app/(app)/skills/[skillId]/page.tsx` (no Install affordance authored). New component would live at `web/components/skycot/skills/InstallSkillButton.tsx`.
- Suggested fix: ship the existing fix brief — `<InstallSkillButton>` + the gateway `POST /api/v1/skills/<id>/install` route + cross-tenant binding test.

### F6 [P1] — Gateway `OPTIONS /skills` returns 404 (CORS preflight is still broken on the un-prefixed route)

CORS preflight verified live:
- `OPTIONS http://127.0.0.1:7400/skills` → 404
- `OPTIONS http://127.0.0.1:7400/api/v1/skills` → 204 with proper `Access-Control-Allow-*` headers

The SPA `useSkills()` hook resolves to whichever the apiGet base URL is. The browser blocks the call at preflight, the hook errors, and the page falls through to the 6-card seed (which is precisely how F2 stays exposed even when the operator seeds the gateway).

Fix PR **#2300** (`codex-fix-skills-gateway-route-prefix-and-cors-preflight`) is OPEN.

- Reproduction:
  - `curl -X OPTIONS -H "Origin: http://localhost:3000" http://127.0.0.1:7400/skills` → 404
  - `curl -X OPTIONS -H "Origin: http://localhost:3000" http://127.0.0.1:7400/api/v1/skills` → 204
- File path: gateway router source under `crates/argo-cli/src/handlers/gateway/`
- Suggested fix: merge #2300. Until then the SPA literally cannot read live gateway skills.

### F7 [P1] — Workflow-detail Run button still labelled "Run workflow", not "Run now"

`/workflows/[verticalId]/[name]/page.tsx:280` ships `{running ? "Running…" : "Run workflow"}`. Scene 5 narration says "Hit Run." Verbatim mismatch on-camera. Cosmetic but the scope brief calls it out.

- Reproduction:
  - Navigate to any workflow detail page (e.g. `/workflows/sales/qualify_lead`)
  - Read the primary CTA button label
- File path: `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:280`
- Suggested fix: change the label to "Run now" — 1-line edit. Or update the runbook narration. Pick one and stop drifting.

### F8 [P1] — `/workflows/build` is still a 404; no top-level "compose from scratch" entry point

`curl -I http://127.0.0.1:3000/workflows/build` redirects to signin (which is correct for unauth) but the actual app route directory `web/app/(app)/workflows/` has no `build/` child — only `[verticalId]/[name]/build/`. So an operator can only EDIT existing workflows; the runbook's "open the canvas blank and compose" beat has no entry-point.

- Reproduction:
  - `ls web/app/(app)/workflows/` → no `build/` directory
- File path: would need a new `web/app/(app)/workflows/build/page.tsx` (or a redirect into `[vertical]/new/build`)
- Suggested fix: add a thin `/workflows/build` page that asks "which vertical?" then routes into the builder. Alternative: drop the URL from the runbook and tell the operator to enter via "Propose new" → builder.

### F9 [P2] — Builder's SubWorkflowPicker still reads from the static `SEED_WORKFLOWS` fixture

`web/app/(app)/workflows/[verticalId]/[name]/build/SubWorkflowPicker.tsx:5` still `import { SEED_WORKFLOWS } from "./seed-workflows"` and `:24` `Object.entries(SEED_WORKFLOWS)…`. Dragging a sub-workflow node into a builder canvas during Scene 5 surfaces options from a local TS fixture, not the gateway.

Fix PR **#2301** (`codex-fix-workflow-builder-subworkflow-picker-uses-static-fixture`) is OPEN.

- Reproduction:
  - Open any workflow's builder (e.g. `/workflows/sales/qualify_lead/build`) → drag a sub-workflow palette item → confirm the picker dropdown is hard-coded
- File path: `web/app/(app)/workflows/[verticalId]/[name]/build/SubWorkflowPicker.tsx`
- Suggested fix: merge #2301. Function-vs-dressing rule: only a P2 because Scene 5 does not appear to drill into the sub-workflow picker; if the runbook narration starts pointing the camera at it, this jumps to P1.

### F10 [P2] — Empty-search count pill on `/skills` still says "6 skills · 0 shown"

DOM-verified: when the search filter excludes everything, the count pill in the eyebrow row reads `6 skills · 0 shown` while the body underneath reads `No skills match "<query>"`. Two redundant phrasings of the same empty state. Lowers polish.

- Reproduction:
  - `/skills` → type any non-matching query → read the count pill plus the body
- File path: `web/app/(app)/skills/page.tsx:113-117` (count pill JSX) + `:178` (empty body)
- Suggested fix: when `filtered.length === 0 && query.length > 0` show `0 of 6` in the pill, or drop the second clause from the body. Pick one source of truth.

---

## Resolved since the 11:00 audit

None of the 11 prior findings have shipped to `origin/main` as of this run. The fix PRs **#2298** (MOCK pill, F1), **#2300** (skills CORS, F6), **#2301** (SubWorkflowPicker, F9) are all OPEN. The matching briefs for **F2** (`fix-skills-marketplace-search-seed-competitor-pricing`), **F3** (`fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds`), and **F5** (`fix-skills-detail-install-button-and-flow`) exist but have no PRs yet. Two prior P2 findings (F10 in the 11:00 audit — non-canon-name regression test; F11 — empty-state phrasing) remain open as housekeeping.

---

## Orientation score: 38 / 100

Unchanged from the 11:00 audit. The two demo surfaces load and look like the locked-design memories (A1 marketplace + brand-loud workflow catalog), so the visual register is fine — but the load-bearing demo beats (search beat fan-out, click Install, MCP+CU workflow, no MOCK label) all still fail. Per the function-vs-dressing rule: F1 is on-camera theatre; F2 + F3 + F5 are function-side absences; F4 is scene-vocabulary mismatch; none of the P0s are dressing-side. The five fixes are short, mostly 1-3 file changes, and three of them have open PRs ready to merge. The shortest path to a recordable Scene 4 + Scene 5 is: merge #2298, merge #2300, author + merge the three remaining fix briefs.
