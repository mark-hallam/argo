# Scene 4 + 5 UX audit — Claim 4 (Skills shared + learned) + Claim 5 (Any-PC execution)

**Audit run**: 2026-05-27 0415 Melbourne
**Scope**: `/skills` marketplace + search + install flow, `/workflows` index + cards, `/workflows/build` canvas, computer-use / MCP indicators (Claim 5)
**Methodology**: live SPA via Claude Preview MCP (initially), then curl-against-running-SPA + gateway probes + source read (the preview browser hung mid-session and would not recover). Cross-checked against prior audit `docs/skycot/ux-audits/2026-05-26-0820/scene-4-and-5-claim-4-and-5.md`.
**Function-vs-dressing rule** applied per `docs/skycot/demos/v1-demo-positioning-poc-vs-public.md`.
**Trigger**: caller specifically asked whether PR #2407 "seed demo marketplace skill catalog on gateway boot" actually fixes the perpetual-loading bug.

## TL;DR

PR #2407 ships **half the fix**: the SPA-side seed got the right rows (`pricing.competitor_scrape` is now in `SEED_SKILLS`, learned-by chips wired, author labels differentiated, Rio/Sam non-canon names removed), but the gateway-side seed is **not running on the live binary** — the gateway at 7400 was built from `ae4041a84` on 2026-05-26 14:33, well **before** PR #2407's commit `a509bc75e` was merged. So `/api/v1/skills` returns `{skills: [], total_count: 0}` and the SPA paints from the mock fallback. Perpetual-loading is no longer visible because the fallback kicks in, but the "real catalog from the real gateway" promise the PR sells is not actually live. **Demo-side**: Scene 4's search beat is still broken in two new ways — the runbook tells the operator to type `competitor pricing scrape` (word order doesn't substring-match `pricing.competitor_scrape` → 0 results), and **every one** of the six TRY chips below the search bar ("approval routing", "invoice extraction", etc.) returns 0 results because the chip text is noun-phrase but skill names + descriptions are verb-stem. Install button still does not exist on the skill-detail page; `customer_research_followup` workflow still does not exist; no first-class `mcp_invoke` / `cu_action` step kind. Counts: **3 P0** (search beat + install button + Scene 5 workflow), **3 P1** (gateway-binary stale, TRY chips all 0-match, no visible CU substrate indicator), **2 P2** (workflow-runs 404 noise, dressing-side polish). Orientation 44 / 100 — up from 36 since the SPA-side seed shipped and worker-name canon is fixed, but the function-side core is still hollow.

## Headline blockers

| # | Severity | One-line finding | Scene impact |
|---|---|---|---|
| F1 | P0 | Searching `competitor pricing scrape` (literal runbook wording) returns 0 results — substring filter misses word-order mismatch with `pricing.competitor_scrape` | Scene 4 search beat (4:00–5:20) shows "No skills match" |
| F2 | P0 | Every TRY chip below the search bar returns 0 results (chip text is noun-phrase, skill names are verb-stem) | Scene 4 fallback if operator clicks a chip instead of typing — also shows "No skills match" |
| F3 | P0 | No Install button on `/skills/[skillId]` — only Provenance + Workers-using + adoption count + back-link | Scene 4 "Click **Install**" has no target |
| F4 | P0 | `customer_research_followup` workflow does not exist in SPA sample seed; no `mcp_invoke` / `cu_action` step kinds | Scene 5 MCP+CU combo (5:20–6:40) has nothing to run |

## All findings, in order

### F1 [P0] — Scene 4 search beat returns 0 results because runbook wording word-orders differently than seed skill names

`web/app/(app)/skills/page.tsx:65-78` defines the filter as a case-insensitive substring match across `name`, `description`, and `layer`. The PR #2407-added Scene-4 anchor row is `pricing.competitor_scrape` with description starting "Scrape competitor product pricing…". The Scene 4 runbook in `docs/skycot/demos/v1-mission-mvp-runbook.md` directs the operator to type literally `competitor pricing scrape`. Substring match fails: neither the name nor the description contains the substring `competitor pricing scrape` in that word order. Verified by inlining the filter in Python against the actual seed strings — 0 matches. The count-pill flips to `7 skills · 0 shown` and the grid renders the `skills-empty` panel with "No skills match 'competitor pricing scrape'. Try a different search or clear the filter." Camera reads this directly.
- Reproduce:
  - Open `/skills`, type `competitor pricing scrape` in the hero search.
  - Observe count pill flips to `0 shown` and empty-state panel renders.
  - Source: `web/app/(app)/skills/page.tsx:72-77`, seed at `web/lib/api/skills.ts:64-73`.
