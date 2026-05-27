# Scene 4 + 5 — Skills marketplace + Workflows (Claims 4 + 5)

**Audit run**: 2026-05-28 02:52 local
**Scope**: `/skills`, `/workflows`, `/workflows/build`, `/workflows/templates`, workflow detail + run
**Agent**: e2e-ux-journey-test-agent
**Source of truth**: `docs/skycot/demos/v1-mission-mvp-runbook.md` + `docs/skycot/demos/v1-demo-positioning-poc-vs-public.md`

## TL;DR

19 findings: **9 P0 demo-blockers**, **8 P1 visible-issues**, **2 P2 polish**. Scene 4 (`/skills`) is **unrecordable as scripted**: hero-search "competitor pricing scrape" returns 0 results, skill-detail has no install action, and worker-skills tab data is fixture (F4 — "Demo mode" banner aside, the cards LOOK real to a camera). Scene 5 (`/workflows`) is **doubly broken**: the only real workflow in the gateway (`customer_research_followup` — the canonical MCP + computer-use combo) is INVISIBLE because demo-bypass mode short-circuits to 8 fixture cards, AND even if it were visible the viewer cannot render `mcp_invoke` or `cu_action` step kinds. Net effect: the surfaces meant to prove Claims 4 + 5 (Skills shared+learned, Any-PC execution) currently prove neither — both narrations would require theatre the camera reads as real. Headline fix is one brief: re-seed the SPA's `SAMPLE_WORKFLOW_SUMMARIES` + extend the viewer's `WorkflowStepKind` so `customer_research_followup` shows up + renders end-to-end.

## Headline blockers

