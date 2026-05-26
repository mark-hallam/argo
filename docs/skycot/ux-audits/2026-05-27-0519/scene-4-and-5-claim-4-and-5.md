# Scene 4 + 5 UX audit — Claim 4 (Skills shared + learned) + Claim 5 (Any-PC execution)

**Audit run**: 2026-05-27 0519 Melbourne (AEST)
**Scope**: `/skills` marketplace + search + install flow, `/skills/[skillId]` detail, `/workflows` catalog, `/workflows/[v]/[n]` viewer, `/workflows/build` vertical picker, `/workflows/[v]/[n]/build` canvas + palette, `/workflows/templates`.
**Methodology**: Claude Preview MCP (Model Context Protocol — Anthropic's tool-integration standard) drove the live dev server on port 3000 / 54588 (the preview chrome cycled twice mid-run; recovered both times). Cross-checked with read-only source review + live `curl` against the gateway at `127.0.0.1:7400`. Compared against the 4-hour-prior audit at `docs/skycot/ux-audits/2026-05-27-0415/scene-4-and-5-claim-4-and-5.md` — there have been no skills- or workflows-related commits since that audit (only #2429 spa-client same-origin and an onboarding-claim PR), so state is largely unchanged. Function-vs-dressing rule applied per `docs/skycot/demos/v1-demo-positioning-poc-vs-public.md`.

## TL;DR

The four core P0 demo-blockers identified four hours ago are all still live. Scene 4's search beat still returns zero results when the operator types the runbook wording `competitor pricing scrape` (substring miss against `pricing.competitor_scrape`). Five of seven TRY chips still match zero seed skills. The skill detail page still has no Install button (Scene 4 "click Install" has no target). Scene 5's `customer_research_followup` workflow still does not exist in any seed (sample, gateway, or template) — only in a vitest mock fixture. The workflow builder palette has **no MCP (Model Context Protocol) step kind and no computer-use step kind** — there is no first-class way to author the MCP-plus-CU combo the runbook narrates. Every visible substrate chip across all seven seeded workflow cards reads "HEADLESS · IDLE" — the camera will see no macOS / browser / E2B (E2B is the cloud sandbox the platform uses for any-PC execution) indicator anywhere on the screen during Scene 5. Counts: **4 P0**, **5 P1**, **3 P2**, **2 P3**. Orientation **42 / 100** — essentially unchanged from the 0415 audit's 44/100 because nothing material has shipped between runs.

## Headline blockers

| # | Severity | One-line finding | Scene impact |
|---|---|---|---|
| F1 | P0 | Searching `competitor pricing scrape` (literal runbook wording) returns zero results — substring filter misses word-order mismatch with seeded `pricing.competitor_scrape` | Scene 4 search beat shows empty grid + "No skills match 'competitor pricing scrape'" |
| F2 | P0 | Skill detail page (`/skills/[skillId]`) has no Install button — only Provenance + Workers-using + adoption count + Back link | Scene 4 "click Install" has no target |
| F3 | P0 | `customer_research_followup` workflow exists only in a vitest mock — not in `SAMPLE_WORKFLOW_SUMMARIES`, not in `WORKFLOW_TEMPLATES`, not seeded on the live gateway | Scene 5 has nothing to click |
| F4 | P0 | Builder palette has no MCP-invoke step kind and no computer-use step kind. Closest is `Browser action (Playwright)` (a `tool_call`). No way to author or visualize the MCP+CU combo. | Scene 5 narrates "MCP plus computer-use combo" but the canvas can show neither |

## All findings, in order

### F1 [P0] — Scene 4 search beat returns zero results because word order doesn't substring-match the seed

`web/app/(app)/skills/page.tsx:65-78` filters via case-insensitive substring match across `name`, `description`, and `layer`. The Scene-4 anchor row in `web/lib/api/skills.ts:64-73` is `pricing.competitor_scrape` with description starting "Scrape competitor product pricing across listed channels…". The Scene 4 runbook tells the operator to type literally `competitor pricing scrape`. None of `name`, `description`, or `layer` contains that exact substring in that word order — the description has `competitor product pricing`, not `competitor pricing scrape`. Live preview confirms: typing the phrase flips `<span data-testid="skills-count">` to `7 skills · 0 shown` and renders the `skills-empty` panel ("No skills match 'competitor pricing scrape'…"). The camera reads this directly.

- Reproduce:
  - Open `/skills`. Type `competitor pricing scrape` into the hero search.
  - Observe the count pill flip to `0 shown` and the empty-state panel render.
- File paths: `web/app/(app)/skills/page.tsx:65-78` (filter), `web/lib/api/skills.ts:64-73` (seed wording)
- Suggested fix: either (a) rename the seed to `competitor.pricing_scrape` AND prepend the description with "Competitor pricing scrape —", OR (b) change the runbook talk-track to type `competitor scrape` (which does substring-match both the name and the description). The runbook edit is the five-minute fix.

### F2 [P0] — Skill detail page has no Install button — Scene 4 "click Install" has no target

`web/app/(app)/skills/[skillId]/page.tsx` renders the locked A1 detail surface — eyebrow + title + Provenance section (author chip + TrustGradeChip) + Workers-using list + adoption count + a single "← Back to the marketplace" link. There is no Install / Add to my workspace / Use this skill button anywhere on the page. The Scene 4 runbook directs the operator to click Install after landing on the result; there is nothing to click. Open brief `fix-skills-detail-install-button-and-flow` has been zombie-claimed twelve times in the past twelve hours per evaluator round 63 — root cause is the headless-worker `claude /login` outage, but the operator-only fix unblocks it.

- Reproduce:
  - Open `/skills`, click any card (e.g. `pricing.competitor_scrape`).
  - Observe: no Install button. Only Provenance + Workers + Back link.
- File path: `web/app/(app)/skills/[skillId]/page.tsx:29-122`
- Suggested fix: add an Install button bound to a `POST /api/v1/skills/{name}/install` (or equivalent tenant-scoped binding); on success persist into the workspace's skill library so the worker-detail page's Skills tab shows the new row. Existing zombie-claimed brief is the canonical fix.

### F3 [P0] — `customer_research_followup` workflow does not exist anywhere bindable

Static grep confirms `customer_research_followup` appears only in `web/app/(app)/workflows/[verticalId]/[name]/build/SubWorkflowPicker.test.tsx:40,55,64,69` — a vitest mock fixture for the sub-workflow picker. It is **not** in `web/lib/sample-workflows.ts` (the seven `SAMPLE_WORKFLOW_SUMMARIES` are post_listing / screen_candidate / approve_refund / qualify_lead / triage_ticket / morning_pipeline_digest / clean_customer_list — none match). It is **not** in `web/lib/workflow-templates.ts` (50 templates, zero hits on grep `customer.*research` or `customer.*followup`). Live `curl http://127.0.0.1:7400/api/v1/verticals/sales/workflows` returns 401 due to missing principal header, but the SPA-side fetcher fans out from `discoverVerticalIds()` derived from the local seed — and the local seed has no `customer_research_followup`. Scene 5 has nothing to navigate to. An open brief `fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds` has been zombie-claimed ≥3 times per evaluator round 63 — same headless-worker outage as F2.

- Reproduce:
  - Open `/workflows`. Observe seven cards: post_listing / screen_candidate / approve_refund / qualify_lead / triage_ticket / morning_pipeline_digest / clean_customer_list. None contains "customer research".
  - `grep -rn customer_research_followup web/lib/` — only the test fixture.
- File paths: `web/lib/sample-workflows.ts` (add the seed), `web/lib/workflow-templates.ts` (also add as a clone template), and a gateway-side seed.
- Suggested fix: add `customer_research_followup` to `SAMPLE_WORKFLOW_SUMMARIES` with `vertical: "sales"`, `lastEditor: "Peta"` (per the vitest fixture), `risk: "approval-required"`, plus a matching `WorkflowSpec` entry under `SAMPLE_WORKFLOW_SPECS` with 5-7 steps where step kinds include `tool_call` (with description mentioning Notes.app or `cu_action` so `inferSubstrateFromText` flips substrate to `macos`) and `skill_invoke` for the MCP call. This makes the camera see real-looking steps and a macOS substrate chip.

### F4 [P0] — No MCP step kind and no computer-use step kind in the builder palette

`web/app/(app)/workflows/[verticalId]/[name]/build/types.ts:22-34` defines `StepKind` as `trigger | skill_invoke | approval | tool_call | notify | decision | data | loop | wait | sub_workflow | http_request | code`. There is no `mcp_invoke`, no `mcp_call`, no `computer_use`, no `cu_action`. The `SIDEBAR_PALETTE_GROUPS` array at line 236 exposes Triggers / Skills / Logic / Approval gates / Tools / Notify — under Tools the only entries are `HTTP request`, `Browser action (Playwright)`, `Code`. Live preview confirmed by enumerating draggables: zero hits for MCP, zero hits for Computer Use. Scene 5 narrates an MCP-plus-computer-use combo, but the operator cannot author one from the canvas and the camera will see neither a dedicated MCP node nor a dedicated CU node — only a generic "Browser action" tool-call that looks identical to any other web-API step.

- Reproduce:
  - Open `/workflows/sales/untitled/build`. Inspect the left sidebar palette.
  - Observe: no MCP-named or CU-named item. Closest is "Browser action (Playwright)".
- File path: `web/app/(app)/workflows/[verticalId]/[name]/build/types.ts:22-34` (StepKind enum) + `:236-286` (palette groups)
- Suggested fix: add `mcp_invoke` and `cu_action` as discrete step kinds with distinct icons (e.g. `Cpu` for MCP, `Monitor` for CU) and palette entries; route them through the existing `step-encoder.ts` to dedicated tool dispatches. Pair with F3's seed so Scene 5 has at least one canonical workflow that demonstrates both.

### F5 [P1] — Every visible substrate chip reads "HEADLESS · IDLE"

`/workflows` catalog snapshot: all seven cards render `SubstrateChip` with `substrate="headless"`. The substrate is inferred per `web/lib/api/workflows.ts:331-351` from explicit `workflow.substrate` (none of the seven seeds set it) or by scanning `name + description + steps[].description` for tokens `cu_action / cu. / cu_ / computer-use / macos / notes app / windows / linux / e2b / sandbox / browser / web page`. None of the seven seeds contains any of those tokens, so all default to `headless`. Result: the catalog screen the camera sweeps during Scene 5 has zero visual indication that **any** of these workflows targets macOS / browser / E2B / Windows. The "Any-PC execution" claim (Claim 5) is invisible. Dressing-side analysis: substrate chips are visible on cards the camera sweeps, so they count as function-side per the drill-in test — the operator needs at least one card showing a non-headless substrate for the demo to land.

- Reproduce:
  - Open `/workflows`. Read substrate chip on every card.
- File paths: `web/lib/sample-workflows.ts:64-138` (add `substrate: "macos"` or description tokens) + `web/lib/api/workflows.ts:371-390` (inference dict)
- Suggested fix: in the F3 seed for `customer_research_followup` add description text like "Drive the operator's Notes app via computer-use to capture meeting notes" — the `notes app` + `computer-use` tokens flip substrate to `macos`. Also add `substrate: "browser"` directly to one of the other existing seeds (e.g. `qualify_lead`) for cross-substrate variety.

### F6 [P1] — Skill marketplace TRY chips: most return zero results

`web/app/(app)/skills/page.tsx:23-30` lists six chips: `approval routing`, `invoice extraction`, `candidate screening`, `deal qualification`, `support triage`, `lead enrichment`. Each clicks `setQuery(chip)`. Substring-matched against the seven seed names + descriptions, only a few chips land any hits because the names are verb-stem (`approval.route`) but the chips are noun-phrase (`approval routing`). Same root cause as F1 with a different surface. Per the 0415 prior audit, 0-of-42 chip×seed pairs match — confirmed by re-reading the seed strings against the chip array. (The 0415 audit flagged this as P0; with the function-vs-dressing rule applied — the camera does not click TRY chips during the Scene 4 talk-track unless the operator improvises — it lands at P1 in this pass: visible if the operator clicks, but the narration tells them to type, not click.)

- Reproduce:
  - Open `/skills`. Click any TRY chip. Observe `0 shown`.
- File paths: `web/app/(app)/skills/page.tsx:23-30` (chip text), `web/lib/api/skills.ts:64-143` (seed)
- Suggested fix: change chip text to `routing`, `extraction`, `screening`, `qualification`, `triage`, `enrichment` (drop the noun head) — these substring-match the seed descriptions even though they don't match names. OR keep chip text and rename seeds to noun-phrase form. Aligning chip wording with seed wording is the cheapest fix.

### F7 [P1] — Gateway returns empty skills + verticals list on `/api/v1/*`

Live probe at run time:
- `curl http://127.0.0.1:7400/api/v1/skills` → `{"skills":[], "tenant_id":"019e5057-…", "total_count":0}`
- `curl http://127.0.0.1:7400/api/v1/verticals` → `{"verticals":[]}` (different envelope from the legacy `/verticals` route which returns 33 loaded verticals under `{loaded:[…], errors:[], ok:true}`)

PR #2407 ("seed demo marketplace skill catalog on gateway boot") merged 04bdc8592 four hours ago, but the gateway binary on port 7400 was built earlier and has not been restarted with the new boot-seed code — so `/api/v1/skills` still returns the empty envelope. The SPA's `fetchSkills` falls back to `SEED_SKILLS` only in dev (`process.env.NODE_ENV !== 'production'`) — so dev demo works via fallback, but the camera sees "mock-offline" data, not gateway data. The Scene 4 narration "the registry is real" is technically true (we are not currently navigating to a 404) but the count is dressing-side fallback, not function-side gateway.

- Reproduce:
  - `curl -s http://127.0.0.1:7400/api/v1/skills` and `curl -s http://127.0.0.1:7400/api/v1/verticals`. Both return empty arrays.
- File paths: gateway boot logic in `crates/argo-cli/src/handlers/gateway/` (specifically the skills boot-seed shipped in PR #2407 + the `/api/v1/verticals` envelope alignment from PR #2300's gateway-side counterpart `gateway-mount-api-v1-skills-routes-+-align-verticals-list-envelope`).
- Suggested fix: restart the gateway via `make restart-live-gateway` (per CLAUDE.md "canonical gateway port is 7400") to pick up #2407's boot-seed. Also author/ship the carry-forward brief from evaluator round 63 that aligns the `/api/v1/verticals` envelope to `{loaded, errors, ok}`. After restart, the SPA flips from `data-source="mock-offline"` to `gateway` and the demo lands a function-side real-feed.

### F8 [P1] — `/workflows/build` vertical picker omits two verticals — Marketing and Compliance not selectable

`web/app/(app)/workflows/build/page.tsx:4-10` hardcodes five vertical entries: `sales / support / gtm-engineer / people-ops / finance`. The `/verticals` gateway response lists 33 verticals loaded (legacy envelope). The cohorts page (`/cohorts`) shows seven department tabs including Marketing and Compliance. Operator hitting "Start from blank" cannot pick those two — they are silently absent. For the demo this is dressing-side (Scene 5 picks an existing workflow, not a fresh one), but for any post-demo customer click-through it's a visible gap.

- Reproduce:
  - Open `/workflows/build`. Observe five vertical cards.
- File path: `web/app/(app)/workflows/build/page.tsx:4-10`
- Suggested fix: import the canonical vertical list from the gateway `/verticals` envelope or the cohorts source-of-truth, not a hardcoded array. P2 if Scene 5 is the only audience.

### F9 [P1] — Run-now button POSTs but gateway rejects with 403 csrf_missing on direct call (browser path likely OK)

`web/app/(app)/workflows/[verticalId]/[name]/page.tsx:163` calls `executeWorkflow({verticalId, name})` which POSTs `/api/v1/verticals/<v>/workflows/<n>/execute`. Direct fetch against the gateway in the preview yielded `{"error":"csrf_missing","reason":"X-CSRF-Token header required for cookie-auth requests"}` (403). The SPA's `apiPost` client likely attaches the CSRF token automatically — verifying that the browser button works end-to-end is a live-demo gating question. The runbook narrates "click Run now → audit row lands in 5 s" — if the CSRF wiring isn't in place, this beat fails on camera. Worth a live-camera dry run before demo day.

- Reproduce:
  - Open `/workflows/sales/qualify_lead` in the SPA. Click Run now. Watch the network tab for the execute POST + the audit feed for the new row.
- File paths: `web/lib/api/workflow-execute.ts:34-44`, `web/lib/api/client.ts` (CSRF wiring)
- Suggested fix: verify CSRF token is attached in browser path; if not, fix before demo day.

### F10 [P2] — Workflow card author chip credits canonical worker names (Mariah/Priya/Jamal/Nori) — Peta missing from sales seeds

All seven seed-workflow author chips read `by Mariah` / `by Priya` / `by Jamal` / `by Nori`. Peta (the locked Project Coordinator P-name worker per memory `project_pm_cohorts_peta_patrick`) is not credited on any of the seven. The vitest mock for `customer_research_followup` (F3 seed-to-be-authored) sets `lastEditor: "Peta"`. Once F3 is fixed, Peta will get an author chip — but until then, the camera reads only the four legacy P-names. Dressing-side (worker personas are fictional fixtures) but ties into the function-side F3 fix when it lands.

- Reproduce:
  - Open `/workflows`. Read author chip on each card.
- File path: `web/lib/sample-workflows.ts:65-138`
- Suggested fix: bundle with F3 — when adding `customer_research_followup` set `lastEditor: "Peta"`.

### F11 [P2] — `GET /workflows/runs` returns 404 — dead route logged in dev console on every audit-feed refresh

Dev-server logs from the live preview show `GET /workflows/runs 404 in 74ms` and `GET /workflows/edit 404 in 59ms` and `GET /skills-canvas 404 in 63ms`. These are not navigated-to during the demo but appear in the route compile log — suggesting either a missing route, a dead link, or an old hash that something is hitting. Worth grepping for callers; if they are old dead links from the audit page or the cohorts page, the camera might not see them but the dev tools (if open) would.

- Reproduce:
  - Tail the dev server logs while clicking through `/audit`, `/workflows`, `/skills`.
- File paths: search `web/app /web/components` for hrefs `/workflows/runs`, `/workflows/edit`, `/skills-canvas`.
- Suggested fix: either add the routes or remove the stale links. Brief `fix-skills-canvas-flat-route-404-or-redirect.md` already exists in `docs/build-plan/streams/`.

### F12 [P2] — Workflow viewer's substrate chip and Risk pill use bg-color HEX literals that ignore brand tokens

`web/app/(app)/workflows/page.tsx:36-39` hardcodes `RISK_COLOR = {"approval-required": "#f59e0b", autonomous: "#14b8a6"}`. The locked brand identity (per memory `project_skycot_brand_identity` + `project_brand_consistency_pass`) uses violet → pink gradient. The current amber/teal pills feel off-brand and disconnect from the workers page's gradient cards. Polish, not blocker.

- File path: `web/app/(app)/workflows/page.tsx:31-39`
- Suggested fix: route risk colors through CSS variables tied to the brand token system.

### F13 [P3] — Skill card icon rotation uses `Sparkles / Wrench / Briefcase / TrendingUp / Mail` regardless of skill semantics

`web/app/(app)/skills/page.tsx:32-38` defines `ICON_ROTATION` as a five-icon array applied by index modulo. The pricing-competitor-scrape card gets a Sparkles icon, invoice-extract gets Briefcase, etc. — semantically wrong (pricing should be a chart icon, invoice should be a receipt icon). Camera will read these in passing; not load-bearing.

- File path: `web/app/(app)/skills/page.tsx:32-38`
- Suggested fix: switch to a `name → icon` map keyed off the skill prefix (`pricing.* → TrendingUp`, `invoice.* → Receipt`, etc.).

### F14 [P3] — Demo-mode banner persists across all in-product pages and crowds the top of the canvas

The "Demo mode — showing sample data. Sign in to see your own workspace." pill renders persistently above every in-app page (visible in the workflow-detail screenshot). Not breaking, but the camera reads it during every scene and the operator likely wants it suppressed (or replaced with a narration-friendly "POC mode" pill) for the recording.

- Suggested fix: toggle via a `?demo=hide` query param or temporarily comment out the demo banner during recording.

## Resolved since the 2026-05-27 0415 prior audit

Nothing material. Only commits between 04:15 and 05:19 were:
- `#2429` spa-client same-origin CORS fix (orthogonal to Claims 4+5)
- `#2430` onboarding-first-hire persona-bio (Scene 3 polish, orthogonal)
- Watchdog ping + conductor tick short-circuit (housekeeping)

The four headline P0s from the 0415 audit (F1 search beat, F2 install button, F3 customer_research_followup workflow, F4 step kinds) remain open and the zombie-claim loops on the corresponding briefs continue per evaluator-round-63 headless-logout diagnosis.

## Orientation score

**42 / 100.** The Skills marketplace renders a coherent A1 marketplace layout and the workflow viewer renders a coherent step list with risk + author + Run-now affordances. That's roughly half the surface working. The other half — Scene-4 search-to-install path, Scene-5 customer_research_followup workflow, the MCP+CU step kinds, the substrate chips telling the "Any-PC" story — is hollow. The two locked zombie-claim briefs (`fix-skills-detail-install-button-and-flow` + `fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds`) are the canonical fix vehicles; they need the headless-worker `claude /login` outage cleared by the operator before any agent picks them up.