- File path: `web/app/(app)/skills/page.tsx:65` (filter) + `web/lib/api/skills.ts:64` (seed wording)
- Suggested fix: change the seed name to `competitor.pricing_scrape` AND prepend the description with "Competitor pricing scrape —" so any reasonable word order matches; OR rename the runbook search beat to type `competitor scrape` (which DOES substring-match `pricing.competitor_scrape` AND the description). The talk-track edit is the 5-minute fix; the seed edit is the structural fix.

### F2 [P0] — All six TRY chips below the search bar match zero seed skills

`web/app/(app)/skills/page.tsx:23-30` defines `TRY_CHIPS = ["approval routing", "invoice extraction", "candidate screening", "deal qualification", "support triage", "lead enrichment"]`. Each chip click sets the search query to the chip text via `setQuery(chip)`. None of these six noun-phrases substring-matches any of the seven seed-skill names (`approval.route`, `invoice.extract`, `candidate.screen`, `deal.qualify`, `support.triage`, `lead.enrich`, `pricing.competitor_scrape`) or descriptions. Verified by inlining the filter against all 6 chips × 7 seeds in Python — 0 of 42 pairs match. If the operator falls back to clicking a chip instead of typing during Scene 4, the result is identical to F1 — empty grid with "No skills match" panel. This is worse than F1 because the chips are presented as a curated "Try this" affordance and visibly degrading them telegraphs to the camera that the search subsystem doesn't work.
- Reproduce:
  - Open `/skills`. Click any of the six TRY chips ("approval routing", "invoice extraction", …).
  - Observe count pill flips to `0 shown` every time.
- File paths: `web/app/(app)/skills/page.tsx:23-30` (chip text) + `web/lib/api/skills.ts:64-143` (seed names)
- Suggested fix: rewrite the chip text to substring-match the seed names — e.g. `approval` / `invoice` / `candidate` / `deal` / `support` / `lead` (one-word chips), OR rewrite the seed descriptions to lead with the chip text ("Approval routing — route approvals…"). One-word chips are the cheapest fix and read naturally.

### F3 [P0] — No skill-install affordance on the marketplace or skill-detail page

Carried over from the 2026-05-26-0820 audit; unchanged. `web/app/(app)/skills/[skillId]/page.tsx:38-122` renders only Provenance + Workers-using + adoption count + a back-link. There is no Install button, no progress strip, no "added to worker" toast. The Scene 4 runbook says "Click **Install** on the verified result" + progress strip "Verifying provenance… Installing skill… Available to workers." None of that exists. The only install-shaped flow in the codebase is `/workers/<id>/skills/new` which opens a Ghostwriter form to *draft a new skill from a prompt* — a different verb from the runbook's "install an existing marketplace skill."
- Reproduce:
  - `curl -s -L http://localhost:3000/skills/pricing.competitor_scrape -o /tmp/d.html`
  - `grep -oE 'data-testid="[^"]*"' /tmp/d.html | sort -u` → only the global app-shell test ids + `skill-provenance` / `skill-workers-using` / `skill-author-label` / `skill-adoption-count` / `skill-workers-empty`. No `skill-install-*`.
- File path: `web/app/(app)/skills/[skillId]/page.tsx:38`
- Suggested fix: add a primary Install button + progress strip + post-install navigation; wire to a `POST /api/v1/skills/<name>/install` route that mirrors `SkillStore::install`. Until this lands Scene 4 is unrecordable as scripted.

### F4 [P0] — Scene 5's `customer_research_followup` workflow not in SPA sample seed; MCP + CU not first-class step kinds

Carried over and verified. `rg "customer_research_followup" web/lib/sample-workflows.ts` → 0 hits. The 7-row SPA sample (`web/lib/sample-workflows.ts:64-138`) is `post_listing`, `screen_candidate`, `approve_refund`, `qualify_lead`, `triage_ticket`, `morning_pipeline_digest`, `clean_customer_list`. The full `customer_research_followup` TOML exists in `crates/argo-demo-seed/src/workflow.rs:30-44` and would seed via `argo demo seed`, but the running gateway has 0 workflows in the `workflows` table (`curl /api/v1/verticals/sales/workflows -H 'X-Argo-Principal: …'` returns `{count: 0, items: [], vertical_id: "sales"}`). So the SPA falls back to the 7-row sample, none of which is the runbook target. Separately, `web/app/(app)/workflows/[verticalId]/[name]/build/types.ts` (StepKind enum) lacks `mcp_invoke` and `cu_action` as distinct kinds — the only CU-ish entry is a generic "Browser action" backed by `tool_call`.
- Reproduce:
  - `rg "customer_research_followup" web/ | grep -v test`
  - `curl -s 'http://127.0.0.1:7400/api/v1/verticals/sales/workflows' -H 'X-Argo-Principal: {"tenant":"019e5057-1d3f-7fa2-9c1b-c34ef01cc21e","sub":"demo"}'`