| # | Sev | Scene | Finding |
|---|---|---|---|
| F1 | P0 | Scene 4 | `/skills` hero search "competitor pricing scrape" returns 0 results (substring matcher doesn't tokenise) |
| F2 | P0 | Scene 4 | Runbook narrates "3 results" but seed has 1 matching skill — count mismatch the camera sees |
| F3 | P0 | Scene 4 | Skill detail has no Install / Add-to-worker CTA — install-flow beat unrecordable |
| F4 | P0 | Scene 4 | Worker-skills tab data is `web/lib/sample-worker-skills.ts` fixtures — names don't exist in gateway |
| F8 | P0 | Scene 5 | Real `customer_research_followup` workflow (MCP + computer-use combo) is INVISIBLE; `/workflows` shows 8 unrelated fixture cards via `mock-bypass` short-circuit |
| F9 | P0 | Scene 5 | Workflow detail viewer cannot render `mcp_invoke` or `cu_action` step kinds — type system blocks them |
| F10 | P0 | Scene 5 | All 8 fixture workflows show "Headless · Idle" substrate chip — Claim 5 evidence absent from catalog |
| F11 | P0 | Scenes 4+5 | Author-chip canon inconsistent: skills="Authored by SKYCOT · Trust A", workflow cards="BY MARIAH", workflow detail=no chip |
| F15 | P1→P0 | Scene 5 | "Run now" against any visible (fixture) workflow 401/404s vs. the gateway — the one workflow that exists is invisible (see F8) |

## All findings

### Skills marketplace `/skills`

#### F1 [P0] — Hero search "competitor pricing scrape" returns 0 results (Scene 4 search beat unrecordable as scripted)

The runbook calls for `Hero search "competitor pricing scrape" → 3 results`. The seed catalog has 1 skill matching (`pricing.competitor_scrape` → "Pricing competitor scrape"). Both the exact phrase and the runbook-narrated phrase return **0 shown**. Single-word searches each return 1: `competitor` → 1, `pricing` → 1, `scrape` → 1. Two-word: `competitor pricing` → 0, `pricing competitor` → 0. Cause: `web/app/(app)/skills/page.tsx:72-77` filters with `name.includes(q) || description.includes(q) || layer.includes(q)` — strict substring, not tokenised, so word-order matters.

- Repro:
  - Open `/skills`
  - Type "competitor pricing scrape" into the hero input
  - Count pill shows `7 skills · 0 shown` and empty-state `No skills match "..."`
- File: `web/app/(app)/skills/page.tsx:65-78`
- Suggested fix: split `q` on whitespace, AND-match each token across `name | description | layer`. Or rename the seed skill to `competitor.pricing_scrape` so substring still hits.

#### F2 [P0] — Scene 4 promises "3 results" but the seed only has 7 skills with 0 overlap on the search term

Even with a tokeniser fix, "competitor pricing scrape" maps to exactly 1 seed skill. The runbook narration "we found three relevant skills" cannot be recorded honestly without either (a) seeding 2 more pricing-adjacent skills (e.g. `pricing.benchmark`, `pricing.elasticity_model`), or (b) updating the runbook to "we found this skill". Per the function-vs-dressing rule, the camera SEES the count pill — so this is a function-side mismatch.

- Repro: `/skills` shows `7 skills · 7 shown` total. Search the runbook phrase, gets 0 (after F1 fix: 1).
- File: seed data lives in the gateway (`/api/v1/skills` returns 7 published-verified skills, all from `argo-seed`). Adjust seed in `crates/argo-store/...` or update runbook narration.
- Suggested fix: pick whichever — seed 2 more "pricing.*" skills, OR adjust runbook line.

#### F3 [P0] — Skill detail page has no install / add-to-worker action (Scene 4 install-flow beat unrecordable)

`/skills/pricing.competitor_scrape` renders only PROVENANCE (Authored by SKYCOT · Trust A) + WORKERS USING THIS SKILL (empty state) + adoption count (0). There is **no Install button, no Add-to-worker dropdown, no "Try with…" CTA, no inputs/outputs spec, no version list, no policy block**. The runbook calls for "Click result → install flow". The user-facing copy actively redirects: "Hire a worker and add this skill from their Skills tab to put it to work." — i.e. install is worker-tab-only, not marketplace-driven.

- Repro:
  - Navigate to `/skills/pricing.competitor_scrape`
  - Scan for any button/CTA — only `← Back to the marketplace` link exists
- File: `web/app/(app)/skills/[skillId]/page.tsx:38-122` (read-only by design — no install handler)
- Suggested fix: either add an "Add to worker…" CTA on the skill detail (opens a worker picker → calls a real mutation), or update the runbook to skip the detail-page beat and demo the install from `/workers/<id>/skills` directly.

#### F4 [P0] — Worker-skills tab data is fixture, not real (theater the camera sees if Scene 4 drills in)

`/workers/<id>/skills` shows three sections (Active / Library / Learned) with named skills (`rfp-response-drafter v4.0.0`, `renewal-checklist-runner v3.1.2`, `contract-redline-suggester v1.0.0`) and metrics (trust 89%, `used 18× last 7d`). These skill names do not appear in the gateway catalog (`/api/v1/skills` returns 7 totally different `*.*` skills like `pricing.competitor_scrape`, `approval.route`). The data is sourced from `web/lib/sample-worker-skills.ts` — explicit mock fallback. The "Demo mode — showing sample data" banner appears at the top of the page, but the visible skill cards LOOK real to a viewer. Per the function-vs-dressing rule, function-side (Claim 4 evidence) must be real; mock skills with mock trust scores break the claim.

- Repro:
  - Open `/workers/018f67d0-0000-7000-8000-000000000001/skills`
  - Note skill cards `rfp-response-drafter`, `renewal-checklist-runner`, `contract-redline-suggester`
  - Cross-check `curl http://127.0.0.1:7400/api/v1/skills` — none of those names appear
  - `grep -rn "rfp-response-drafter" web/` → `web/lib/sample-worker-skills.ts:110`
- Files: `web/lib/sample-worker-skills.ts:1-200` (the fallback) + `web/lib/hooks/use-worker-skills.ts` (likely uses the fallback when gateway 404s)
- Suggested fix: if Scene 4 drills into a worker's Skills tab, seed real worker_skill_link rows in the gateway so the Active section shows skills from the real catalog. The current mock + the "Demo mode" banner is honest in dev, but the camera reads the data as real.

#### F5 [P1] — Skill detail "Trust · A" chip is render-time, not learning-loop evidence (Claim 4 "learned" not visible)

The detail page shows `Trust · A`. There is no version timeline, no trust-grade-changed-from-B-to-A history, no drift indicator, no "Learned via N successful runs" telemetry. The page is a static brochure, not a learning-loop surface. For Claim 4 (Skills shared AND LEARNED), this is the place to show learning telemetry. Currently the only learning evidence in the SPA is the "Learned" empty-state on `/workers/<id>/skills` (fixture data, see F4) and a separate `useSkillsLearnedThisMonth` chip on `/skills` (count-based filter, no detail).

- Repro: `/skills/pricing.competitor_scrape` — observe no telemetry / no version history.
- File: `web/app/(app)/skills/[skillId]/page.tsx:38-122`
- Suggested fix: add a "Learning loop" section that shows trust-grade-history + adoption-trend + recent-promotion timeline. For demo, can fixture if the camera doesn't drill in further, but should look like real telemetry not a chip.

#### F6 [P2] — "Try" chips don't include any multi-word combos despite the design promising plain-English search

The locked Skills page design ("hero search bar with suggested-search chips") implies natural-language entry. All 6 TRY chips are single tokens (`approval`, `invoice`, `candidate`, `deal`, `support`, `lead`). The 7th chip "Skills my workers learned this month" toggles a filter. The locked design memory `project_skills_page_design_locked` describes "signature hero-search pattern (28px gradient icon + 18px input + ⌘K + count pill + try-pills)" reusable across catalogs — single-token chips work but the marketing copy ("Drop in a pre-built skill — approval routing, invoice extraction, candidate screening, and dozens more") sets multi-word expectation.

- Repro: `/skills` — observe try chips.
- File: `web/app/(app)/skills/page.tsx:23-30`
- Suggested fix: a couple of multi-word chips would make the search feel smarter. Low priority for POC; nice-to-have.

#### F7 [P2] — Skill detail count-pill says "Used by 0 workers across the SKYCOT marketplace" — claim 4 "shared" reads as empty

The substrate exists (`/api/v1/skills/pricing.competitor_scrape/usage` returns 404 in `preview_network`, so the page falls through to `0` everywhere). For an investor demo, "Used by 0 workers" is the opposite signal of what Claim 4 wants to convey. Either seed adoption (1-2 fixture rows), or replace the integer with "Available to your workspace" + a CTA.

- Repro: `/skills/pricing.competitor_scrape` → "Used by 0 workers across the SKYCOT marketplace."
- File: `web/app/(app)/skills/[skillId]/page.tsx:107-111`
- Suggested fix: the `useSkillUsage` hook needs a real endpoint OR the page should render a more flattering empty state. Operator picks.

### Workflows `/workflows`

#### F8 [P0] — Real `customer_research_followup` workflow (the ONLY MCP + computer-use combo) is INVISIBLE; SPA shows 8 fixture cards instead

The runbook (`docs/skycot/demos/v1-mission-mvp-runbook.md`) Scene 5 narrates `customer_research_followup` — the **only** workflow that combines `skill_invoke` (MCP-shaped Zendesk fetch + summarise) with `tool_call` to `cu.run_macro` on the macOS Notes app. The gateway has this workflow at `GET /api/v1/workflows` (tenant `019e6785-…`, vertical `marketplace`, substrate `macos`, version 3, modified_by `argo-demo-seed`). But `/workflows` shows **8 totally different fixture cards** (`post_listing`, `screen_candidate`, `approve_refund`, `qualify_lead`, `triage_ticket`, `morning_pipeline_digest`, `clean_customer_list`, `weekly_standup_digest`) — none of which involve MCP or computer-use, and all of which show "Headless substrate idle" chips. Root cause: `lib/api/workflows.ts:134-136` short-circuits inside `isAuthBypassed()` mode and returns `SAMPLE_WORKFLOW_SUMMARIES` **before** attempting any gateway query. The eyebrow's `data-source="mock-bypass"` attribute confirms this is the active code path on the running dev server. Per `feedback_no_theater_in_demo` ("v1.0 demo MP4 must be REAL not mocks; reject any demo brief that uses web/lib/sample-* fixtures as runtime data path"), this is the **headline Scene-5 blocker** — the function-side of Claim 5 (Any-PC execution: MCP + computer-use combo) literally cannot be filmed on this surface.

- Repro:
  - `curl http://127.0.0.1:7400/api/v1/workflows` → shows `customer_research_followup` with `cu.run_macro` step
  - Open `/workflows` in demo mode → 8 cards, none is `customer_research_followup`
  - `document.querySelector('.workflows-eyebrow').getAttribute('data-source')` → `"mock-bypass"`
- Files: `web/lib/api/workflows.ts:133-166` (the short-circuit) + `web/lib/sample-workflows.ts:97-...` (the 8 fixture summaries) + `web/app/(app)/workflows/page.tsx:46` (`workflows = data?.workflows ?? SAMPLE_WORKFLOW_SUMMARIES`)
- Suggested fix: in demo mode, still issue the per-vertical gateway calls and merge results with the seed (or replace seed). Simpler short-term: re-seed `SAMPLE_WORKFLOW_SUMMARIES` to include a `marketplace/customer_research_followup` entry (and matching `SAMPLE_WORKFLOW_SPECS` with `mcp_invoke` + `cu_action` steps) so the demo-bypass path shows the same thing the real backend does.

#### F9 [P0] — Workflow detail viewer cannot RENDER `mcp_invoke` or `cu_action` step kinds (type system blocks both)

Even if the real workflow were exposed in the catalog, the `/workflows/<v>/<n>` viewer would not be able to draw its steps. The viewer's `STEP_KIND_META` table at `page.tsx:62-75` covers only `{trigger, skill_invoke, approval, tool_call, notify, decision, data, http_request, code}`. The builder palette/types support `mcp_invoke` and `cu_action` (`build/types.ts:38-39`, `:221`), but those kinds are NOT part of the viewer's `WorkflowStepKind` (imported from `lib/sample-workflows.ts:23-31`). The result: the real `cu.run_macro` step would render as undefined icon/colour/label — i.e., the camera would see a broken row even if the data flowed in. This is a viewer/builder type-divergence (the canonical viewer types live in `sample-workflows.ts` but the builder added two more kinds in `build/types.ts` without back-porting to the viewer).

- Repro:
  - Compare `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:62-75` (9 kinds) vs `web/app/(app)/workflows/[verticalId]/[name]/build/types.ts:218-227` (11 kinds — includes `mcp_invoke` + `cu_action`)
  - Cross-check: real workflow TOML has `kind = "tool_call"` for `cu.run_macro` (degraded to a generic tool-call) — so the viewer would label it "Tool call" and not surface "Computer-use" — Claim 5 evidence is suppressed.
- File: `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:62-75` + `web/lib/sample-workflows.ts:23-31`
- Suggested fix: extend `WorkflowStepKind` to include `mcp_invoke` + `cu_action`; add icons (Plug + Monitor) and labels ("MCP connector", "Computer-use") to `STEP_KIND_META`; align with builder.

#### F10 [P0] — All 8 fixture workflows show "Headless substrate idle" chip — Claim 5 evidence not present anywhere in the catalog

The `SubstrateChip` for every card resolves to `headless` because `resolveWorkflowSubstrate` (`lib/api/workflows.ts:334-354`) infers substrate by text-matching step names/descriptions for words like `cu_action`, `cu.`, `computer-use`, `macos`, `notes app`, etc. None of the 8 fixture workflows contain those tokens. So the entire Scene-5 surface is silent on Claim 5 — no purple "macOS" chip, no "Computer-use" tag, no "MCP connector" pill. A viewer scanning the catalog cannot distinguish a workflow that touches a real desktop from one that doesn't.

- Repro: `/workflows` → all 8 cards display "HEADLESS · IDLE" tag.
- Files: `web/lib/sample-workflows.ts:60-...` (no `cu_action` step in any sample) + `web/lib/api/workflows.ts:374-389` (substrate inference rules)
- Suggested fix: seed at least one fixture workflow with a `cu_action`/`mcp_invoke` step so the card lights up purple; or (better) wire the real workflow in (see F8) and drop the fixtures.

#### F11 [P0] — Author chip canon is inconsistent: workflow cards say "by Mariah", workflow detail shows NO author, skill detail shows "Authored by SKYCOT · Trust A"

Author/provenance chip is a load-bearing demo signal (per memory + runbook). Three surfaces, three different conventions:
- Skills detail (`/skills/<id>`): `Authored by SKYCOT · Trust A` — formal provenance line
- Workflow card (`/workflows`): `BY MARIAH` — bare worker name (lastEditor)
- Workflow detail (`/workflows/<v>/<n>`): **no author chip at all** — only `vertical · risk · version N · substrate chip`

If Scene 5 drills into a workflow, the viewer page does not say who authored it or maintains it. The fixture data assigns `lastEditor: "Mariah"` etc. — implying a worker authored a workflow, which contradicts the "SKYCOT ships workflows that you can fork" model implied on the skills page. Either authorship is "SKYCOT" (templates / first-party packs) or it's the operator / their workers (forks) — pick one and apply consistently.

- Repro:
  - `/skills/pricing.competitor_scrape` → "Authored by SKYCOT · Trust A"
  - `/workflows` card → "BY MARIAH" / "BY JAMAL" etc.
  - `/workflows/sales/qualify_lead` (when reachable) → no author shown
- Files: `web/app/(app)/workflows/page.tsx:267` (card "by {lastEditor}") + `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:213-235` (detail meta row, no author)
- Suggested fix: agree the canon (proposal: `Authored by SKYCOT · maintained by {worker}` when forked; `Authored by SKYCOT` when first-party). Apply to card AND detail meta-row.

#### F12 [P1] — Real workflow versions are integers (`version: 3`), card UI labels them `V3` but detail viewer prints `version 1.0.0`-style strings — incompatible shapes

The gateway returns `version: 3` (integer). The card formats it as `v{wf.versions}` → "V3" (capitalised). The detail viewer renders `version {workflow.version}` (`page.tsx:225`) — the spec-form `workflow.version` is a string. The substrate / versioning surface drifts between the two pages. Minor unless the camera lingers, but the inconsistency adds confusion.

- Repro: `/workflows` card → "V3"; gateway returns `version: 3`; detail spec carries `version: string` field — different shapes.
- Files: `web/app/(app)/workflows/page.tsx:259-262` + `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:225` + `web/lib/api/workflows.ts` (summary → integer, spec → string)
- Suggested fix: normalise — pick one format ("v3" or "version 3"), apply both surfaces.

#### F13 [P1] — `/workflows/build` is a 5-vertical chooser, NOT the visual canvas; `marketplace` vertical (where the real workflow lives) is not listed

`/workflows/build` (`web/app/(app)/workflows/build/page.tsx:4-10`) hard-codes 5 vertical options (sales / support / gtm-engineer / people-ops / finance). The real demo workflow lives in `marketplace`, and the swarm has also been wiring `project-ops` per the M21/M22 PM cohorts. Operators clicking "Browse 50+ templates" or "Propose new" → vertical picker → can't pick the marketplace bucket where Scene 5's workflow is filed. Workflow detail viewer + canvas at `/workflows/<v>/<n>/build` works fine if you have the URL, but the entry path is broken for at least 2 known verticals.

- Repro: visit `/workflows/build` → 5 cards, no `marketplace` and no `project-ops`.
- File: `web/app/(app)/workflows/build/page.tsx:4-10`
- Suggested fix: derive verticals from the gateway `/api/v1/verticals` (or merge gateway + hardcoded list); or at minimum add `marketplace` + `project-ops` to the array.

#### F14 [P1] — Hero CTA "Propose new" opens a modal; "Browse 50+ templates" page shows 50 fixtures with NO link back to real builder for the chosen template

The "Browse 50+ templates" CTA on `/workflows` links to `/workflows/templates` — a fully fixture-driven page (`web/lib/workflow-templates.ts:97-654` — 50 `t(...)` calls, no gateway round-trip ever). Per the file's own doc comment: "Real binding to gateway-stored templates lands in a follow-up brief; this V1 ships a curated seed". Template detail's "Use template" CTA links to `/workflows/{vertical}/{name}/build?from_template={id}` — but `{name}` is the template's `name` (e.g. `qualify_lead`), which may collide with an existing workflow under that vertical. No "this template wraps step X / requires connector Y" preflight surfaced.

- Repro: `/workflows/templates` → 50 cards. Click "qualify_lead" → detail. Click "Use template" → `/workflows/sales/qualify_lead/build?from_template=sales-qualify-lead` — same path as the existing fixture workflow under `sales/qualify_lead`.
- Files: `web/lib/workflow-templates.ts` + `web/app/(app)/workflows/templates/[templateId]/page.tsx:69`
- Suggested fix: dedupe by name-collision check, or namespace template-instantiated workflows (e.g. `qualify_lead_<uuid_short>`); defer wiring templates to gateway storage to a separate brief.

#### F15 [P1] — "Run now" on workflow detail calls `POST /api/v1/verticals/{v}/workflows/{name}/execute` — works for fixture name but 404s vs. the real workflow when the demo gateway has only `customer_research_followup`

`handleRunWorkflow` (`page.tsx:160-175`) calls `executeWorkflow({ verticalId, name })` against the gateway. For demo-bypass with fixture data, all 8 visible workflows have names the gateway has never heard of (no `qualify_lead`, no `triage_ticket` in `019e6785-…` tenant's seed — gateway only has `customer_research_followup`). Result: clicking "Run now" against any fixture workflow either 401s (no auth) or 404s (workflow not found), then displays a `toast.error("Run failed: …")`. The camera would see the toast and the user's confidence in the demo would collapse. Conversely, the workflow the gateway CAN execute is the one the SPA refuses to show (see F8).

- Repro:
  - `curl -X POST http://127.0.0.1:7400/api/v1/verticals/sales/workflows/qualify_lead/execute -H 'Content-Type: application/json' -d '{}'` → `401` (or `404` post-auth) — fixture name unknown to gateway
  - `curl -X POST http://127.0.0.1:7400/api/v1/verticals/marketplace/workflows/customer_research_followup/execute …` → `401` (would `200` with auth) — but UI never reaches this card
- File: `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:160-175` + `web/lib/api/workflow-execute.ts:34-44`
- Suggested fix: tie this to F8 — once `customer_research_followup` is the surfaced demo workflow, "Run now" lands a real audit row. As-is, no "Run now" beat is recordable.

#### F16 [P1] — `/workflows/templates` and the `WORKFLOW_TEMPLATES` list reference `mcp_invoke`/`cu_action`-shaped behaviour in 0 of 50 templates

`grep "MCP\|mcp_invoke\|cu_action\|computer-use\|cu.run_macro" web/lib/workflow-templates.ts` returns 0 matches across the 654 lines / 50 templates. None of the "ready-to-clone" templates uses an MCP connector or a computer-use step. Per the n8n-moat narrative the file comment cites, this is a missing-feature signal: the template gallery doesn't yet showcase the two step types that distinguish SKYCOT from n8n.

- Repro: `grep -c mcp_invoke web/lib/workflow-templates.ts` → `0`; same for `cu_action`.
- File: `web/lib/workflow-templates.ts:97-654`
- Suggested fix: add at least 2 "MCP-powered" template examples (e.g., Notion → Slack digest, Linear → GitHub mirror) and 1 computer-use example (e.g., "Drop summary into macOS Notes app"). Wires the Claim 5 story into the template-marketing surface.

#### F17 [P2] — Workflow card "TRY" chips are all multi-word (good) but unrelated to MCP/computer-use vocabulary

Cards' try-chips: `approval flows`, `data extraction`, `outbound qualification`, `escalation`. Filtering by these works (the search filter is substring on name/vertical/description). But the operator-facing copy doesn't expose the things that differentiate this product — no chip for "computer-use", "macOS", "MCP", "connector", "Zendesk", "Notion". An investor camera would see a generic-looking catalog without the differentiator surfaced.

- Repro: `/workflows` → observe TRY chip row.
- File: `web/app/(app)/workflows/page.tsx:29`
- Suggested fix: add 1-2 chips like `computer-use` or `MCP` once F8/F10 land seed data that matches.

#### F18 [P2] — Workflow detail page has no MCP-connector-status block; you can't tell from the viewer whether the workflow's connectors are connected for THIS tenant

Workflow steps include connector references (e.g., `skill: "zendesk fetch thread"` in the real TOML). The viewer renders the step as "Sparkles · Skill" but doesn't lift the underlying connector or check tenant-side auth state. To know whether running this workflow will fail at "fetch_thread" because Zendesk OAuth has expired, the operator has to navigate to `/settings/connectors`. For Claim 5 ("Any-PC execution") to feel real, the workflow viewer should preflight: "Zendesk connector: ✅ connected · Notes (cu macOS): ✅ macOS-permissions granted" — and gate "Run now" if anything is missing.

- Repro: `/workflows/<v>/<n>` → no connector-state block in the meta row.
- File: `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:205-260`
- Suggested fix: add a preflight section that pulls from `/api/v1/tenant/connectors` (already used on `/settings/connectors`) and renders a green/amber/red status per connector the workflow needs.

#### F19 [P2] — `/workflows/build` "Workflow builder" header is uncoupled from brand language (no Audiowide eyebrow, no gradient ink, no D3 register)

The build entry page (`/workflows/build`) uses a Tailwind-classed header (`text-3xl font-semibold tracking-normal`) with no Audiowide, no warm bg, no gradient — it feels like a placeholder from before the brand-consistency pass. Per memory `project_brand_consistency_pass`, all in-product surfaces should carry the D3 recipe. The 50-template gallery (`/workflows/templates`) does carry the recipe; the builder entry page does not.

- Repro: visit `/workflows/build` → bare Tailwind hierarchy, no SKYCOT-brand affordances.
- File: `web/app/(app)/workflows/build/page.tsx:23-34`
- Suggested fix: port the eyebrow + Audiowide + radial-bg recipe used on `/workflows` index. Low priority unless Scene 5 navigates here.


