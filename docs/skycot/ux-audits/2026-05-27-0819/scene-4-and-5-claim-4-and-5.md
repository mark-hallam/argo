# Scene 4 + 5 UX audit — Claim 4 (Skills shared + learned) + Claim 5 (Any-PC execution)

**Audit run**: 2026-05-27 0819 Melbourne (AEST)
**Scope**: `/skills` marketplace + search + install flow, `/skills/[skillId]` detail, `/workflows` catalog, `/workflows/[v]/[n]` viewer, `/workflows/build` vertical picker, `/workflows/[v]/[n]/build` canvas + palette, gateway POST execute path.
**Methodology**: Preview MCP (Model Context Protocol — Anthropic's tool-integration standard) was responsive for static probes but hung on heavy DnD-canvas-bearing builder routes (consistent with the documented memory-pressure pattern in `CLAUDE.md`). Backfilled with `curl` SSR (Server-Side Render — the HTML the Next.js server returns before the React client takes over) against `http://127.0.0.1:3000`, direct probes against the gateway on `http://127.0.0.1:7400` (schema v83, FRESH, uptime 64.5 ks), plus `rg` + `Read` against `web/` and the seed catalogs. Compared against the open findings in the 0739 audit + the merge log since (`evaluator round 95`, `claim fix-duties-card-vocab`, `claim fix-onboarding-dashboard-hire-button`, `claim fix-worker-activity`, `ops-claim-5-cold-rerun` brief claimed). Function-vs-dressing rule applied per `docs/skycot/demos/v1-demo-positioning-poc-vs-public.md`.

## TL;DR

Material change since the 0739 audit: **F8 from the 0739 audit (workflow-detail fabricated run-history rows) is RESOLVED** — the viewer now renders an honest "Demo workflow · no run history. Live run history appears after this workflow is served by the gateway." empty-state when `data-source` is mock-offline / mock-bypass (`web/app/(app)/workflows/[verticalId]/[name]/page.tsx:395-410`). The other core findings are unchanged: Scene 4 search beat still returns zero results for the literal runbook phrase `competitor pricing scrape`; the skill detail page still has no Install button; Scene 5's `customer_research_followup` workflow still exists only in a vitest mock fixture; every workflow card on the catalog still reads `data-substrate="headless"`; the gateway returns empty `/api/v1/skills` + `/api/v1/verticals` envelopes (still no restart since #2407); and the gateway `POST .../execute` still returns 403 forbidden — now upgraded to a confirmed click-through risk because the SPA's `Run now` button toasts the gateway error message via sonner. Two new findings this run: the SPA-path proxy `/api/workflows/.../execute` is a 404 (no Next.js API route exists at that path — the SPA hits the gateway directly), and the workflow viewer + builder canvas SSR returns skeleton-only HTML (page title leaks "Workflows · SKYCOT" instead of the workflow name; SEO content is empty). Counts: **3 P0**, **5 P1**, **3 P2**, **2 P3**. Orientation **50 / 100** — up 2 from prior 48/100 thanks to F8 resolution.

## Headline blockers

| # | Severity | One-line finding | Scene impact |
|---|---|---|---|
| F1 | P0 | Searching `competitor pricing scrape` (literal runbook wording) returns zero results — substring filter misses word-order mismatch with the seeded description "competitor product pricing" | Scene 4 search beat shows empty grid + "No skills match 'competitor pricing scrape'" |
| F2 | P0 | Skill detail page (`/skills/[skillId]`) has no Install / Add to my workspace / Use this skill button — only Provenance + Workers-using + adoption count + Back link | Scene 4 "click Install" has no target |
| F3 | P0 | `customer_research_followup` workflow exists only in the `SubWorkflowPicker.test.tsx` vitest mock — not in `SAMPLE_WORKFLOW_SUMMARIES`, not in `WORKFLOW_TEMPLATES`, not on the live gateway. None of the 8 seeded workflows uses `mcp_invoke` / `cu_action` step kinds. | Scene 5 has nothing to click |
| F4 | P0 (carried) | Gateway `POST /api/v1/verticals/sales/qualify_lead/execute` returns 403 forbidden — "principal lacks the required workflow capability" — with the standard `x-tenant-id: demo` + `x-argo-principal: human:demo@skycot.com` headers. The SPA's `handleRunWorkflow` toasts the gateway error via sonner, so the camera will read "Run failed: principal lacks the required workflow capability" on screen if Scene 5 narrates Run now. | Scene 5 Run-now click visibly fails with a red toast; no audit row lands |

## All findings, in order

### F1 [P0] — Scene 4 search beat returns zero results because word order doesn't substring-match the seed

`web/app/(app)/skills/page.tsx:65-78` filters via case-insensitive substring match across `name`, `description`, and `layer`. The Scene 4 anchor row in `web/lib/api/skills.ts:64-73` is `pricing.competitor_scrape` with description starting "Scrape competitor product pricing across listed channels…". The Scene 4 runbook directs the operator to type literally `competitor pricing scrape`. None of `name`, `description`, or `layer` contains that exact substring in that word order — the description has `competitor product pricing`, not `competitor pricing scrape`. The empty-state copy ("No skills match 'competitor pricing scrape'") is visible to the camera on the count pill + empty panel.

- Reproduce:
  - Open `/skills`. Type `competitor pricing scrape` into the hero search.
  - Observe the count pill flip to `7 skills · 0 shown` and the empty-state panel render.
- File paths: `web/app/(app)/skills/page.tsx:65-78` (filter), `web/lib/api/skills.ts:64-73` (seed wording)
- Suggested fix: either (a) prepend the description with "Competitor pricing scrape —" so the literal phrase substring-matches, OR (b) change the runbook talk-track to type `competitor scrape` (which substring-matches both the name and the description). The runbook edit is the five-minute fix.

### F2 [P0] — Skill detail page has no Install button — Scene 4 "click Install" has no target

`web/app/(app)/skills/[skillId]/page.tsx:29-122` renders the locked A1 detail surface — eyebrow + title + Provenance section (author chip + TrustGradeChip) + Workers-using list + adoption count + a single "← Back to the marketplace" link. Confirmed live in the SSR HTML at 0820: only the PWA browser-install service-worker hint matches `/install/i` in the markup; no skill-install action button exists. The Scene 4 runbook directs the operator to click Install after landing on the result; there is nothing to click. The open brief `fix-skills-detail-install-button-and-flow` continues to zombie-claim through the headless-worker `claude /login` outage flagged in evaluator round 63 — the operator-side fix is the load-bearing unblock.

- Reproduce:
  - Open `/skills`, click any card (e.g. `pricing.competitor_scrape`).
  - Observe: only Provenance + Workers + Back link; no Install button.
- File path: `web/app/(app)/skills/[skillId]/page.tsx:29-122`
- Suggested fix: add an Install button bound to `POST /api/v1/skills/{name}/install` (or equivalent tenant-scoped binding); on success persist into the workspace's skill library so the worker-detail page's Skills tab shows the new row. Pre-authored brief is the canonical fix vehicle.

### F3 [P0] — `customer_research_followup` workflow does not exist anywhere bindable

`rg customer_research_followup web/lib/ web/app/` returns hits only inside `web/app/(app)/workflows/[verticalId]/[name]/build/SubWorkflowPicker.test.tsx:40,55,64,69` — a vitest mock fixture for the sub-workflow picker. It is **not** in `web/lib/sample-workflows.ts` (the eight `SAMPLE_WORKFLOW_SUMMARIES` are post_listing / screen_candidate / approve_refund / qualify_lead / triage_ticket / morning_pipeline_digest / clean_customer_list / weekly_standup_digest — none match). It is **not** in `web/lib/workflow-templates.ts`. The live gateway returns `{"verticals":[]}` on `/api/v1/verticals`. The `/workflows` page renders 8 cards, none labeled customer_research_followup. The open brief `fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds` continues to zombie-claim per evaluator round 63 — same headless-worker outage as F2. Now that #2415 has shipped the step-kind plumbing, the only remaining work for Scene 5 is the seed itself + the spec that uses an `mcp_invoke` + a `cu_action`.

- Reproduce:
  - Open `/workflows`. Observe 8 cards — none contains "customer research".
  - `rg customer_research_followup web/lib/` — no hits outside the test fixture.
- File paths: `web/lib/sample-workflows.ts` (add the seed summary + spec), `web/lib/workflow-templates.ts` (also clone a template), and a gateway-side seed.
- Suggested fix: add `customer_research_followup` to `SAMPLE_WORKFLOW_SUMMARIES` with `vertical: "sales"`, `lastEditor: "Peta"` (per the vitest fixture), `risk: "approval-required"`, plus a matching `WorkflowSpec` entry under `SAMPLE_WORKFLOW_SPECS` with 5-7 steps **including at least one `kind: "mcp_invoke"` step (e.g. Notion or Slack lookup) and at least one `kind: "cu_action"` step (e.g. opens Notes.app via macOS computer-use)**. The two new kinds already ride through the encoder, the substrate inference, the inspectors, and the palette — they just have no consumer yet in the seed. Add `substrate: "macos"` explicitly to fix F5 on the same card.

### F4 [P0 — carried from 0739] — Gateway run-now returns 403; SPA toasts the error via sonner so the camera reads the failure

Live curl probe at run time: `curl -X POST -H 'x-tenant-id: demo' -H 'x-argo-principal: human:demo@skycot.com' http://127.0.0.1:7400/api/v1/verticals/sales/workflows/qualify_lead/execute` returned 403 — `{"error":{"code":"forbidden","message":"principal lacks the required workflow capability"}}`. The SPA's `handleRunWorkflow` callback at `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:159-174` wraps `executeWorkflow(...)` in a try/catch and on failure calls `toast.error(\`Run failed: ${message}\`)` — the gateway's 403 message ("principal lacks the required workflow capability") will surface directly in a red sonner toast. The Scene 5 runbook narrates "click Run now → audit row lands in 5s"; a 403 mid-recording surfaces as an on-camera red toast with cap-system error wording, and `/audit` shows no new row because the gateway never enqueued the run. Per the function-vs-dressing rule, this is **function-side** — the click-through must reach a real workflow turn and audit row.

Note also F4-b: there is **no SPA-path API proxy** at `/api/workflows/{v}/{n}/execute` — `curl -X POST http://127.0.0.1:3000/api/workflows/sales/qualify_lead/execute` returns 404 (no Next.js route). The SPA's `executeWorkflow` in `web/lib/api/workflow-execute.ts` posts directly to the gateway via `apiPost` + `gatewayBase()`. So there is no SPA-side cookie-elevated path that would route around the 403 — the gateway must accept the demo principal, full stop.

- Reproduce:
  - Operator shell: `curl -s -X POST -H 'x-tenant-id: demo' -H 'x-argo-principal: human:demo@skycot.com' http://127.0.0.1:7400/api/v1/verticals/sales/workflows/qualify_lead/execute` → 403.
  - SPA: open `/workflows/sales/qualify_lead`, click Run now, watch sonner for a red "Run failed: principal lacks the required workflow capability" toast; watch `/audit` — no new row.
- File paths: `crates/argo-cli/src/handlers/gateway/routes_workflows.rs:254-265` (capability check), `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:159-174` (Run-now error toast), `web/lib/api/workflow-execute.ts:34-44` (POST shape)
- Suggested fix: bind `human:demo@skycot.com` to an owner-role override in the demo workspace at gateway boot, OR wire a demo-only fast-path that elevates when `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1`. **Smoke first against the live SPA-path before recording** — only fix if the actual click still 403s with whatever session shape the SPA produces.

### F5 [P1] — Every visible substrate chip reads "HEADLESS · IDLE"

`web/lib/sample-workflows.ts:64-149` defines eight `SAMPLE_WORKFLOW_SUMMARIES`. None of them sets the `substrate` field. The fallback at `web/lib/api/workflows.ts:334-353` infers substrate from explicit `workflow.substrate` (none of the eight set it) or by scanning `name + description + steps[].description` for tokens `cu_action / cu. / cu_ / computer-use / macos / notes app / windows / linux / e2b / sandbox / browser / web page`. `rg substrate|mcp_invoke|cu_action web/lib/sample-workflows.ts` returns **zero hits** — so all eight default to `headless`. Confirmed live in the SSR HTML at 0820: eight `data-substrate="headless"` chips render across the catalog cards. The "Any-PC execution" claim (Claim 5) is invisible on the catalog. The substrate chip renders on every card the camera sweeps, so it counts as function-side per the drill-in test.

- Reproduce:
  - Open `/workflows`. Read the substrate chip on every card.
- File paths: `web/lib/sample-workflows.ts:64-149` (add `substrate: "macos"` or description tokens) + `web/lib/api/workflows.ts:374-393` (inference dict)
- Suggested fix: in the F3 seed for `customer_research_followup` add `substrate: "macos"` (or description text like "Drive the operator's Notes app via computer-use"). Also add `substrate: "browser"` directly to one of the existing seeds (e.g. `qualify_lead`) for cross-substrate variety — the camera then reads at least two different substrate chips on the catalog screen.

### F6 [P1] — Gateway returns empty skills + verticals list on `/api/v1/*` — gateway still not restarted since #2407

Live probe at run time (with `x-tenant-id: demo` + `x-argo-principal: human:demo@skycot.com`):
- `curl http://127.0.0.1:7400/api/v1/skills` → `{"skills":[], "tenant_id":"019e5057-…", "total_count":0}`
- `curl http://127.0.0.1:7400/api/v1/verticals` → `{"verticals":[]}` (different envelope from the legacy `/verticals` route which still returns 33 loaded verticals under `{loaded:[…], errors:[], ok:true}`)

Gateway uptime at audit time was 64.5 ks (~17.9h) — well past the 0739 audit's 61 ks reading, but still pre-dating PR #2407 ("seed demo marketplace skill catalog on gateway boot") which has been merged but never picked up by a restart. The SPA's `fetchSkills` falls back to `SEED_SKILLS` only in dev (`process.env.NODE_ENV !== 'production'`) — dev demo works via fallback but the camera sees `data-source="mock-offline"` data, not function-side gateway data. The Scene 4 narration "the registry is real" is technically true (the SPA doesn't 404) but the count is dressing-side fallback, not function-side gateway. Confirmed live in `/tmp/workflows-html.txt`: `data-source="mock-offline"` is the rendered value.

- Reproduce:
  - `curl -s http://127.0.0.1:7400/api/v1/skills` and `curl -s http://127.0.0.1:7400/api/v1/verticals`. Both return empty arrays.
- File paths: gateway boot logic in `crates/argo-cli/src/handlers/gateway/`.
- Suggested fix: restart the gateway via `make restart-live-gateway` (or `scripts/restart-live-gateway.sh`) to pick up #2407's boot-seed. Then ship the open `fix-gateway-skills-and-verticals-boot-seed-tenant-resolution` brief to align the `/api/v1/verticals` envelope to `{loaded, errors, ok}`. After restart, the SPA flips from `data-source="mock-offline"` to `gateway` and Scene 4's narration lands a function-side real-feed.

### F7 [P1] — `/workflows/build` vertical picker still omits Marketing and Compliance + Project Ops

`web/app/(app)/workflows/build/page.tsx:4-10` hardcodes five vertical entries: `sales / support / gtm-engineer / people-ops / finance`. The cohorts page (`/cohorts`) shows seven department tabs including Marketing and Compliance and a Project & Ops tab (per memory `project_pm_cohorts_peta_patrick`). The catalog page already links to `/workflows/project-ops/weekly_standup_digest` (one of the 8 seeds!) — confirmed in the live SSR HTML — so the vertical picker is **out of sync with the actual seed catalog**. Operator hitting "Start from blank" cannot pick the vertical that already owns one of the eight live workflows. For the v1.0 demo this is dressing-side (Scene 5 picks an existing workflow, not a fresh one), but for any post-demo customer click-through it is a visible gap and a real internal inconsistency.

- Reproduce:
  - Open `/workflows/build`. Observe five vertical cards (no Project Ops).
  - Open `/workflows`. Observe one card linking to `/workflows/project-ops/weekly_standup_digest`.
- File path: `web/app/(app)/workflows/build/page.tsx:4-10`
- Suggested fix: import the canonical vertical list from the gateway `/verticals` envelope (or at minimum from the union of `SAMPLE_WORKFLOW_SUMMARIES.vertical` values), not a hardcoded array. Add `project-ops` minimum to align with the existing seed.

### F8 [P1 — new] — Workflow viewer + builder canvas SSR return skeleton-only HTML — page title is the section name, not the workflow name

`/workflows/sales/qualify_lead` SSR returns `<title>Workflows · SKYCOT</title>` with only `workflow-viewer-skeleton-step-0..4` testids in the HTML body — no workflow name, no step names, no Run-now button, no run history. `/workflows/sales/qualify_lead/build` SSR returns the same title + `workflow-builder-skeleton-node-0..2`. The Skill detail page `/skills/pricing.competitor_scrape` SSRs the skill name in the `<title>` (correct) but its `og:title` still says "Skill marketplace — SKYCOT" (parent route's metadata, not the skill's). This is a **dressing-side** SEO/share-card issue — the camera doesn't read the title bar during Scene 5 — but the underlying problem (the workflow viewer is fully client-rendered, no SSR'd content) means the demo will show the loading skeleton for ~1-2 seconds on cold load, and any share-link the operator pastes into Slack will show a generic SKYCOT card instead of the specific workflow.

- Reproduce:
  - `curl -s http://127.0.0.1:3000/workflows/sales/qualify_lead | grep -oE '<title>[^<]+</title>'` → `<title>Workflows · SKYCOT</title>`.
  - `curl -s http://127.0.0.1:3000/skills/pricing.competitor_scrape | grep -oE 'og:title[^>]*content="[^"]*"'` → `og:title" content="Skill marketplace — SKYCOT"`.
- File paths: `web/app/(app)/workflows/[verticalId]/[name]/page.tsx` (needs `generateMetadata` per-workflow), `web/app/(app)/skills/[skillId]/page.tsx` (same, per-skill)
- Suggested fix: add a `generateMetadata` export to each dynamic route that resolves the workflow/skill name from the gateway (or seed fallback) and stamps `title` + `og:title` + `og:description`. For the demo specifically: pre-warm `/workflows/sales/qualify_lead` (or whichever workflow Scene 5 lands on) before recording so the skeleton flash is gone.

### F9 [P1] — Even with #2415 shipped, the catalog has no demonstrator card — Scene 5 needs F3 to land for the new step kinds to be visible

#2415 wired the new `mcp_invoke` + `cu_action` step kinds into the builder palette (Plug + MonitorSmartphone icons, 29-connector MCP group, dedicated inspectors, TOML round-trip in `step-encoder.ts`, `propose-toml.ts` AI-streaming support). But the eight seed workflows in `web/lib/sample-workflows.ts` use only `trigger / skill_invoke / decision / tool_call / notify / approval / data / code` — `rg '"mcp_invoke"|"cu_action"' web/lib/sample-workflows.ts` returns zero hits. So the builder palette is wired but **the catalog screen has nothing to demonstrate it on**. The Scene 5 narration "the MCP plus computer-use combo" can show the palette items but cannot show a workflow card that actually runs them. F3 + the spec edit there is the unblock.

- Reproduce:
  - Open any existing workflow's `/build` route; inspect the palette → MCP connectors group + Computer-use action tool item both present in `web/app/(app)/workflows/[verticalId]/[name]/build/types.ts:274-326`.
  - Open `/workflows`; none of the eight cards uses these step kinds.
- File paths: `web/app/(app)/workflows/[verticalId]/[name]/build/types.ts:236-326` (palette confirmed wired) vs `web/lib/sample-workflows.ts` (no consumer)
- Suggested fix: fold the seed work into F3 (one PR adds `customer_research_followup` summary + spec + uses `mcp_invoke` for the Notion / Slack call + uses `cu_action` for the Notes.app step + sets `substrate: "macos"`). Single seed change unblocks F3 + F5 + F9 at once.

### F10 [P2] — Workflow card author chip distribution still imbalanced (now Jamal×3, not Mariah×3)

All eight seed-workflow author chips counted from `web/lib/sample-workflows.ts:lastEditor` lines: Mariah×2, Priya×1, Jamal×3 (approve_refund, triage_ticket, morning_pipeline_digest), Nori×1, Peta×1. Jamal now appears on three cards — same over-indexing pattern as the prior audit, just with a different worker. Splitting these across the rest of the roster (and including Patrick once, per memory `project_pm_cohorts_peta_patrick`) would land a more diverse author-chip canon on the catalog screen. Dressing-side (worker personas are fictional fixtures), but the camera sweeps the catalog.

- Reproduce:
  - Open `/workflows`. Count author chips: Jamal appears on 3 of 8 cards.
- File path: `web/lib/sample-workflows.ts:72,82,92,103,113,124,135,146`
- Suggested fix: rebalance to roughly Mariah×2, Priya×2, Jamal×2, Nori×1, Peta×1 (or include Patrick once and trim Jamal to 2).

### F11 [P2] — Workflow viewer's risk pill uses bg-color HEX literals that ignore brand tokens

`web/app/(app)/workflows/page.tsx:36-39` hardcodes `RISK_COLOR = {"approval-required": "#f59e0b", autonomous: "#14b8a6"}`. The locked brand identity (per memory `project_skycot_brand_identity` + `project_brand_consistency_pass`) uses violet → pink gradient. The current amber/teal pills feel off-brand and disconnect from the workers-page gradient cards. Polish, not blocker.

- File path: `web/app/(app)/workflows/page.tsx:31-39`
- Suggested fix: route risk colors through CSS variables tied to the brand token system.

### F12 [P2 — newly surfaced] — `/workflows` page has no SSR for card rosters either — `data-source="mock-offline"` is the source of truth even when the gateway is live

The catalog page SSRs all 8 card markup directly (so the camera sweep works without a client-render race), but it stamps `data-source="mock-offline"` on the wrapping container because `fetchWorkflows` resolves to the seed fallback when the gateway returns empty. After the F6 gateway restart, this stamp will flip to `gateway` (or `mock-bypass` per `web/lib/api/workflows.ts`). For the recording, **pre-warming the catalog after the F6 restart is the only way the camera reads function-side data on Scene 5**. If the operator records pre-restart, the catalog sweep is dressing-side fallback. Per the function-vs-dressing rule, the catalog sweep is the camera's primary Scene-5 frame — so the stamp matters.

- Reproduce:
  - `curl -s http://127.0.0.1:3000/workflows | grep -oE 'data-source[^>]*=\\?"[^"]+"'` → currently `data-source="mock-offline"`.
- File path: `web/app/(app)/workflows/page.tsx` (where `source` is read) + dev-server caching behaviour
- Suggested fix: tie demo recording to the F6 gateway restart sequence; verify `data-source="gateway"` in the SSR HTML before recording Scene 5.

### F13 [P3] — Skill card icon rotation uses `Sparkles / Wrench / Briefcase / TrendingUp / Mail` regardless of skill semantics

`web/app/(app)/skills/page.tsx:32-38` defines `ICON_ROTATION` as a five-icon array applied by index modulo. The pricing-competitor-scrape card gets a Sparkles icon, invoice-extract gets Briefcase, etc. — semantically wrong (pricing should be a chart icon, invoice should be a receipt icon). Camera will read these in passing; not load-bearing. Existing brief `fix-skills-marketplace-card-icon-rotation-semantic-name-map.md` covers this.

- File path: `web/app/(app)/skills/page.tsx:32-38`
- Suggested fix: switch to a `name → icon` map keyed off the skill prefix (`pricing.* → TrendingUp`, `invoice.* → Receipt`, etc.).

### F14 [P3] — Workflow TRY chips ("approval flows / data extraction / outbound qualification / escalation") still multi-word — half zero-result against current seeds

`web/app/(app)/workflows/page.tsx:29` defines `TRY_CHIPS = ["approval flows", "data extraction", "outbound qualification", "escalation"]`. Substring-matched against the eight seed workflows' `name + vertical + description`: `approval flows` matches `approve_refund`'s description ("approval"), `data extraction` matches nothing, `outbound qualification` matches nothing, `escalation` matches `triage_ticket`'s description. Half the chips zero-result like the Skills chips did before #2431 (Skills page TRY chips have already been trimmed to single-word stems: `approval`, `invoice`, `candidate`, `deal`, `support`, `lead` — confirmed at `web/app/(app)/skills/page.tsx:23-30`). Same pattern, same fix shape; the impact is far smaller because Scene 5 narration does not direct the operator to click these chips.

- File path: `web/app/(app)/workflows/page.tsx:29`
- Suggested fix: rewrite to single-word stems: `approval`, `extraction`, `qualification`, `escalation` (or rename the chips to match the seed catalog's vertical names: `sales`, `support`, `finance`, `gtm`).

## Resolved since the 2026-05-27 0739 prior audit

- **F8 from 0739 audit (workflow viewer run-history fabricated rows) RESOLVED.** The viewer at `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:395-410` now renders an honest empty-state ("Demo workflow · no run history. Live run history appears after this workflow is served by the gateway.") in the `isDemoRunHistory` branch. The prior audit flagged this as theatre-the-camera-might-see-if-it-scrolls-down on the workflow detail page; that risk is closed.

All other 0739 findings persist: F1 search beat, F2 install button, F3 customer_research_followup workflow, F4 gateway 403, F5 substrate chips, F6 gateway empty, F7 vertical picker, F9 no demonstrator card, F10 author chips, F11 risk pill HEX, F12 icon rotation, F13 multi-word TRY chips on workflows. F4 is upgraded from "may break" to "will visibly fail with a red toast" because tracing the SPA's `handleRunWorkflow` confirms the gateway's 403 message text surfaces directly through sonner. Two new findings added: F8 (workflow + builder pages SSR skeleton-only with wrong page title; skill detail OG title leaks parent route) and F12 (`/workflows` page is `data-source="mock-offline"` until F6 restart lands — recording sequence matters).

## Orientation score

**50 / 100.** Up 2 from the prior 48/100 thanks to F8 (run-history empty-state landing properly). The Skills marketplace renders a coherent A1 marketplace layout with single-word TRY chips. The workflow builder palette is first-class for MCP + CU (post-#2415) — the Plug icon group + Computer-use Tools item land the right vocabulary on the canvas. The workflow viewer's demo run-history now reads as honest emptiness, not theatre.

The other half — Scene 4 search-to-install path (F1+F2), Scene 5 `customer_research_followup` workflow + the MCP+CU demonstrator card (F3 + F9), the substrate chips telling the "Any-PC" story (F5), the gateway 403 on Run-now (F4 — now confirmed as a visible red sonner toast), and the `mock-offline` data source on `/workflows` (F12) — is still hollow or actively wrong. The two locked zombie-claim briefs (`fix-skills-detail-install-button-and-flow` + `fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds`) are still the canonical fix vehicles; they need the headless-worker `claude /login` outage cleared by the operator before any agent picks them up.

**Three highest-leverage operator-only actions, in order**: (a) restart the gateway via `make restart-live-gateway` to pick up #2407's boot-seed (flips F6 + F12 in one move — gateway becomes the source of truth for both /skills and /workflows); (b) bind `human:demo@skycot.com` to an owner-role override in the demo workspace at boot so F4's 403 disappears for the SPA's Run-now (the curl probe confirmed it isn't a CSRF or cookie issue — the gateway's capability engine rejects the principal outright); (c) clear the headless-worker `claude /login` outage so F2 + F3 (the two pre-authored briefs) can finally pick up and merge — without those two seeds Scene 4 and Scene 5 cannot be recorded as the runbook scripts them.