- File paths: `web/lib/sample-workflows.ts:64`, `web/app/(app)/workflows/[verticalId]/[name]/build/types.ts:22`
- Suggested fix: add a `customer_research_followup` entry to `SAMPLE_WORKFLOW_SUMMARIES` with `vertical: "sales"` + a substrate hint that resolves to `macos` so the SubstrateChip paints; OR rewrite Scene 5 to use one of the existing 7 workflows and edit the runbook talk track. The workflow-builder StepKind addition is a separate, larger brief.

### F5 [P1] — PR #2407 gateway-side seed is shipped to git but the running binary is stale; mock-fallback masks it

The live gateway at port 7400 reports `gateway-built-from: ae4041a84e0797af4e658beed3b1631f20e5db43` (recorded 2026-05-26 14:33). PR #2407's merge SHA `a509bc75e` is **not** an ancestor of `ae4041a84` (`git merge-base --is-ancestor a509bc75e ae4041a84 || echo "stale"` → stale). So `seed_marketplace_if_demo_mode` in `crates/argo-cli/src/handlers/gateway/demo_seed_skills.rs` never runs in the live process; `/api/v1/skills` returns `{skills:[], total_count:0, tenant_id:"019e5057-1d3f-7fa2-9c1b-c34ef01cc21e"}`. The SPA falls back to `SEED_SKILLS` via the `mock-offline` branch (`web/lib/api/skills.ts:181-184`), which paints the catalog. So the camera sees a populated marketplace **but** the function-side promise — "real gateway, real catalog" — is not live. This is dressing-acceptable for a POC investor demo IF the camera does not click into provenance details (the seed-fallback `seedUsage` returns synthetic adoption counts), but it's a P1 because: (a) the PR description claims gateway-side install; (b) a rebuilt gateway is one `cargo build --release -p argo-cli && launchctl restart` away and turns the failed promise into a fulfilled one.
- Reproduce:
  - `cat ~/.argo/gateway-built-from`
  - `git merge-base --is-ancestor a509bc75e ae4041a84e0797af4e658beed3b1631f20e5db43 && echo OK || echo STALE`
  - `curl -s 'http://127.0.0.1:7400/api/v1/skills' -H 'X-Argo-Principal: …'` → `total_count: 0`
- File path: `crates/argo-cli/src/handlers/gateway/boot_serve.rs:91` (seed call site)
- Suggested fix: rebuild the gateway binary (`cargo build --release -p argo-cli`) and restart it. Then re-verify `/api/v1/skills` returns `total_count: 7`. Operator-side action — no code change. Until then, document the mock-fallback path in the demo runbook so the operator knows the catalog visible on-screen is the SPA seed, not the gateway.

### F6 [P1] — No visible computer-use / macOS substrate indicator on workflow detail page

The `SubstrateChip` component (`web/components/skycot/workflow/SubstrateChip.tsx`) exists and renders macos / linux / windows / e2b / browser / headless chips. The workflow **index cards** render this via `<SubstrateChip substrate={substrate} status="idle" compact />` at `web/app/(app)/workflows/page.tsx:266`. Good. But the workflow **detail page** (`/workflows/{vertical}/{name}`) has no substrate chip on the header — `grep -n "SubstrateChip" web/app/(app)/workflows/[verticalId]/[name]/page.tsx` finds the import on line 36 but no usage in the JSX. Scene 5's narration "Right now you're watching the macOS executor drive the Notes app" lands the camera on the detail page, where there is currently no visible reinforcement that the macOS executor is the runtime substrate. The actual Notes-opening behaviour is the load-bearing visual, but for Claim-5 "any-PC execution" the on-screen pill would convert ambient theatre into explicit proof.
- Reproduce: `grep -c "SubstrateChip" web/app/(app)/workflows/\[verticalId\]/\[name\]/page.tsx` → 1 (import only, no render).
- File path: `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:36`
- Suggested fix: render `<SubstrateChip substrate={resolveWorkflowSubstrate(workflow)} status={running ? "running" : "idle"} />` in the workflow-detail header next to the "Run now" button so the camera always sees the executor identity.

### F7 [P1] — `/workflows/{v}/{n}` always fetches `/workflow-runs` even in mock-offline mode (404 noise)

Carried over from the prior audit; unchanged. `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:108-132` fires `fetchWorkflowRuns` regardless of the workflow's data source. Against a gateway with no rows this is a guaranteed 404 per page open — dev-server console pollution and ~200-400ms of extra latency on the page load. Not visible to the camera but it shows up in console-log captures if the operator opens dev-tools mid-demo.
- File path: `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:108`
- Suggested fix: skip `fetchWorkflowRuns` when `source === "mock-offline"`, or surface a graceful 0-row fallback.

