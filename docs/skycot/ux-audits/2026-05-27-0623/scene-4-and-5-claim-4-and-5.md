# Scene 4 + 5 UX audit — Claim 4 (Skills shared + learned) + Claim 5 (Any-PC execution)

**Audit run**: 2026-05-27 0623 Melbourne (AEST)
**Scope**: `/skills` marketplace + search + install flow, `/skills/[skillId]` detail, `/workflows` catalog, `/workflows/[v]/[n]` viewer, `/workflows/build` vertical picker, `/workflows/[v]/[n]/build` canvas + palette.
**Methodology**: Started the Next.js dev server via Claude Preview MCP (Model Context Protocol — Anthropic's tool-integration standard). The server compiled the middleware in 4.2 s then hung on `Compiling /landing` and was killed twice by the OS — consistent with the documented worker-side memory-pressure pattern (see `CLAUDE.md` "memory-pressure safeguards"). Pivoted to static read-only source review + live `curl` against the gateway at `127.0.0.1:7400` (schema v83, FRESH). Cross-checked against the 4-hour-prior audit `docs/skycot/ux-audits/2026-05-27-0519/scene-4-and-5-claim-4-and-5.md` and the `git log` since then. Function-vs-dressing rule applied per `docs/skycot/demos/v1-demo-positioning-poc-vs-public.md`.

## TL;DR

Material change since the 0519 audit: **F6 (TRY chips zero-results) is RESOLVED** — #2431 ("rewrite TRY chips to single-word stems") shipped at 20:05 last night. All six new chips (`approval / invoice / candidate / deal / support / lead`) now substring-match a seeded skill name. Everything else is unchanged. The four core P0 demo-blockers are still live: Scene 4 search beat still returns zero results for the literal runbook phrase `competitor pricing scrape` (word-order substring miss against `pricing.competitor_scrape`); the skill detail page still has no Install button so Scene 4's "click Install" beat has no target; Scene 5's `customer_research_followup` workflow still exists only in a vitest mock fixture (not in the SPA seed, not in templates, not in the live gateway); the workflow-builder palette still has **no MCP step kind and no computer-use step kind** — there is no first-class way to author the MCP+CU combo the runbook narrates and no visible canvas affordance for either backend. Counts: **4 P0**, **5 P1**, **2 P2**, **2 P3**. Orientation **44 / 100** — up 2 from the prior 42/100 thanks to the chip fix.

## Headline blockers

| # | Severity | One-line finding | Scene impact |
|---|---|---|---|
| F1 | P0 | Searching `competitor pricing scrape` (literal runbook wording) returns zero results — substring filter misses word-order mismatch with seeded `pricing.competitor_scrape` | Scene 4 search beat shows empty grid + "No skills match 'competitor pricing scrape'" |
| F2 | P0 | Skill detail page (`/skills/[skillId]`) has no Install button — only Provenance + Workers-using + adoption count + Back link | Scene 4 "click Install" has no target |
| F3 | P0 | `customer_research_followup` workflow exists only in a vitest mock — not in `SAMPLE_WORKFLOW_SUMMARIES`, not in `WORKFLOW_TEMPLATES`, not on the live gateway | Scene 5 has nothing to click |
| F4 | P0 | Builder palette has no MCP-invoke step kind and no computer-use step kind. Closest is `Browser action (Playwright)` (a `tool_call`). No way to author or visualize the MCP+CU combo. | Scene 5 narrates "MCP plus computer-use combo" but the canvas can show neither |

## All findings, in order

### F1 [P0] — Scene 4 search beat returns zero results because word order doesn't substring-match the seed

`web/app/(app)/skills/page.tsx:65-78` filters via case-insensitive substring match across `name`, `description`, and `layer`. The Scene-4 anchor row in `web/lib/api/skills.ts:64-73` is `pricing.competitor_scrape` with description starting "Scrape competitor product pricing across listed channels…". The Scene 4 runbook tells the operator to type literally `competitor pricing scrape`. None of `name`, `description`, or `layer` contains that exact substring in that word order — the description has `competitor product pricing`, not `competitor pricing scrape`. The count pill flips to `7 skills · 0 shown` and the `skills-empty` panel renders ("No skills match 'competitor pricing scrape'…"). The camera reads this directly.

- Reproduce:
  - Open `/skills`. Type `competitor pricing scrape` into the hero search.
  - Observe the count pill flip to `0 shown` and the empty-state panel render.
- File paths: `web/app/(app)/skills/page.tsx:65-78` (filter), `web/lib/api/skills.ts:64-73` (seed wording)
- Suggested fix: either (a) rename the seed to `competitor.pricing_scrape` AND prepend the description with "Competitor pricing scrape —", OR (b) change the runbook talk-track to type `competitor scrape` (which substring-matches both the name and the description). The runbook edit is the five-minute fix.

### F2 [P0] — Skill detail page has no Install button — Scene 4 "click Install" has no target

`web/app/(app)/skills/[skillId]/page.tsx` renders the locked A1 detail surface — eyebrow + title + Provenance section (author chip + TrustGradeChip) + Workers-using list + adoption count + a single "← Back to the marketplace" link. There is no Install / Add to my workspace / Use this skill button anywhere on the page. The Scene 4 runbook directs the operator to click Install after landing on the result; there is nothing to click. The open brief `fix-skills-detail-install-button-and-flow` continues to zombie-claim through the headless-worker `claude /login` outage flagged in evaluator round 63 — the operator-side fix unblocks it.

- Reproduce:
  - Open `/skills`, click any card (e.g. `pricing.competitor_scrape`).
  - Observe: no Install button. Only Provenance + Workers + Back link.
- File path: `web/app/(app)/skills/[skillId]/page.tsx:29-122`
- Suggested fix: add an Install button bound to `POST /api/v1/skills/{name}/install` (or equivalent tenant-scoped binding); on success persist into the workspace's skill library so the worker-detail page's Skills tab shows the new row. Pre-authored brief is the canonical fix vehicle.

### F3 [P0] — `customer_research_followup` workflow does not exist anywhere bindable

Static `rg` confirms `customer_research_followup` appears only in `web/app/(app)/workflows/[verticalId]/[name]/build/SubWorkflowPicker.test.tsx:40,55,64,69` — a vitest mock fixture for the sub-workflow picker. It is **not** in `web/lib/sample-workflows.ts` (the eight `SAMPLE_WORKFLOW_SUMMARIES` are post_listing / screen_candidate / approve_refund / qualify_lead / triage_ticket / morning_pipeline_digest / clean_customer_list / weekly_standup_digest — none match). It is **not** in `web/lib/workflow-templates.ts`. The live gateway returns `{"verticals":[]}` on `/api/v1/verticals`. Scene 5 has nothing to navigate to. The open brief `fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds` continues to zombie-claim per evaluator round 63 — same headless-worker outage as F2.

- Reproduce:
  - Open `/workflows`. Observe 8 cards — none contains "customer research".
  - `rg customer_research_followup web/lib/` — no hits outside the test fixture.
- File paths: `web/lib/sample-workflows.ts` (add the seed), `web/lib/workflow-templates.ts` (also clone a template), and a gateway-side seed.
- Suggested fix: add `customer_research_followup` to `SAMPLE_WORKFLOW_SUMMARIES` with `vertical: "sales"`, `lastEditor: "Peta"` (per the vitest fixture), `risk: "approval-required"`, plus a matching `WorkflowSpec` entry under `SAMPLE_WORKFLOW_SPECS` with 5-7 steps including a `tool_call` whose description mentions `Notes.app` or `cu_action` (so `inferSubstrateFromText` flips substrate to `macos`) and a `skill_invoke` for the MCP call. This makes the camera see real-looking steps and a macOS substrate chip.

### F4 [P0] — No MCP step kind and no computer-use step kind in the builder palette

`web/app/(app)/workflows/[verticalId]/[name]/build/types.ts:22-34` defines `StepKind` as `trigger | skill_invoke | approval | tool_call | notify | decision | data | loop | wait | sub_workflow | http_request | code`. There is no `mcp_invoke`, no `mcp_call`, no `computer_use`, no `cu_action`. The `SIDEBAR_PALETTE_GROUPS` array at `web/app/(app)/workflows/[verticalId]/[name]/build/types.ts:236-286` exposes Triggers / Skills / Logic / Approval gates / Tools / Notify — under Tools the only entries are `HTTP request`, `Browser action (Playwright)`, `Code`. Scene 5 narrates an MCP-plus-computer-use combo, but the operator cannot author one from the canvas and the camera will see neither a dedicated MCP node nor a dedicated CU node — only a generic "Browser action" tool-call that looks identical to any other web-API step.

- Reproduce:
  - Open `/workflows/sales/untitled/build`. Inspect the left sidebar palette.
  - Observe: no MCP-named or CU-named item. Closest is "Browser action (Playwright)".
- File path: `web/app/(app)/workflows/[verticalId]/[name]/build/types.ts:22-34` (StepKind enum) + `:236-286` (palette groups)
- Suggested fix: add `mcp_invoke` and `cu_action` as discrete step kinds with distinct icons (e.g. `Cpu` for MCP, `Monitor` for CU) and palette entries; route them through the existing `step-encoder.ts` to dedicated tool dispatches. Pair with F3's seed so Scene 5 has at least one canonical workflow that demonstrates both.

### F5 [P1] — Every visible substrate chip reads "HEADLESS · IDLE"

`web/lib/sample-workflows.ts:64-149` defines eight `SAMPLE_WORKFLOW_SUMMARIES`. None of them sets the `substrate` field. The fallback at `web/lib/api/workflows.ts:331-351` infers substrate from explicit `workflow.substrate` (none of the eight set it) or by scanning `name + description + steps[].description` for tokens `cu_action / cu. / cu_ / computer-use / macos / notes app / windows / linux / e2b / sandbox / browser / web page`. None of the eight contains any of those tokens, so all default to `headless`. The catalog screen the camera sweeps during Scene 5 has zero visual indication that any of these workflows targets macOS / browser / E2B (E2B is the cloud sandbox the platform uses for any-PC execution) / Windows. The "Any-PC execution" claim (Claim 5) is invisible on the catalog. The substrate chip is rendered on every card the camera sweeps, so it counts as function-side per the drill-in test.

- Reproduce:
  - Open `/workflows`. Read the substrate chip on every card.
- File paths: `web/lib/sample-workflows.ts:64-149` (add `substrate: "macos"` or description tokens) + `web/lib/api/workflows.ts:371-390` (inference dict)
- Suggested fix: in the F3 seed for `customer_research_followup` add description text like "Drive the operator's Notes app via computer-use to capture meeting notes" — the `notes app` + `computer-use` tokens flip substrate to `macos`. Also add `substrate: "browser"` directly to one of the existing seeds (e.g. `qualify_lead`) for cross-substrate variety.

### F6 [P1] — Gateway returns empty skills + verticals list on `/api/v1/*`

Live probe at run time (with `x-tenant-id: demo` + `x-argo-principal: human:demo@skycot.com` headers attached):
- `curl http://127.0.0.1:7400/api/v1/skills` → `{"skills":[], "tenant_id":"019e5057-…", "total_count":0}`
- `curl http://127.0.0.1:7400/api/v1/verticals` → `{"verticals":[]}` (different envelope from the legacy `/verticals` route which returns 33 loaded verticals under `{loaded:[…], errors:[], ok:true}`)

PR #2407 ("seed demo marketplace skill catalog on gateway boot") was merged but the gateway binary on port 7400 (PID 7348, uptime ~57 ks) was built earlier and has not been restarted — so `/api/v1/skills` still returns the empty envelope. The SPA's `fetchSkills` falls back to `SEED_SKILLS` only in dev (`process.env.NODE_ENV !== 'production'`) — so dev demo works via fallback, but the camera sees "mock-offline" data, not gateway data. The Scene 4 narration "the registry is real" is technically true (we are not currently navigating to a 404) but the count is dressing-side fallback, not function-side gateway.

- Reproduce:
  - `curl -s http://127.0.0.1:7400/api/v1/skills` and `curl -s http://127.0.0.1:7400/api/v1/verticals`. Both return empty arrays.
- File paths: gateway boot logic in `crates/argo-cli/src/handlers/gateway/`.
- Suggested fix: restart the gateway via `make restart-live-gateway` (per CLAUDE.md "canonical gateway port is 7400") to pick up #2407's boot-seed. Then ship the open `fix-gateway-skills-and-verticals-boot-seed-tenant-resolution` brief (already in `docs/build-plan/streams/`) to align the `/api/v1/verticals` envelope to `{loaded, errors, ok}`. After restart, the SPA flips from `data-source="mock-offline"` to `gateway` and the demo lands a function-side real-feed.

### F7 [P1] — `/workflows/build` vertical picker omits two verticals — Marketing and Compliance not selectable

`web/app/(app)/workflows/build/page.tsx:4-10` hardcodes five vertical entries: `sales / support / gtm-engineer / people-ops / finance`. The legacy `/verticals` gateway response lists 33 verticals loaded. The cohorts page (`/cohorts`) shows seven department tabs including Marketing and Compliance. Operator hitting "Start from blank" cannot pick those two — they are silently absent. For the demo this is dressing-side (Scene 5 picks an existing workflow, not a fresh one), but for any post-demo customer click-through it is a visible gap.

- Reproduce:
  - Open `/workflows/build`. Observe five vertical cards.
- File path: `web/app/(app)/workflows/build/page.tsx:4-10`
- Suggested fix: import the canonical vertical list from the gateway `/verticals` envelope or the cohorts source-of-truth, not a hardcoded array. P2 if Scene 5 is the only audience.

### F8 [P1] — Run-now button POSTs but gateway requires CSRF + principal — browser path needs end-to-end dry-run before recording

`web/app/(app)/workflows/[verticalId]/[name]/page.tsx:163` calls `executeWorkflow({verticalId, name})` which POSTs `/api/v1/verticals/<v>/workflows/<n>/execute`. Direct curl against the gateway in the audit yielded `{"error":"X-Argo-Principal header is required"}` (401) without a principal header and `{"error":"bad principal"}` (400) with a plain string principal — the gateway expects the JWT-derived `human:<email>` shape. The SPA's `apiPost` client attaches the principal from the session peek (`web/lib/api/client.ts:170-172`) and a CSRF token when needed; **the browser-driven path is likely OK** but the demo MP4 budget can't afford a live failure here. The runbook narrates "click Run now → audit row lands in 5 s" — verify this end-to-end before demo day.

- Reproduce:
  - Open `/workflows/sales/qualify_lead` in the SPA. Click Run now. Watch the network tab for the execute POST + the audit feed for the new row.
- File paths: `web/lib/api/workflow-execute.ts:34-44`, `web/lib/api/client.ts:140-198` (principal + CSRF wiring)
- Suggested fix: do one live dry-run of "Run now" → audit-row-appears before recording. If the principal-from-session peek is empty (e.g. demo bypass cookie path), patch `apiPost` to fallback-stamp `human:demo@skycot.com` so the gateway accepts the call.

### F9 [P1] — `/workflows/[v]/[n]` viewer's run history pulls from `mock-offline` source whenever gateway 404s

`web/app/(app)/workflows/[verticalId]/[name]/page.tsx:104-106` derives `isDemoRunHistory = source === "mock-offline" || source === "mock-bypass"` — and when the gateway returns no workflow row, the SPA falls through to seed and the run-history table renders fabricated rows. The page renders a "DEMO RUN HISTORY" pill in that mode (per the same code path), which is honest, but the rows themselves are theatre. If Scene 5 drills into the run history during the audit pass-through, the camera will read fabricated rows. Per the function-vs-dressing rule, this is function-side if the camera clicks a row — currently rated P1 because Scene 5 narrates `/audit` (not the workflow's local run-history table) as the audit destination.

- Reproduce:
  - Open `/workflows/sales/qualify_lead`. Scroll to "Run history". Look for the source pill.
- File path: `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:104-106` + the `WorkflowRunsTable` it renders.
- Suggested fix: when the run-history fallback is active, suppress the table (render an empty-state) instead of showing fabricated rows. This is honest theatre — same pattern as `/audit`'s gateway-falls-back-empty path.

### F10 [P2] — Workflow card author chip credits Mariah/Priya/Jamal/Nori/Peta — Mariah is over-indexed (3 of 8 cards)

All eight seed-workflow author chips: Mariah (3), Priya (1), Jamal (3), Nori (1), Peta (1). Mariah appearing on three cards including `post_listing` (Marketplace) and `morning_pipeline_digest` and `qualify_lead` overloads her persona — she is already the canonical Scene 1+2 worker. Splitting these between Mariah and the rest of the five-roster (Priya/Jamal/Nori/Peta — Patrick is the new sixth M22 worker per memory `project_pm_cohorts_peta_patrick`) would land a more diverse author-chip canon on the catalog screen. Dressing-side (worker personas are fictional fixtures), but the camera sweeps the catalog.

- Reproduce:
  - Open `/workflows`. Count author chips on the eight cards.
- File path: `web/lib/sample-workflows.ts:65-149`
- Suggested fix: rebalance to roughly Mariah×2, Priya×2, Jamal×2, Nori×1, Peta×1 (or include Patrick once and trim Mariah to 2).

### F11 [P2] — Workflow viewer's risk pill uses bg-color HEX literals that ignore brand tokens

`web/app/(app)/workflows/page.tsx:36-39` hardcodes `RISK_COLOR = {"approval-required": "#f59e0b", autonomous: "#14b8a6"}`. The locked brand identity (per memory `project_skycot_brand_identity` + `project_brand_consistency_pass`) uses violet → pink gradient. The current amber/teal pills feel off-brand and disconnect from the workers page's gradient cards. Polish, not blocker.

- File path: `web/app/(app)/workflows/page.tsx:31-39`
- Suggested fix: route risk colors through CSS variables tied to the brand token system.

### F12 [P3] — Skill card icon rotation uses `Sparkles / Wrench / Briefcase / TrendingUp / Mail` regardless of skill semantics

`web/app/(app)/skills/page.tsx:32-38` defines `ICON_ROTATION` as a five-icon array applied by index modulo. The pricing-competitor-scrape card gets a Sparkles icon, invoice-extract gets Briefcase, etc. — semantically wrong (pricing should be a chart icon, invoice should be a receipt icon). Camera will read these in passing; not load-bearing. Existing brief `fix-skills-marketplace-card-icon-rotation-semantic-name-map.md` covers this in `docs/build-plan/streams/`.

- File path: `web/app/(app)/skills/page.tsx:32-38`
- Suggested fix: switch to a `name → icon` map keyed off the skill prefix (`pricing.* → TrendingUp`, `invoice.* → Receipt`, etc.).

### F13 [P3] — Workflow TRY chips ("approval flows / data extraction / outbound qualification / escalation") not yet realigned to single-word stems like the Skills page

`web/app/(app)/workflows/page.tsx:29` defines `TRY_CHIPS = ["approval flows", "data extraction", "outbound qualification", "escalation"]`. Substring-matched against the eight seed workflows' `name + vertical + description`: `approval flows` matches `approve_refund`'s description ("approval"), `data extraction` matches nothing, `outbound qualification` matches nothing, `escalation` matches `triage_ticket`'s description. Half the chips zero-result like the Skills chips did before #2431. Same pattern, same fix shape — though the impact is far smaller because Scene 5 narration does not direct the operator to click these chips. Worth a follow-on cleanup brief mirroring the #2431 Skills chip fix.

- File path: `web/app/(app)/workflows/page.tsx:29`
- Suggested fix: rewrite to single-word stems: `approval`, `extraction`, `qualification`, `escalation` (or rename the chips to match the seed catalog's vertical names: `sales`, `support`, `finance`, `gtm`).

## Resolved since the 2026-05-27 0519 prior audit

- **F6 (TRY chips zero-results) RESOLVED** — #2431 "rewrite TRY chips to single-word stems that hit seeds" merged at T20-05. New chips `approval / invoice / candidate / deal / support / lead` each substring-match a seeded skill name (`approval.route`, `invoice.extract`, etc.). Confirmed by reading `web/app/(app)/skills/page.tsx:23-30`.
- **F11 from 0519 audit (dead `/workflows/runs`, `/workflows/edit`, `/skills-canvas` routes)** — could not reproduce in static rg of `web/app /web/components`. Either already cleaned or the dev-log warnings were transient. Removed from current punch list.

All other 0519 findings (F1 search beat, F2 install button, F3 customer_research_followup workflow, F4 step kinds, F5 substrate chips, F7 gateway empty, F8 vertical picker, F9 Run-now CSRF check) remain open and are renumbered as F1-F8 above. F10 (Peta missing from sales seeds) folded into F10 (Mariah over-indexed) — same root cause, different framing.

## Orientation score

**44 / 100.** The Skills marketplace renders a coherent A1 marketplace layout with a now-working TRY chip strip (post-#2431). The workflow viewer renders a coherent step list with risk + author + Run-now affordances. That's just over half the surface working. The other half — Scene-4 search-to-install path (F1+F2), Scene-5 `customer_research_followup` workflow (F3), the MCP+CU step kinds (F4), the substrate chips telling the "Any-PC" story (F5) — is hollow. The two locked zombie-claim briefs (`fix-skills-detail-install-button-and-flow` + `fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds`) are the canonical fix vehicles; they need the headless-worker `claude /login` outage cleared by the operator before any agent picks them up. Gateway restart for #2407 boot-seed is the single highest-leverage operator-only action — flips F6 from `mock-offline` to function-side `gateway` and unblocks the "registry is real" narration.
