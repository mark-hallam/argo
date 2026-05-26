# Scene 4 + 5 UX audit — Claim 4 (Skills shared + learned) + Claim 5 (Any-PC execution)

**Audit run**: 2026-05-27 0739 Melbourne (AEST)
**Scope**: `/skills` marketplace + search + install flow, `/skills/[skillId]` detail, `/workflows` catalog, `/workflows/[v]/[n]` viewer, `/workflows/build` vertical picker, `/workflows/[v]/[n]/build` canvas + palette.
**Methodology**: Started the Next.js dev server via Claude Preview MCP (Model Context Protocol — Anthropic's tool-integration standard). Drove the live SPA for `/skills` search + detail + `/workflows` listing; the in-app navigation timed out repeatedly on heavy DnD-canvas-bearing builder routes (consistent with the documented memory-pressure pattern in `CLAUDE.md`). Backfilled with static `rg` + `Read` against `web/` + a few `curl` probes against the gateway on port 7400 (schema v83, FRESH). Compared against the 4 of the 5 prior open findings from the 0623 audit + the `git log` since then. Function-vs-dressing rule applied per `docs/skycot/demos/v1-demo-positioning-poc-vs-public.md`.

## TL;DR

Material change since the 0623 audit: **F4 (no MCP step kind / no computer-use step kind) is RESOLVED** — #2415 ("wire MCP connector catalog + cu_action into the builder palette") merged at T06-47 this morning. The builder palette now ships a 29-connector MCP group + a discrete "Computer-use action" Tools item with Plug + MonitorSmartphone icons, plus dedicated inspectors and TOML round-trip in `step-encoder.ts`. Everything else is unchanged. Three core P0 demo-blockers remain live: Scene 4 search beat still returns zero results for the literal runbook phrase `competitor pricing scrape`; the skill detail page still has no Install button; Scene 5's `customer_research_followup` workflow still exists only in a vitest mock fixture (not in the 8 sample-summaries, not in `WORKFLOW_TEMPLATES`, not on the live gateway). A new live finding (F4 below): the gateway `POST .../execute` returns **403 forbidden — principal lacks the required workflow capability** when probed with the standard headers — a real Scene-5 Run-now risk that needs an SPA-path smoke before recording. Counts: **3 P0**, **5 P1**, **2 P2**, **2 P3**. Orientation **48 / 100** — up 4 from the prior 44/100 thanks to #2415.

## Headline blockers

| # | Severity | One-line finding | Scene impact |
|---|---|---|---|
| F1 | P0 | Searching `competitor pricing scrape` (literal runbook wording) returns zero results — substring filter misses word-order mismatch with seeded `pricing.competitor_scrape` description ("competitor product pricing") | Scene 4 search beat shows empty grid + "No skills match 'competitor pricing scrape'" |
| F2 | P0 | Skill detail page (`/skills/[skillId]`) has no Install / Add to my workspace / Use this skill button — only Provenance + Workers-using + adoption count + Back link | Scene 4 "click Install" has no target |
| F3 | P0 | `customer_research_followup` workflow exists only in the `SubWorkflowPicker.test.tsx` vitest mock — not in `SAMPLE_WORKFLOW_SUMMARIES`, not in `WORKFLOW_TEMPLATES`, not on the live gateway. None of the 8 seeded workflows uses the new `mcp_invoke` / `cu_action` step kinds either, so even with #2415 wired the catalog has no card to demonstrate the MCP+CU combo. | Scene 5 has nothing to click |
| F4 | P0 (newly surfaced) | Gateway `POST /api/v1/verticals/sales/workflows/qualify_lead/execute` returns 403 forbidden — "principal lacks the required workflow capability" — with the standard `x-tenant-id: demo` + `x-argo-principal: human:demo@skycot.com` headers. SPA-path may still work via the session-derived role, but Scene 5's "click Run now → audit row" beat is at risk if demo-mode session does not bind to owner/admin. | Scene 5 Run-now may fail mid-recording with no audit row landing |

## All findings, in order

### F1 [P0] — Scene 4 search beat returns zero results because word order doesn't substring-match the seed

`web/app/(app)/skills/page.tsx:65-78` filters via case-insensitive substring match across `name`, `description`, and `layer`. The Scene 4 anchor row in `web/lib/api/skills.ts:64-73` is `pricing.competitor_scrape` with description starting "Scrape competitor product pricing across listed channels…". The Scene 4 runbook directs the operator to type literally `competitor pricing scrape`. None of `name`, `description`, or `layer` contains that exact substring in that word order — the description has `competitor product pricing`, not `competitor pricing scrape`. Confirmed live in the SPA at 0742: count pill flipped to `7 skills · 0 shown` and the `skills-empty` panel rendered ("No skills match 'competitor pricing scrape'…"). Typing the single token `competitor` returns the expected 1 card. The camera reads the count pill + the empty-state copy directly.

- Reproduce:
  - Open `/skills`. Type `competitor pricing scrape` into the hero search.
  - Observe the count pill flip to `7 skills · 0 shown` and the empty-state panel render.
- File paths: `web/app/(app)/skills/page.tsx:65-78` (filter), `web/lib/api/skills.ts:64-73` (seed wording)
- Suggested fix: either (a) prepend the description with "Competitor pricing scrape —" so the literal phrase substring-matches, OR (b) change the runbook talk-track to type `competitor scrape` (which substring-matches both the name and the description). The runbook edit is the five-minute fix.

### F2 [P0] — Skill detail page has no Install button — Scene 4 "click Install" has no target

`web/app/(app)/skills/[skillId]/page.tsx:29-122` renders the locked A1 detail surface — eyebrow + title + Provenance section (author chip + TrustGradeChip) + Workers-using list + adoption count + a single "← Back to the marketplace" link. Confirmed live in the SPA at 0743: no element in the DOM matches /install|add to|use this/i. The Scene 4 runbook directs the operator to click Install after landing on the result; there is nothing to click. The open brief `fix-skills-detail-install-button-and-flow` continues to zombie-claim through the headless-worker `claude /login` outage flagged in evaluator round 63 — the operator-side fix is the load-bearing unblock.

- Reproduce:
  - Open `/skills`, click any card (e.g. `pricing.competitor_scrape`).
  - Observe: only Provenance + Workers + Back link; no Install button.
- File path: `web/app/(app)/skills/[skillId]/page.tsx:29-122`
- Suggested fix: add an Install button bound to `POST /api/v1/skills/{name}/install` (or equivalent tenant-scoped binding); on success persist into the workspace's skill library so the worker-detail page's Skills tab shows the new row. Pre-authored brief is the canonical fix vehicle.

### F3 [P0] — `customer_research_followup` workflow does not exist anywhere bindable

`rg customer_research_followup web/lib/ web/app/` returns hits only inside `web/app/(app)/workflows/[verticalId]/[name]/build/SubWorkflowPicker.test.tsx:40,55,64,69` — a vitest mock fixture for the sub-workflow picker. It is **not** in `web/lib/sample-workflows.ts` (the eight `SAMPLE_WORKFLOW_SUMMARIES` are post_listing / screen_candidate / approve_refund / qualify_lead / triage_ticket / morning_pipeline_digest / clean_customer_list / weekly_standup_digest — none match). It is **not** in `web/lib/workflow-templates.ts`. The live gateway returns `{"verticals":[]}` on `/api/v1/verticals`. Confirmed live in the SPA at 0744: the `/workflows` page renders 8 cards, none labeled customer_research_followup. The open brief `fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds` continues to zombie-claim per evaluator round 63 — same headless-worker outage as F2. Now that #2415 has shipped the step-kind plumbing, the only remaining work for Scene 5 is the seed itself + the spec that uses an `mcp_invoke` + a `cu_action`.

- Reproduce:
  - Open `/workflows`. Observe 8 cards — none contains "customer research".
  - `rg customer_research_followup web/lib/` — no hits outside the test fixture.
- File paths: `web/lib/sample-workflows.ts` (add the seed summary + spec), `web/lib/workflow-templates.ts` (also clone a template), and a gateway-side seed.
- Suggested fix: add `customer_research_followup` to `SAMPLE_WORKFLOW_SUMMARIES` with `vertical: "sales"`, `lastEditor: "Peta"` (per the vitest fixture), `risk: "approval-required"`, plus a matching `WorkflowSpec` entry under `SAMPLE_WORKFLOW_SPECS` with 5-7 steps **including at least one `kind: "mcp_invoke"` step (e.g. Notion or Slack lookup) and at least one `kind: "cu_action"` step (e.g. opens Notes.app via macOS computer-use)**. The two new kinds already ride through the encoder, the substrate inference, the inspectors, and the palette — they just have no consumer yet in the seed. Add `substrate: "macos"` explicitly to fix F5 on the same card.

### F4 [P0 — newly surfaced] — Gateway run-now returns 403 forbidden with standard demo headers — Scene 5 Run-now beat is at risk

Live curl probe at run time: `curl -X POST -H 'x-tenant-id: demo' -H 'x-argo-principal: human:demo@skycot.com' http://127.0.0.1:7400/api/v1/verticals/sales/workflows/qualify_lead/execute` returned `403 forbidden — principal lacks the required workflow capability`. The gateway capability check at `crates/argo-cli/src/handlers/gateway/routes_workflows.rs:254-265` binds the principal to a role via `CapabilityEngine`, then runs `engine.check(principal, requested)` — both can fail. The SPA path may still succeed because `web/lib/api/client.ts` derives the principal from the next-auth session and the demo-bypass cookie path can elevate the role; **but the curl 403 says the gateway is not "everyone-on-demo gets owner"**. The Scene 5 runbook narrates "click Run now → audit row lands in 5s"; a 403 mid-recording would force a re-take. Per the function-vs-dressing rule, this is **function-side** — the click-through must reach a real workflow turn and audit row.

- Reproduce:
  - From the operator's shell: `curl -s -X POST -H 'x-tenant-id: demo' -H 'x-argo-principal: human:demo@skycot.com' http://127.0.0.1:7400/api/v1/verticals/sales/workflows/qualify_lead/execute` → 403.
  - In the SPA: open `/workflows/sales/qualify_lead`, click Run now, watch dev-tools network for the execute POST + audit feed for a new row.
- File paths: `crates/argo-cli/src/handlers/gateway/routes_workflows.rs:254-265` (capability check), `web/lib/api/client.ts:140-198` (principal + CSRF wiring), `web/lib/api/workflow-execute.ts:34-44` (POST shape)
- Suggested fix: do one live SPA-path smoke of Run-now → audit-row-appears before recording. If the SPA path also 403s, the operator can either (a) bind `human:demo@skycot.com` to an owner-role override in the demo workspace, or (b) wire a demo-only fast-path that bypasses the capability check when `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1`. **Smoke first, fix only if it actually fails.**

### F5 [P1] — Every visible substrate chip reads "HEADLESS · IDLE"

`web/lib/sample-workflows.ts:64-149` defines eight `SAMPLE_WORKFLOW_SUMMARIES`. None of them sets the `substrate` field. The fallback at `web/lib/api/workflows.ts:334-353` infers substrate from explicit `workflow.substrate` (none of the eight set it) or by scanning `name + description + steps[].description` for tokens `cu_action / cu. / cu_ / computer-use / macos / notes app / windows / linux / e2b / sandbox / browser / web page`. `rg substrate|mcp_invoke|cu_action web/lib/sample-workflows.ts` returns **zero hits** — so all eight default to `headless`. The catalog screen the camera sweeps during Scene 5 has zero visual indication that any of these workflows targets macOS / browser / E2B / Windows. The "Any-PC execution" claim (Claim 5) is invisible on the catalog. The substrate chip renders on every card the camera sweeps, so it counts as function-side per the drill-in test.

- Reproduce:
  - Open `/workflows`. Read the substrate chip on every card.
- File paths: `web/lib/sample-workflows.ts:64-149` (add `substrate: "macos"` or description tokens) + `web/lib/api/workflows.ts:374-393` (inference dict)
- Suggested fix: in the F3 seed for `customer_research_followup` add `substrate: "macos"` (or description text like "Drive the operator's Notes app via computer-use"). Also add `substrate: "browser"` directly to one of the existing seeds (e.g. `qualify_lead`) for cross-substrate variety — the camera then reads at least two different substrate chips on the catalog screen.

### F6 [P1] — Gateway returns empty skills + verticals list on `/api/v1/*`

Live probe at run time (with `x-tenant-id: demo` + `x-argo-principal: human:demo@skycot.com`):
- `curl http://127.0.0.1:7400/api/v1/skills` → `{"skills":[], "tenant_id":"019e5057-…", "total_count":0}`
- `curl http://127.0.0.1:7400/api/v1/verticals` → `{"verticals":[]}` (different envelope from the legacy `/verticals` route which returns 33 loaded verticals under `{loaded:[…], errors:[], ok:true}`)

PR #2407 ("seed demo marketplace skill catalog on gateway boot") was merged but the gateway binary on port 7400 (uptime ~61 ks at audit time) has not been restarted — so `/api/v1/skills` still returns the empty envelope. The SPA's `fetchSkills` falls back to `SEED_SKILLS` only in dev (`process.env.NODE_ENV !== 'production'`) — dev demo works via fallback but the camera sees `data-source="mock-offline"` data, not function-side gateway data. The Scene 4 narration "the registry is real" is technically true (the SPA doesn't 404) but the count is dressing-side fallback, not function-side gateway.

- Reproduce:
  - `curl -s http://127.0.0.1:7400/api/v1/skills` and `curl -s http://127.0.0.1:7400/api/v1/verticals`. Both return empty arrays.
- File paths: gateway boot logic in `crates/argo-cli/src/handlers/gateway/`.
- Suggested fix: restart the gateway via `make restart-live-gateway` to pick up #2407's boot-seed. Then ship the open `fix-gateway-skills-and-verticals-boot-seed-tenant-resolution` brief to align the `/api/v1/verticals` envelope to `{loaded, errors, ok}`. After restart, the SPA flips from `data-source="mock-offline"` to `gateway` and Scene 4's narration lands a function-side real-feed.

### F7 [P1] — `/workflows/build` vertical picker omits two verticals — Marketing and Compliance not selectable

`web/app/(app)/workflows/build/page.tsx:4-10` hardcodes five vertical entries: `sales / support / gtm-engineer / people-ops / finance`. The legacy `/verticals` gateway response lists 33 verticals loaded. The cohorts page (`/cohorts`) shows seven department tabs including Marketing and Compliance. Operator hitting "Start from blank" cannot pick those two. For the v1.0 demo this is dressing-side (Scene 5 picks an existing workflow, not a fresh one), but for any post-demo customer click-through it is a visible gap.

- Reproduce:
  - Open `/workflows/build`. Observe five vertical cards.
- File path: `web/app/(app)/workflows/build/page.tsx:4-10`
- Suggested fix: import the canonical vertical list from the gateway `/verticals` envelope or the cohorts source-of-truth, not a hardcoded array.

### F8 [P1] — `/workflows/[v]/[n]` viewer's run history pulls from `mock-offline` source whenever gateway 404s

`web/app/(app)/workflows/[verticalId]/[name]/page.tsx:104-106` derives `isDemoRunHistory = source === "mock-offline" || source === "mock-bypass"` — and when the gateway returns no workflow row, the SPA falls through to seed and the run-history table renders fabricated rows. The page renders a "DEMO RUN HISTORY" pill in that mode (honest dressing), but the rows themselves are theatre. If Scene 5 drills into the run history during the audit pass-through, the camera will read fabricated rows. Per the function-vs-dressing rule, this is function-side if the camera clicks a row — currently rated P1 because Scene 5 narrates `/audit` (not the workflow-local run-history table) as the audit destination.

- Reproduce:
  - Open `/workflows/sales/qualify_lead`. Scroll to "Run history". Look for the source pill.
- File path: `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:104-106` + the `WorkflowRunsTable` it renders.
- Suggested fix: when the run-history fallback is active, suppress the table (render an empty-state) instead of showing fabricated rows. This matches `/audit`'s gateway-falls-back-empty pattern.

### F9 [P1] — Even with #2415 shipped, the catalog has no demonstrator card — Scene 5 needs F3 to land for the new step kinds to be visible

#2415 wired the new `mcp_invoke` + `cu_action` step kinds into the builder palette (Plug + MonitorSmartphone icons, 29-connector MCP group, dedicated inspectors, TOML round-trip in `step-encoder.ts`, `propose-toml.ts` AI-streaming support). But the eight seed workflows in `web/lib/sample-workflows.ts` use only `trigger / skill_invoke / decision / tool_call / notify / approval / data / code` — `rg '"mcp_invoke"|"cu_action"' web/lib/sample-workflows.ts` returns zero hits. So the builder palette is wired but **the catalog screen has nothing to demonstrate it on**. The Scene 5 narration "the MCP plus computer-use combo" can show the palette items but cannot show a workflow card that actually runs them. F3 + the spec edit there is the unblock.

- Reproduce:
  - Open any existing workflow's `/build` route; inspect the palette → MCP connectors group + Computer-use action tool item both present.
  - Open `/workflows`; none of the eight cards uses these step kinds.
- File paths: `web/app/(app)/workflows/[verticalId]/[name]/build/types.ts:236-326` (palette confirmed wired) vs `web/lib/sample-workflows.ts` (no consumer)
- Suggested fix: fold the seed work into F3 (one PR adds `customer_research_followup` summary + spec + uses `mcp_invoke` for the Notion / Slack call + uses `cu_action` for the Notes.app step + sets `substrate: "macos"`). Single seed change unblocks F3 + F5 + F9 at once.

### F10 [P2] — Workflow card author chip credits Mariah/Priya/Jamal/Nori/Peta — Mariah is over-indexed (3 of 8 cards)

All eight seed-workflow author chips: Mariah (3), Priya (1), Jamal (3), Nori (1), Peta (1). Mariah appearing on three cards including `post_listing` (Marketplace) and `morning_pipeline_digest` and `qualify_lead` overloads her persona — she is already the canonical Scene 1+2 worker. Splitting these across the rest of the roster (and including Patrick once, per memory `project_pm_cohorts_peta_patrick`) would land a more diverse author-chip canon on the catalog screen. Dressing-side (worker personas are fictional fixtures), but the camera sweeps the catalog.

- Reproduce:
  - Open `/workflows`. Count author chips on the eight cards.
- File path: `web/lib/sample-workflows.ts:64-149`
- Suggested fix: rebalance to roughly Mariah×2, Priya×2, Jamal×2, Nori×1, Peta×1 (or include Patrick once and trim Mariah to 2).

### F11 [P2] — Workflow viewer's risk pill uses bg-color HEX literals that ignore brand tokens

`web/app/(app)/workflows/page.tsx:36-39` hardcodes `RISK_COLOR = {"approval-required": "#f59e0b", autonomous: "#14b8a6"}`. The locked brand identity (per memory `project_skycot_brand_identity` + `project_brand_consistency_pass`) uses violet → pink gradient. The current amber/teal pills feel off-brand and disconnect from the workers-page gradient cards. Polish, not blocker.

- File path: `web/app/(app)/workflows/page.tsx:31-39`
- Suggested fix: route risk colors through CSS variables tied to the brand token system.

### F12 [P3] — Skill card icon rotation uses `Sparkles / Wrench / Briefcase / TrendingUp / Mail` regardless of skill semantics

`web/app/(app)/skills/page.tsx:32-38` defines `ICON_ROTATION` as a five-icon array applied by index modulo. The pricing-competitor-scrape card gets a Sparkles icon, invoice-extract gets Briefcase, etc. — semantically wrong (pricing should be a chart icon, invoice should be a receipt icon). Camera will read these in passing; not load-bearing. Existing brief `fix-skills-marketplace-card-icon-rotation-semantic-name-map.md` covers this.

- File path: `web/app/(app)/skills/page.tsx:32-38`
- Suggested fix: switch to a `name → icon` map keyed off the skill prefix (`pricing.* → TrendingUp`, `invoice.* → Receipt`, etc.).

### F13 [P3] — Workflow TRY chips ("approval flows / data extraction / outbound qualification / escalation") still multi-word — half zero-result against current seeds

`web/app/(app)/workflows/page.tsx:29` defines `TRY_CHIPS = ["approval flows", "data extraction", "outbound qualification", "escalation"]`. Substring-matched against the eight seed workflows' `name + vertical + description`: `approval flows` matches `approve_refund`'s description ("approval"), `data extraction` matches nothing, `outbound qualification` matches nothing, `escalation` matches `triage_ticket`'s description. Half the chips zero-result like the Skills chips did before #2431. Same pattern, same fix shape — though the impact is far smaller because Scene 5 narration does not direct the operator to click these chips. Worth a follow-on cleanup brief mirroring the #2431 Skills chip fix.

- File path: `web/app/(app)/workflows/page.tsx:29`
- Suggested fix: rewrite to single-word stems: `approval`, `extraction`, `qualification`, `escalation` (or rename the chips to match the seed catalog's vertical names: `sales`, `support`, `finance`, `gtm`).

## Resolved since the 2026-05-27 0623 prior audit

- **F4 from 0623 audit (no MCP step kind + no computer-use step kind in palette) RESOLVED** — #2415 "wire MCP connector catalog + cu_action into the builder palette" merged at T06-47. Confirmed via `rg "mcp_invoke|cu_action|StepKind" web/app/(app)/workflows/[verticalId]/[name]/build/types.ts`: `StepKind` now includes both arms, `STEP_KIND_META` has Plug + MonitorSmartphone icons, `SIDEBAR_PALETTE_GROUPS` grew from 6 to 7 groups (added "MCP connectors" generated from the 29-connector `CONNECTOR_CATALOG`), Tools group gained "Computer-use action" item, new `McpInvokeInspector` + `CuActionInspector` files exist, `step-encoder.ts` round-trips the new TOML fields. Builder is now first-class for MCP + CU — but no seed workflow uses them yet (see F9 above).

All other 0623 findings (F1 search beat, F2 install button, F3 customer_research_followup workflow, F5 substrate chips, F6 gateway empty, F7 vertical picker, F8 Run-now CSRF check, F9 mock-offline run history, F10 Mariah-over-indexed, F11 risk pill HEX, F12 icon rotation, F13 TRY chips) remain open. F8 from the 0623 audit (Run-now CSRF question) is now upgraded to F4 in this audit and elevated to P0 because the curl probe surfaced a concrete 403 from the gateway path — a real demo-day risk that needs an SPA-path smoke before recording.

## Orientation score

**48 / 100.** Up 4 from the prior 44/100. The Skills marketplace renders a coherent A1 marketplace layout with working TRY chips (post-#2431). The workflow builder palette is now first-class for MCP + CU (post-#2415) — the Plug icon group + Computer-use Tools item land the right vocabulary on the canvas. That's roughly two-thirds of the surface working. The other third — Scene 4 search-to-install path (F1+F2), Scene 5 `customer_research_followup` workflow + the MCP+CU demonstrator card (F3 + F9), the substrate chips telling the "Any-PC" story (F5), and the now-surfaced 403 on the gateway execute path (F4) — is hollow or risky. The two locked zombie-claim briefs (`fix-skills-detail-install-button-and-flow` + `fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds`) are still the canonical fix vehicles; they need the headless-worker `claude /login` outage cleared by the operator before any agent picks them up. Two highest-leverage operator-only actions: (a) restart the gateway via `make restart-live-gateway` to pick up #2407's boot-seed (flips F6 from `mock-offline` to function-side `gateway`); (b) smoke-test SPA-path Run-now before recording to confirm whether the gateway 403 (F4) is operator-session-shape only or a real Run-now blocker.