### F8 [P2] — "Skills my workers learned this month" chip is wired but needs gateway-up to show data

PR #2407 added `MOCK_LEARNED_SKILL_IDS = ["support.triage", "lead.enrich"]` (`web/lib/api/skills.ts:146`) but the `useSkillsLearnedThisMonth` hook only returns that fallback when the gateway 404s; with the gateway up-but-empty (current state) the hook returns an empty array. Toggling the chip empties the grid because the filter intersects against `[]`. Once the gateway-side seed runs (F5), this resolves naturally — the gateway will return `learned_by` rows for the 2 marketplace skills and the chip will populate. Until then, the chip is silently broken from the camera's perspective.
- Reproduce: with current stale gateway, click "Skills my workers learned this month" chip — grid empties.
- File path: `web/lib/api/skills.ts:283-310` (hook fallback)
- Suggested fix: extend `useSkillsLearnedThisMonth` to return `MOCK_LEARNED_SKILL_IDS` whenever `isAuthBypassed()` OR the gateway envelope is `total_count: 0`. Mirrors the `fetchSkills` mock-offline branch.

### F9 [P2] — Workflow-builder canvas `/workflows/build` is a vertical-picker, not a canvas

The `/workflows/build` route (`web/app/(app)/workflows/build/page.tsx`) renders a 5-tile vertical picker (Sales / Support / GTM / People Ops / Finance) — not a canvas. Clicking a tile routes to `/workflows/{vertical}/untitled/build` which IS the real react-flow canvas. The intermediate step is fine UX but if Scene 5 says "open the builder" and lands at the picker, the camera shows a 5-row choose-vertical card grid rather than the expected canvas. Recordable but a beat lost.
- File path: `web/app/(app)/workflows/build/page.tsx:12`
- Suggested fix: optional — change the demo runbook narration to "open a new sales workflow" and click straight into `/workflows/sales/untitled/build`.

## Resolved since prior audit (2026-05-26-0820)

| # | Resolution |
|---|---|
| Prior F1 (no competitor scrape in SEED_SKILLS) | **Partial.** Skill is now in SEED_SKILLS as the leading row (`web/lib/api/skills.ts:64-73`). But search wording mismatch keeps it unreachable via the runbook's literal search beat (current F1) and TRY chips never reach it (current F2). |
| Prior F4 (Rio + Sam non-canon worker names on workflow cards) | **Fully resolved.** `grep "Rio\|Sam" web/lib/sample-workflows.ts` → 0 hits. All 7 workflows now `lastEditor`-attributed to Mariah / Priya / Jamal / Nori (Peta still absent — Scene 5 won't narrate Peta on a workflow card, so OK). |
| Prior F9 (skill-detail author labels uniformly "Authored by SKYCOT") | **Fully resolved.** PR #2407 added `SEED_AUTHOR_LABEL_BY_NAME` map (`web/lib/api/skills.ts:148-154`) with differentiated labels — "Authored by SKYCOT" for 4 skills, "Authored by your org" for `deal.qualify`, and `learned_by` populated for `support.triage` (Mariah) + `lead.enrich` (Priya). Provenance card now visibly varies across the catalog. |
| Prior F5 (workflows eyebrow no longer says "MOCK") | **Holds.** Eyebrow renders `WORKFLOWS · v0.1`, mock-source moved to `data-source` attr. Unchanged. |
| Prior F2 (no Install button) | **Unchanged.** Current F3. |
| Prior F3 (Scene 5 workflow + MCP/CU step kinds missing) | **Unchanged.** Current F4. |
| Prior F6 (no visible CU substrate indicator) | **Partially resolved on workflow index cards** — `<SubstrateChip>` renders on `/workflows` grid cards via `web/app/(app)/workflows/page.tsx:266`. Workflow detail page still has no chip — see current F6. |

## Orientation score — 44 / 100

Up from 36. The SPA-side half of PR #2407 is in (`pricing.competitor_scrape` seed, learned-by chips, author label differentiation, canon worker names), which removes the perpetual-loading symptom and the cosmetic Rio/Sam mismatch. But the function-side core is still hollow: the running gateway binary doesn't have the boot-seed; the search beat the runbook scripts breaks because the chip + runbook wording doesn't substring-match the seed names; no Install button; no `customer_research_followup` workflow; no first-class MCP / CU step kinds. The two cheapest demo-unblocks (one operator action + one talk-track edit) are: (a) rebuild + restart the gateway so PR #2407's gateway-side seed actually runs; (b) edit the Scene-4 runbook to type a word that substring-matches the seed (`competitor scrape` works, `pricing` works, `scrape` works) and edit the TRY chips to one-word labels. Those two get Scene 4 to "search returns results" without any structural code change. Install button + Scene-5 workflow + CU step kinds remain real engineering briefs.
