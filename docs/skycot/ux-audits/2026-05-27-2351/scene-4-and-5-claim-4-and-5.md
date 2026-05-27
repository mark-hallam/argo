# Scene 4 + 5 UX audit — Claim 4 (Skills shared + learned) + Claim 5 (Any-PC execution)

**Audit run**: 2026-05-27 23:51 Melbourne (AEDT)
**Scope**: `/skills` marketplace + search + install flow, `/skills/[skillId]` detail, `/workflows` catalog, `/workflows/[v]/[n]` viewer, `/workflows/build` vertical picker, `/workflows/[v]/[n]/build` canvas + palette + save round-trip, `/workflows/templates`, `/skills-canvas` (redirect target).
**Methodology**: Claude Preview MCP (Model Context Protocol — Anthropic's tool-integration standard) drove the live dev server on port 61039. Preview chrome was unstable and self-navigated three times during the run; cross-checked everything against direct `curl` against `http://localhost:61039` (the SPA dev server) + `http://127.0.0.1:7400` (the gateway, schema_version=83) + read-only source review under `~/code/argo/web/`. Compared against the 6-hour-prior audit at `docs/skycot/ux-audits/2026-05-27-0519/scene-4-and-5-claim-4-and-5.md` and the 30+ hours of zombie-claim churn in `git log` to identify what has and hasn't shipped. Function-vs-dressing rule applied per `docs/skycot/demos/v1-demo-positioning-poc-vs-public.md`.

## TL;DR

The four headline P0 demo-blockers from the 0519 audit are still live; nothing in the skills/workflows surface has shipped between audits (every recent commit is `claim`/`release (zombie; no PR opened)` churn). Scene 4's search beat still returns zero results when the operator types the runbook wording — confirmed live in the preview: `count` flipped to `7 skills · 0 shown` and the `skills-empty` panel rendered "No skills match 'competitor pricing scrape'". The skill detail page still has no Install button anywhere (no testid, no element, no API client — `web/lib/api/skill-install.ts` doesn't exist). The `customer_research_followup` workflow now **exists on the gateway** (a real change since prior audit: gateway returns it under `vertical_id="marketplace"`, with `substrate=macos` and a `cu.run_macro` step), but the SPA cannot reach it: dev mode short-circuits to the local seed via `isAuthBypassed()` (because `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` is set in `web/.claude/launch.json`), and even with that flag off, the SPA's `discoverVerticalIds()` derives verticals from the local seed which doesn't include `marketplace`. A separate finding: the workflow Run-now button is conditionally `disabled` unless `source === "gateway"` — but in dev/demo mode the source is always `mock-bypass`, so Scene 2/Scene 5's "click Run now" beat would camera-render a disabled button on every seeded card. Counts: **5 P0**, **6 P1**, **3 P2**, **2 P3**. Orientation **44 / 100** — up 2 points from 0519 because the gateway now seeds `customer_research_followup` (one half of the SCN-5 fix), but the SPA-side data-source plumbing means the camera still doesn't see it.

## Headline blockers

| # | Severity | One-line finding | Scene impact |
|---|---|---|---|
| F1 | P0 | Searching `competitor pricing scrape` (literal runbook wording) returns zero results — substring filter misses word-order mismatch with seeded `pricing.competitor_scrape` | Scene 4 search beat shows empty grid + "No skills match 'competitor pricing scrape'" |
| F2 | P0 | Skill detail page (`/skills/[skillId]`) has no Install button — only Provenance + Workers-using + adoption count + Back link; no `skill-install.ts` API client in repo | Scene 4 "click Install" has no target |
| F3 | P0 | `customer_research_followup` workflow exists ONLY on the gateway (under `vertical_id="marketplace"`); SPA never queries `marketplace` (not in `discoverVerticalIds()`) AND short-circuits to local seed via `isAuthBypassed()` | Scene 5 has nothing in `/workflows` to click |
| F4 | P0 | Run-now button on workflow viewer disabled whenever `source !== "gateway"`; in dev/demo the source is `mock-bypass`, so the button camera-renders as greyed-out for every seeded card | Scene 2 + Scene 5 "click Run now" beats break |
| F5 | P0 | All 8 seeded workflow cards render `data-substrate="headless"` — verified live in SSR HTML. The "Any-PC execution" Claim-5 story has zero visual indicator in the catalog | Scene 5 sweep across `/workflows` shows uniform HEADLESS chips, no macOS/browser/E2B visible anywhere |

## All findings, in order

### F1 [P0] — Scene 4 search beat returns zero results because word order doesn't substring-match the seed

`web/app/(app)/skills/page.tsx:65-78` filters via case-insensitive `String.includes()` across `name`, `description`, and `layer`. The Scene-4 anchor row in `web/lib/api/skills.ts:64-73` is `pricing.competitor_scrape` with description starting "Scrape competitor product pricing across listed channels…". The Scene 4 runbook tells the operator to type literally `competitor pricing scrape`. None of `name`, `description`, or `layer` contains that exact substring in that word order — the description has `competitor product pricing`, not `competitor pricing scrape`. **Live preview confirms** (this run): typing the phrase flips `<span data-testid="skills-count">` to `7 skills · 0 shown` and renders the `skills-empty` panel "No skills match 'competitor pricing scrape'. Try a different search or clear the filter." The camera reads this directly.

- Reproduce:
  - Open `/skills`. Type `competitor pricing scrape` into the hero search.
  - Observe count flip to `0 shown` and the empty-state panel render.
- File paths: `web/app/(app)/skills/page.tsx:65-78` (filter), `web/lib/api/skills.ts:64-73` (seed), `web/app/(app)/skills/page.tsx:177-181` (empty-state copy)
- Suggested fix: either (a) prepend the seed description with "Competitor pricing scrape —" so the substring match lands, OR (b) change the runbook talk-track to type `competitor scrape` (which substring-matches both the name and the description). The runbook edit is the five-minute fix.

### F2 [P0] — Skill detail page has no Install button — Scene 4 "click Install" has no target

`web/app/(app)/skills/[skillId]/page.tsx:29-122` renders the locked A1 detail surface — eyebrow + title + Provenance section (author chip + `TrustGradeChip`) + Workers-using list + adoption count + a single "← Back to the marketplace" link. There is no Install / Add to my workspace / Use this skill button anywhere on the page. Live testid scan returns only `skill-provenance`, `skill-workers-using`, `skill-adoption-count` — no `install-button` or similar. Static repo scan confirms `web/lib/api/skill-install.ts` does not exist. The Scene 4 runbook directs the operator to click Install after landing on the result; there is nothing to click. Brief `fix-skills-detail-install-button-and-flow` has been zombie-claimed repeatedly per the recent `git log` (claim/release cycles every ~10-15 min) and never opens a PR — root cause is the headless-worker `claude /login` outage per evaluator round 63, but the operator-only fix unblocks it.

- Reproduce:
  - Open `/skills`, click any card (e.g. `pricing.competitor_scrape`).
  - Observe: no Install button. Only Provenance + Workers + Back link.
  - `curl -s http://localhost:61039/skills/pricing.competitor_scrape | grep -oE 'data-testid="[^"]+"'` → no install testid.
- File paths: `web/app/(app)/skills/[skillId]/page.tsx:29-122`
- Suggested fix: add an Install button bound to a `POST /api/v1/skills/{name}/install` (gateway-side endpoint exists per `crates/argo-cli/src/handlers/gateway/`); on success persist into the workspace's skill library so the worker-detail page's Skills tab shows the new row. The existing zombie-claimed brief is the canonical fix vehicle; it needs the operator to clear the headless-worker outage.

### F3 [P0] — `customer_research_followup` workflow is gateway-seeded but unreachable from the SPA

**New since 0519 audit**: gateway now returns the workflow at `GET /api/v1/workflows` (total_count=1), seeded under `vertical_id="marketplace"`, `substrate="macos"`, with three steps including a `tool_call` step using `tool="cu.run_macro"` for the macOS Notes app. **Live confirmation**:
```
curl http://127.0.0.1:7400/api/v1/workflows
→ {"total_count":1,"workflows":[{"name":"customer_research_followup","vertical_id":"marketplace","substrate":"macos",...}]}
```

The function-side of the seed is real (this is good — it's the half-fix progress since 0519). **The SPA cannot reach it for two reasons**:

1. `web/lib/api/workflows.ts:133-135` short-circuits at the top of `fetchAllWorkflows()` when `isAuthBypassed()` returns true, returning `SAMPLE_WORKFLOW_SUMMARIES` (eight local seeds — `post_listing / screen_candidate / approve_refund / qualify_lead / triage_ticket / morning_pipeline_digest / clean_customer_list / weekly_standup_digest`) and never calling the gateway. The launch.json sets `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1`, making this path always-true in dev/demo.
2. Even if the bypass were off, `web/lib/api/workflows.ts:125-131` `discoverVerticalIds()` derives the per-vertical fan-out list from `SAMPLE_WORKFLOW_SUMMARIES.map(wf => wf.vertical)`. None of the 8 seed entries has `vertical: "marketplace"`, so the SPA never fans out a `GET /api/v1/verticals/marketplace/workflows` call — the gateway data is invisible.

Live SSR confirms: `/workflows` HTML contains 8 `workflow-card` divs, `data-source="mock-offline"` on the eyebrow pill, and zero hits on `customer_research_followup` in the rendered grid.

- Reproduce:
  - Open `/workflows`. Observe 8 cards; none labelled customer_research_followup.
  - `curl http://localhost:61039/workflows | grep customer_research_followup` → no hits.
  - `curl http://127.0.0.1:7400/api/v1/workflows` → the workflow is there with vertical_id=marketplace.
- File paths: `web/lib/api/workflows.ts:109-113` (`isAuthBypassed`), `:125-131` (`discoverVerticalIds`), `:133-135` (bypass short-circuit), `web/lib/sample-workflows.ts:64-149` (seed list, lacks marketplace).
- Suggested fix: lowest-risk is to **add `customer_research_followup` directly into `SAMPLE_WORKFLOW_SUMMARIES`** with `vertical: "sales"` (or wherever the runbook talks about it), description that contains the substrate-inference tokens "Notes app" + "computer-use" so `inferSubstrateFromText` flips substrate to `macos`, and `lastEditor: "Peta"` (already promised by the vitest fixture). Higher-leverage fix: also (a) include `"marketplace"` (or the gateway's actual vertical_id) in `discoverVerticalIds()` so live-mode catches it, AND (b) make `isAuthBypassed()` no longer suppress gateway reads (use the bypass to inject a principal header, not skip the network entirely).

### F4 [P0] — Run-now button stays disabled because the SPA reads "mock-bypass" not "gateway"

`web/app/(app)/workflows/[verticalId]/[name]/page.tsx:106` sets `const isLive = source === "gateway";`. Line 257 disables the Run-now button with `disabled={running || !isLive}`. The tooltip on disabled (line 261) reads "Available once this workflow is published to the gateway". In dev/demo, `isAuthBypassed()` returns true so the workflow viewer's `useWorkflow` hook resolves `source` to `mock-bypass` for every seed-served workflow — meaning the Run-now button camera-renders as **disabled** on every card the operator would click. Scene 2 (Mariah's duty Run-now beat) and Scene 5 (the `customer_research_followup` Run-now beat) both depend on the operator visibly clicking Run-now and watching audit rows land. If the button is greyed out, the demo can't run.

This is a recent regression in semantics — F9 in the 0519 audit flagged CSRF as a possible problem; the actual problem is upstream (the source-pill gating).

- Reproduce:
  - Open `/workflows/sales/qualify_lead` in the SPA.
  - Inspect the Run-now button data-testid="wfd-run-workflow"; in dev/demo it is `disabled` with `data-source="mock-bypass"`.
- File paths: `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:106-107` (`isLive`), `:252-265` (button + tooltip)
- Suggested fix: either (a) flip `isLive` to `source !== "mock-offline"` (so both gateway and mock-bypass can run — mock-bypass in dev is a deliberate operator choice), OR (b) make `fetchWorkflow` in dev mode call the gateway through and let `mock-bypass` mean "auth-bypass auto-applied" not "skip network". The first is the demo-day fix; the second is the principled fix.

### F5 [P0] — All 8 seeded workflow cards render `substrate=headless` — "Any-PC execution" story invisible

Live SSR confirms (this run):
```
curl -s http://localhost:61039/workflows | grep -oE 'data-substrate="[a-z]+"' | sort | uniq -c
→ 8 data-substrate="headless"
```

`web/lib/api/workflows.ts:334-353` (`resolveWorkflowSubstrate`) defaults to `"headless"` when no explicit substrate is set and the workflow's name + description + steps text doesn't match any of the inference tokens (`cu_action / cu. / cu_ / computer-use / macos / notes app / windows / linux / e2b / sandbox / browser / web page`). None of the 8 entries in `SAMPLE_WORKFLOW_SUMMARIES` sets `substrate` explicitly, and none of their descriptions contains any of those tokens. Result: the `/workflows` catalog sweep during Scene 5 has zero visual indication that **any** of these workflows targets macOS / browser / E2B / Windows. The "Any-PC execution" claim (Claim 5) is invisible to the camera.

- Reproduce:
  - Open `/workflows`. Read substrate chips on every card.
- File paths: `web/lib/sample-workflows.ts:64-149` (add `substrate: "macos"` directly OR add description tokens), `web/lib/api/workflows.ts:374-393` (inference dict — token list is the authoritative seed)
- Suggested fix: bundle with F3 — when adding `customer_research_followup` to the seed, set `substrate: "macos"` explicitly AND add description text mentioning "Notes app" / "computer-use" so the inference also fires. For variety, also add `substrate: "browser"` to one of the existing seeds (e.g. `qualify_lead`'s description mentions "CRM update" — could be reworded to "browser-drive the CRM").

### F6 [P1] — Skill marketplace TRY chips now too vague — "approval" matches 1 skill but "skills my workers learned this month" matches 0 in the workspace

`web/app/(app)/skills/page.tsx:23-30` now uses single-noun chips (`approval / invoice / candidate / deal / support / lead`). This is a chip-text fix since 0519 (good progress on the prior F6) — each chip now substring-matches exactly one skill seed. **But** the seventh chip "Skills my workers learned this month" toggles `learnedOnly` against `useSkillsLearnedThisMonth().data?.learned.skill_ids`, which in dev/demo with no real learn-events resolves to an empty set → clicking the chip flips the page to empty. F6 stands as a P1 because: (a) it's a TRY pill that's visibly broken (sets count to 0 shown); (b) the demo operator may accidentally click it on the sweep; (c) the 6 single-noun chips deliver one match each — fine for casual browse but visually thin compared to the original "approval routing / invoice extraction / ..." copy that promised richer matches.

- Reproduce:
  - Open `/skills`. Click "Skills my workers learned this month". Observe `0 shown`.
- File paths: `web/app/(app)/skills/page.tsx:23-30` (chip list), `web/lib/hooks/use-skills.ts` (`useSkillsLearnedThisMonth` hook — seeded empty in dev)
- Suggested fix: in dev mode, populate `useSkillsLearnedThisMonth` with 2-3 seeded skill_ids so the chip lands real matches. Polish, not a demo-blocker.

### F7 [P1] — Gateway returns 7 verified skills but SPA still shows 7 seeds; data-source pill says "mock-bypass" not "gateway"

Live state has shifted from 0519: `curl http://127.0.0.1:7400/api/v1/skills` now returns **7 marketplace skills with full provenance** (`provenance_source = "published:verified:argo-seed"`, `trust_grade = "published_verified"`, version 0.1.0). PR #2407's gateway boot-seed is alive. **But** the SPA shows the same 7 seeds because `isAuthBypassed()` short-circuits `fetchSkills()` (`web/lib/api/skills.ts:175-178`) to `SEED_SKILLS` before any gateway call. The seven seeds happen to match the gateway's seven 1:1 by name, so the camera sees the right cards — but the eyebrow pill reads `data-source="mock-bypass"` not `data-source="gateway"`. The Scene 4 narration "the registry is real" is technically false on the dev path; if the operator records with `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=0`, the gateway feed lands and the pill flips.

- Reproduce:
  - `curl http://127.0.0.1:7400/api/v1/skills | jq '.skills | length'` → 7.
  - `curl http://localhost:61039/skills | grep data-source` → `mock-bypass` in dev.
- File paths: `web/lib/api/skills.ts:163-178` (bypass), `web/.claude/launch.json` (sets `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1`)
- Suggested fix: same as F3's principled fix — make `isAuthBypassed()` inject a principal header and let the gateway answer, instead of treating bypass as "skip network entirely". Pair with launch.json env-var rework.

### F8 [P1] — `/workflows/build` vertical picker still omits Marketing and Compliance + the new "marketplace" vertical

`web/app/(app)/workflows/build/page.tsx:4-10` still hardcodes 5 verticals: `sales / support / gtm-engineer / people-ops / finance`. The `customer_research_followup` workflow lives at `vertical_id="marketplace"` on the gateway — the only seeded workflow there — and the operator can't even reach that vertical via the "Start from blank" entry point. Cohorts page (`/cohorts`) lists 7 dept tabs including Marketing and Compliance, plus the new Sales & Investor Relations cohort, which all need vertical mappings. Persistent gap since 0519.

- Reproduce:
  - Open `/workflows/build`. Observe 5 vertical cards.
- File path: `web/app/(app)/workflows/build/page.tsx:4-10`
- Suggested fix: import the canonical vertical list from the gateway's `/api/v1/verticals` envelope (which returns `[{id:"marketplace", name:"Marketplace", workflow_count:1}]` and could expand as more verticals seed) or from the cohorts source-of-truth.

### F9 [P1] — `useSubWorkflowPicker` references `customer_research_followup` only as a test fixture — Scene 5 sub-workflow chain unreachable

`web/app/(app)/workflows/[verticalId]/[name]/build/SubWorkflowPicker.test.tsx:40,55,64,69` is the **only** in-tree reference to `customer_research_followup` outside the gateway seed. `SubWorkflowPicker.tsx` itself uses `useWorkflows()` for its candidate list (line 79) → falls back to `SEED_WORKFLOWS` (line 7, sourced from `./seed-workflows`). Neither path includes `customer_research_followup`. Scene 5 demonstrates the chain "MCP fetch → summarise → CU drop-note"; if the operator wanted to assemble that chain from sub-workflow building blocks, the picker can't find them.

- Reproduce:
  - `grep -rn customer_research_followup web/` → only the .test fixture.
- File paths: `web/app/(app)/workflows/[verticalId]/[name]/build/SubWorkflowPicker.tsx:7-95`, `web/app/(app)/workflows/[verticalId]/[name]/build/seed-workflows.ts`
- Suggested fix: bundles into the F3 fix.

### F10 [P1] — Builder palette has MCP + CU step kinds (F4 from 0519 shipped) — but the workflow viewer doesn't surface them distinctively when rendered

Verified via source read: `web/app/(app)/workflows/[verticalId]/[name]/build/types.ts:25-39` now defines `StepKind` with `mcp_invoke` and `cu_action`. The `SIDEBAR_PALETTE_GROUPS` array at `:274-335` exposes them: under "Tools" there's `tool-cu-action` ("Computer-use action · Drive a real desktop app (macOS — Notes, Mail, etc.)"), and a dedicated "MCP connectors" group enumerated from `CONNECTOR_CATALOG`. `STEP_KIND_META` at `:221-222` gives them distinct icons (`Plug` for MCP, `MonitorSmartphone` for CU) and brand colors (`#0891b2` cyan / `#8b5cf6` violet). `step-encoder.ts:99-122` round-trips MCP+CU through TOML. `propose-toml.ts:25-28` accepts AI-proposal language ("mcp", "computer_use") and maps to the right kind.

**The shipped half**: the operator can author MCP+CU steps in the canvas, and they persist through save → reload via the gateway TOML round-trip. F4 from 0519 ships.

**The unshipped half**: the workflow viewer (`page.tsx` step timeline at `:322-329`) uses `STEP_KIND_META` icons by step kind — so if a workflow contains `mcp_invoke` steps the timeline icons render correctly. But none of the 8 seeded workflows uses these step kinds, so Scene 5's visible timeline still shows generic `skill_invoke / tool_call / approval / notify`. The promise "MCP connector calls + computer-use" is invisible on every card the operator could record. Combined with F3 (gateway workflow unreachable), Scene 5's screenshot would still show no canonical MCP+CU example.

- File paths: viewer rendering `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:322-340`; seed gap `web/lib/sample-workflows.ts:64-149`.
- Suggested fix: paired with F3 — add `customer_research_followup` to the seed with explicit `mcp_invoke` step (e.g. `zendesk:search_tickets`) and `cu_action` step (`notes:new_note`); when the viewer renders, the Plug + MonitorSmartphone icons land in the timeline visibly.

### F11 [P1] — Workflow detail page shows runs panel + history requires `isLive` to render (Scene 5 audit-row beat broken)

`web/app/(app)/workflows/[verticalId]/[name]/page.tsx:130-136` makes the runs query conditional on `isLive` (same flag as the disabled Run-now). With `mock-bypass` source, no runs query fires, no runs render. Line 423 (`{isLive ? <RunsPanel /> : ...}`) shows a different surface for non-live workflows. The Scene 5 talk-track "click Run → audit rows land in `/audit`" depends on the runs query rendering live data. With auth-bypass on, this whole panel is bypassed.

- File paths: `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:106, 134, 423`
- Suggested fix: same as F4 — fix `isLive` semantics so mock-bypass-in-dev still treats the gateway as the data source.

### F12 [P2] — Risk-pill hex literals on workflow viewer still ignore brand tokens

`web/app/(app)/workflows/[verticalId]/[name]/page.tsx:218-221` (workflow viewer header) and `web/app/(app)/workflows/page.tsx:36-39` (catalog risk colors) both hardcode `"approval-required": "#f59e0b"` and `autonomous: "#14b8a6"`. Locked brand identity is violet → pink gradient (per memory `project_skycot_brand_identity` + `project_brand_consistency_pass`). The current amber/teal pills feel off-brand. Polish, not blocker — but stands as a recurring finding (was F12 in 0519, still unaddressed).

- File paths: `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:218-224`, `web/app/(app)/workflows/page.tsx:31-39`
- Suggested fix: route risk colors through CSS variables tied to the brand token system. Polish-tier brief.

### F13 [P2] — Templates page lists 50+ entries but no `customer_research_followup` clone template — Scene 5 operator cannot create one even via templates

`web/lib/workflow-templates.ts` lists 50+ template definitions (confirmed via the templates page testid scan: 57+ `template-card-*` entries spanning compliance / eng / finance / sales / support / people-ops / marketing). None of them is named or described as `customer_research_followup` or anything similar (no `customer_research`, no `mcp + cu combo`, no Zendesk-followup template). The templates page offers no path to the Scene 5 demo workflow either.

- File path: `web/lib/workflow-templates.ts`
- Suggested fix: add a `template-card-marketplace-customer-research-followup` entry mirroring the gateway seed. P2 because Scene 5 doesn't navigate through `/workflows/templates`; the operator clicks a card from `/workflows` directly.

### F14 [P2] — Demo-mode banner persists across all in-product pages and crowds the top of the workflow viewer

The "Demo mode — showing sample data. Sign in to see your own workspace." pill still renders persistently above every in-app page. F14 from 0519 unchanged. Not breaking, but the camera reads it during every scene. Worth a `?demo=hide` toggle for the recording or a `NEXT_PUBLIC_HIDE_DEMO_BANNER` env-var.

- Suggested fix: toggle via query param or temporarily comment out the banner during recording.

### F15 [P3] — Skill card icon rotation still semantically wrong (sparkle on pricing, briefcase on invoice)

`web/app/(app)/skills/page.tsx:32-38` defines `ICON_ROTATION` as a five-icon array applied by `idx % 5`. The pricing-competitor-scrape card gets a Sparkles icon (should be TrendingUp), invoice-extract gets Briefcase (should be Receipt). Unchanged since 0519.

- File path: `web/app/(app)/skills/page.tsx:32-38`
- Suggested fix: switch to a name → icon map keyed off the skill prefix.

### F16 [P3] — `/workflows/runs` and `/workflows/edit` still 404 (not yet redirected)

Live curl confirms: `GET /workflows/runs → 404`, `GET /workflows/edit → 404`. `/skills-canvas` was fixed (`next.config.ts:46-50` now redirects `/skills-canvas → /skills`). The two `/workflows/*` dead routes remain unrouted. Not navigated to during the demo, but appear in the dev-server log on every audit-feed refresh if something hits them.

- Reproduce: `curl -s -o /dev/null -w "%{http_code}\n" http://localhost:61039/workflows/runs` → 404.
- File path: `web/next.config.ts:42-55` (redirect block) — add two more entries.
- Suggested fix: add `{ source: "/workflows/runs", destination: "/workflows", permanent: false }` and similar for `/workflows/edit`.

## Resolved since the 2026-05-27 0519 prior audit

- **Gateway seed for `customer_research_followup` (half of F3)** — gateway now returns the workflow with `substrate=macos` and a `cu.run_macro` step. SPA-side data plumbing not yet hooked through (F3 above).
- **Gateway skills boot-seed (half of F7)** — gateway now returns 7 verified marketplace skills. SPA still short-circuits to local seed in dev (F7 above).
- **`/skills-canvas` 307 redirect to `/skills`** — `next.config.ts:46-50` shipped between audits. F11 from 0519 partly closed.
- **TRY chips on `/skills` simplified** — single-noun chips now substring-match the seed (F6 from 0519 partly closed; learned-this-month chip new failure mode noted in F6 above).

## Sanity checks on the function-vs-dressing rule

Per `docs/skycot/demos/v1-demo-positioning-poc-vs-public.md`, the test is "does the camera drill into this?" + "function-side or dressing-side?"

- F1, F2, F3, F4, F5 — all function-side (Scene 4 search beat, Install click, Scene 5 workflow click, Run-now button, substrate display the camera DOES sweep). Real P0s.
- F6 — function-side but Scene 4 narration tells the operator to *type* not click chips. Stays P1.
- F7 — function-side (the "registry is real" narration is camera-audible) but the camera doesn't read the data-source pill, just the card content. The cards happen to match between seed and gateway, so the camera sees right content; the inner truth is wrong. P1.
- F11 — function-side. The Scene-5 audit-rows beat depends on it. P1 only because Scene 5's audit drill-in happens in Scene 6 narratively; the camera does see runs render here though.
- F12, F13, F14 — dressing-side or off-camera. P2-P3.
- F15, F16 — dressing-side. P3.

## Orientation score

**44 / 100.** Up 2 points from the 0519 audit because the gateway-side seed for `customer_research_followup` + the 7 marketplace skills + the MCP/CU step kinds in the builder palette are real shipped progress on the function side. **But** the SPA's data-plumbing in dev mode (`NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` → short-circuit to local seeds → `mock-bypass` source-pill → `isLive=false` → Run-now disabled) means the camera still sees none of it. The two zombie-claimed briefs (`fix-skills-detail-install-button-and-flow` + `fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds`) remain the canonical fix vehicles; both need the headless-worker outage cleared. A new shipping priority is suggested: a tiny brief that fixes `isAuthBypassed()` semantics in `web/lib/api/{workflows,skills}.ts` so the bypass injects a principal header instead of skipping the network — this single fix would convert ~3 of the P0 findings to passing (F3 gateway-seed reaches the SPA, F4 Run-now becomes enabled because source flips to `gateway`, F7 data-source pill flips to gateway).
