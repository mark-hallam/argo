# Whole-site MVP claims-coverage rollup — 2026-05-27 07:39 Melbourne

**Scope**: every SPA route NOT walked by the 4 sibling agents (scene-1-2 / scene-4-5 / scene-6 / onboarding-hire). Marketing (`/landing`, `/pricing`, `/customers`, `/compare`, `/security`, `/sla`, `/changelog`, `/privacy`, `/terms`, `/for/*`, `/docs` + 10 stubbed sub-routes, `/help`), in-app surfaces outside the 6-scene runbook (`/cohorts`, `/settings/*` 11 sub-pages, `/brand`, `/brand/components`, `/team`, `/workflows/templates`, `/verticals`, `/genesis`, `/retro`, `/orchestration`, `/support/inbox`, `/billing`, `/billing/subscribe`, `/skills-canvas` if present), theme provider wiring, viewport-responsiveness markers, plus the load-bearing **6-MISSION-claim coverage rollup**.

**Method**: gateway probes against `http://127.0.0.1:7400` (live; schema v83 same as T0623), source-of-truth `.tsx` + brief-ledger inspection (read-only — no cargo, no SPA dev-server boot), prior-audit DELTA cross-check against `2026-05-27-0623/`, BUILD_STATUS verification of P0 brief ledger rows. Preview chrome was not navigated this run — relying on static-source + gateway evidence + ledger state.

**Working tree**: branch `main`, gateway 7400 schema **v83** (unchanged from T0623), HEAD `0977044fe` ("ux-audit-T0623MEL heartbeat + DELTA report").

**Time since prior whole-site audit**: 76m (T0623 → T0739). **Zero net-new code commits in the interval** — the only commits since T0623 are the T0623 audit's own DELTA + heartbeat. No PRs merged. No P0 briefs claimed. 15 open PRs per conductor heartbeat. This is a **stall interval** — the synthesis briefs from T0519 + T0623 are sitting in queue while workers cycle on lower-priority rows.

---

## TL;DR

19 findings: **3 P0** (skills+verticals seed STILL empty 142 minutes after #2407; `/api/v1/audit` STILL 100% SPA self-traffic — Scene 6 unrecordable; worker-detail Run-now P0 brief CLAIMED but worker zombie-released with no PR — Scene 2 still has no Run-now affordance), **7 P1** (default "Approvals" title leaks across 9+ in-app pages, `/api/v1/workers` collection still 404, `/api/v1/verticals` empty, `/billing/subscribe` only has H1 on error fallback not primary view, `/team` STILL 2 H1s, 7 of 11 settings sub-pages still missing metadata, `/workflows/templates` breaks the `· SKYCOT` title chain), **7 P2** (cohorts SSR empty-flash, retro SSR "Loading retros…", workflows API returns 0, `/genesis` H1 mismatches studio scope, settings shell duplicate-H1, cohorts hero pluralisation flicker, audit feed still 100% self-traffic + no agent-turn rows), **2 P3** (`/skills-canvas` flat route 404, workers/team sidebar label split, customer logos still stub avatars).

**6-MISSION-claim functional verdict from the SPA, T0739** (all flat vs T0623):
- 1 = 🟡 **LIKELY-LIVE** (real worker "Maya" via `/api/v1/team`; `/workers` collection 404 unchanged)
- 2 = ❓ **NOT-OBSERVABLE-FROM-SPA** (audit feed still 100% self-traffic — no agent-turn rows to drill into)
- 3 = 🟡 **LIKELY-LIVE** (`/verticals` seed still empty, wizard reads SEED_VERTICALS fixture)
- 4 = 🔴 **BROKEN-IN-SPA** (skills catalog STILL empty 142 min after #2407; root-cause brief TODO 76 min more)
- 5 = ❓ **NOT-OBSERVABLE-FROM-SPA** (workflows API returns 0; static template cards on `/workflows/templates` are dressing; computer-use cold-rerun owed)
- 6 = 🟡 **LIKELY-LIVE-but-degraded** (cross-tenant backend LIVE; SPA audit-drill-in surface degraded by F3 self-traffic)

**Direction vs T0623** (76m prior): **strictly flat**. 0 PRs merged. 0 P0s claimed. The conductor + supervisor are alive but no fix-cycle landed in the interval. The DELTA report from T0623 named the same headline blockers I'm naming again — verbatim.

**Single biggest unlock for the demo**: still `fix-gateway-skills-and-verticals-boot-seed-tenant-resolution` — same brief, now TODO 140+ minutes. Same root cause unblocks F1 (Scene 4 skills) AND F8 (Scene 3 verticals).

---

## Headline blockers

- **F1 [P0] `/skills` catalog STILL EMPTY 142 minutes after #2407** — same gateway state as T0519 and T0623. `curl http://127.0.0.1:7400/api/v1/skills` → `{"skills":[],"tenant_id":"019e5057-1d3f-7fa2-9c1b-c34ef01cc21e","total_count":0}`. Same payload for `?q=competitor`, `?q=research`, `?q=draft`, `?q=email`. Scene 4 unrecordable. Brief `fix-gateway-skills-and-verticals-boot-seed-tenant-resolution` queued as P0 since 2026-05-26T19:19Z (140 min ago) — still TODO.
- **F2 [P0] `/api/v1/audit` still 100% SPA self-traffic** — re-confirmed at T0739. 10 of 10 rows have `worker.name="unknown"`, `verb="http.request"`, `action_type` is `audit.detail.viewed` or `audit.replay`. Worse than T0623 (5/5) — the audit table now has 10 rows but every row is the SPA reading itself. Scene 6 audit drill-in shows "audit.detail.viewed by unknown" on every row the camera clicks. Function-side fabrication per the function-vs-dressing rule. T0519 synthesis queued `fix-audit-feed-filter-or-bucket-spa-self-traffic-http-request` as P1; **should be P0** (Scene 6 specifically drills into this surface).
- **F3 [P0] Worker-detail `Run-now` action STILL absent + brief released without PR** — T0415MEL P0 brief `fix-worker-detail-run-now-action-and-endpoint` was CLAIMED by a worker, the worker exited zombie without opening a PR (per BUILD_STATUS row), salvageable WIP at `/tmp/fix-worker-detail-run-now-action-and-endpoint`. Scene 2 needs a Run-now button on the worker surface. Brief still ostensibly CLAIMED (not RELEASED back to TODO), so no second worker can pick it up.
- **F4 [P1] `(app)/layout.tsx:20` default title "Approvals" STILL leaks across 9+ in-app pages** — unchanged from T0519, T0623. Verified at `web/app/(app)/layout.tsx:18-22`: `default: "Approvals"`. Confirmed leak surfaces: `/brand`, `/brand/components`, `/retro`, `/genesis`, `/orchestration`, `/verticals`, `/support/inbox`, `/billing`, `/billing/subscribe`. Demo camera reads the tab bar.

---

## All findings, in order

### Public marketing pages

#### F1 [P0] — `/skills` marketplace API STILL empty across all query stems
Re-verified at T0739: `curl -s http://127.0.0.1:7400/api/v1/skills` returns `{"skills":[],"tenant_id":"019e5057-1d3f-7fa2-9c1b-c34ef01cc21e","total_count":0}`. Same for `?q=competitor`, `?q=research`, `?q=draft`, `?q=email`. The cosmetic chip-rewrite #2431 still has nothing to point at. **140 minutes** since the root-cause brief was queued.

Repro:
- `curl -s http://127.0.0.1:7400/api/v1/skills | head -c 200` → empty catalog
- `for q in research email draft competitor ; do curl -s "http://127.0.0.1:7400/api/v1/skills?q=$q" ; done` → all empty
- Brief file `docs/build-plan/streams/fix-gateway-skills-and-verticals-boot-seed-tenant-resolution.md` exists; ledger row `TODO`

Fix: claim the queued P0 brief. Three hypotheses in the brief body (tenant binding / feature gate / idempotency sentinel).

#### F5 [P1] — `/sla` STILL no metadata (T0519 F5 → T0623 F5 — 3 audits unresolved)
`web/app/sla/page.tsx` has no `export const metadata`. Tab reads the root marketing default. 3 × 4-LOC fix unauthored as a brief.

File: `web/app/sla/page.tsx`.

#### F6 [P3] — `/customers` logos still stub avatars (T0519 F7 → T0623 F6 — unresolved)
Per memory `feedback_logos_from_svgl_app`: customer logos must come from svgl.app. Placeholder badges still in place for the 5 fictional customers. Dressing-side; flag only.

#### F7 [P2] — `/docs` 10 sub-routes now stubbed (T0623 F2 CLOSED — #2416 shipped)
Verified `/docs/api/`, `/docs/audit/`, `/docs/getting-started/`, `/docs/integrations/`, `/docs/skills/`, `/docs/workflows/` all carry `page.tsx` (plus 2 deeper drill-in pages: `audit/trust-ratchet`, `getting-started/hire-first-teammate`, `integrations/slack-approvals`, `workflows/visual-canvas`). T0623 F2 is RESOLVED.

### In-app pages (outside 6-scene hot path)

#### F2 [P0] — `/api/v1/audit` 10 of 10 rows = SPA self-traffic (DEGRADED from T0623 F3)
Re-confirmed at T0739. Every audit row in the live feed is `http.request - unknown - audit.detail.viewed` or `audit.replay`. The audit page renders the locked B3 Etherscan-style table — but every row the demo camera could drill into is the SPA's own auth/replay pings, NOT agent turns. Per the function-vs-dressing rule, **this is P0** (Scene 6 specifically drills into this surface).

Repro:
- `curl -s 'http://127.0.0.1:7400/api/v1/audit?limit=10' | python3 -c 'import sys,json; [print(x.get("verb"),"-",x.get("worker",{}).get("name"),"-",x.get("action_type")) for x in json.load(sys.stdin).get("items",[])]'` → 10/10 unknown/http.request
- T0623 had 5/5; T0739 has 10/10 — self-traffic accumulates as audits keep running

Fix: T0519 brief `fix-audit-feed-filter-or-bucket-spa-self-traffic-http-request` is queued P1; **recommend re-classify to P0**. Either filter `worker.name == "unknown"` at gateway list, OR bucket self-traffic into `/audit/system` view.

#### F3 [P0] — Worker-detail `Run-now` action + endpoint STILL absent; brief CLAIMED-zombie
T0415MEL P0 brief `fix-worker-detail-run-now-action-and-endpoint` shows in the ledger as CLAIMED on mac-host at 2026-05-26T19:51:19Z, but the worker zombie-released without opening a PR. Brief sits CLAIMED, so no worker can pick it up. Salvageable WIP per ledger annotation: `/tmp/fix-worker-detail-run-now-action-and-endpoint`. Scene 2 of the runbook has no Run-now affordance.

Fix: release the brief back to TODO + a fresh worker picks up the WIP from `/tmp`.

#### F4 [P1] — `(app)/layout.tsx:20` default title "Approvals" leaks across 9+ pages (T0519 F3 → T0623 F4 — STILL unresolved)
Re-verified at T0739 via `Read` of `web/app/(app)/layout.tsx`: metadata template has `default: "Approvals"`. 9 confirmed leak surfaces unchanged: `/brand`, `/brand/components`, `/retro`, `/genesis`, `/orchestration`, `/verticals`, `/support/inbox`, `/billing`, `/billing/subscribe`. Worst case: `/genesis` (an investor moment per memory `project_pitch_agent_v1_demo`) reads "Approvals · SKYCOT" in the tab.

Fix: change default to `"SKYCOT"` + add `export const metadata` to 9 orphan pages. T0623 queued `fix-settings-sub-pages-add-missing-metadata` for the settings cluster, but the broader `(app)` default-title fix is still unbriefed.

#### F8 [P1] — `/api/v1/workers` collection STILL 404 (T0519 F4 → T0623 F7 — unresolved)
- `curl -s http://127.0.0.1:7400/api/v1/team` → 1 real worker "Maya"
- `curl -s http://127.0.0.1:7400/api/v1/workers` → `{"error":"not found"}`

SPA workers grid falls back to fictional roster (Mariah/Peta/Priya/Jamal/Nori) while the workspace has only "Maya". Route-level 404 brief is still missing (the T0519 brief covers the sidebar label, not the route alias).

File: `crates/argo-cli/src/handlers/gateway/routes_workers_list.rs`.

#### F9 [P1] — `/api/v1/verticals` STILL empty `{"verticals":[]}` (T0519 F10 → T0623 F8 — unresolved)
Same root-cause as F1. `/onboarding/pick-vertical` and `/verticals` fall back to SEED_VERTICALS fixture (5 entries vs the 30+ the gateway is meant to seed). Brief `fix-gateway-skills-and-verticals-boot-seed-tenant-resolution` (P0, TODO since T0519) covers both F1 + F9.

#### F10 [P1] — `/billing/subscribe` primary view STILL has no `<h1>` (T0519 F12 → T0623 F9 — unresolved)
Verified: only `<h1>` in `web/app/(app)/billing/subscribe/page.tsx:64` is on the **error fallback** ("Couldn't start checkout") — primary checkout-redirect view has 0 H1s. Plus the F4 leak: tab reads "Approvals · SKYCOT". T0519 brief `fix-billing-subscribe-missing-h1-and-metadata` queued — TODO.

File: `web/app/(app)/billing/subscribe/page.tsx`.

#### F11 [P1] — `/team` has 2 `<h1>` elements on one page (T0519 F13 → T0623 F10 — unresolved)
Verified via `grep -n '<h1' web/app/(app)/team/page.tsx`: 2 H1s — line 55 `Team` + line 250 (same class). Semantic-HTML violation: screen-reader heading-nav lands on two "Team" anchors.

Fix: drop the secondary H1 OR convert one to `<h2>`.

#### F12 [P1] — 7 of 11 `/settings/*` sub-pages still missing metadata (T0519 F17 → T0623 F11 — unresolved)
Confirmed via `grep -l "export const metadata" web/app/(app)/settings/*/layout.tsx`. Metadata present on:
- `api-keys`, `data-export`, `members`, `security` (4)

Missing on:
- `billing`, `connectors`, `knowledge`, `notifications`, `notify-channels`, `providers`, `workspace` (7)

Each missing-metadata page reads the parent layout's title — tab reads e.g. "Settings · SKYCOT" instead of "Connectors · SKYCOT". T0623 brief `fix-settings-sub-pages-add-missing-metadata` queued P1 — TODO 76m.

#### F13 [P1] — `/workflows/templates` STILL breaks the `· SKYCOT` title chain (T0519 F11 → T0623 F12 — unresolved)
`web/app/(app)/workflows/templates/layout.tsx:5` exports `title: "Workflow templates"` as a string literal. Next.js treats this as an override that doesn't inherit the parent layout's `"%s · SKYCOT"` template. Tab reads "Workflow templates" without the SKYCOT suffix — inconsistent with sibling in-app pages.

Fix: `title: { absolute: "Workflow templates · SKYCOT" }`.

#### F14 [P2] — `/cohorts` SSR STILL shows `CohortsEmptyPreview` even with 1 real cohort (T0623 F13 — unresolved)
Re-verified `web/app/(app)/cohorts/page.tsx:103-127`: rendering ternary is `cohortsQuery.isPending ? skeleton : cohorts.length === 0 ? EmptyPreview : Magazine`. During SSR `isPending=false` AND `cohorts=[]` (no fetch yet) → renders `CohortsEmptyPreview` (which uses `SAMPLE_COHORT_SUMMARIES` — fictional cohort data). Gateway has 1 real cohort `marketing-intern::unknown`. Demo recording captures ~200-500ms of empty-preview flash before hydration completes.

Fix: gate on `cohortsQuery.isSuccess && cohorts.length === 0` rather than `cohorts.length === 0` alone.

#### F15 [P2] — `/cohorts` hero pluralises "0 cohorts" SSR → "1 cohort" post-hydrate (T0623 F14 — unresolved)
`web/app/(app)/cohorts/page.tsx:80-86`: subtitle reads `{total} cohorts` ternary once query resolves. SSR total=0 — visible flicker on first paint. Same fix as F14.

#### F16 [P2] — `/settings/*` sub-pages render duplicate "Settings" `<h1>` from shell + per-page H1 (T0623 F15 — unresolved)
Confirmed across the 4 settings sub-pages with their own H1 (`api-keys`, `data-export`, `members`, `security`) — they all have 2 H1s in rendered HTML: shell-level "Settings" + page-specific heading. The 7 without their own H1 have only the shell "Settings" — no clear top-level heading.

Fix: demote shell "Settings" label to `<h2>` (or move to `<header role="banner">`); make sub-page H1 the primary.

#### F17 [P2] — `/genesis` H1 mismatches operator-clarified studio scope (T0623 F16 — unresolved)
Per memory `feedback_genesis_is_ai_teammate_onboarding_studio` (operator 2026-05-25T11:40Z): Genesis is the AI Teammate Onboarding Studio. Current page hero reads "Hire your next teammate." (per `BrandLoudHero` import + content at top of `web/app/(app)/genesis/page.tsx`). Reads as hire-CTA, not studio.

Fix: H1 → "Onboard a new teammate." or similar.

#### F18 [P2] — `/api/v1/workflows` returns 0 + `/workflows/templates` lists 50+ static cards (T0623 F17 — unresolved)
- `curl -s http://127.0.0.1:7400/api/v1/workflows` → 0 workflows
- `web/lib/workflow-templates.ts` has 50+ hardcoded cards including `competitor_intel_digest`

Scene 5 narration says "the camera clicks Run on `customer_research_followup`" — that exact template name only exists as a test fixture (`SubWorkflowPicker.test.tsx`); runtime workflows API empty. Cards on `/workflows/templates` are dressing; the moment camera runs one, runtime engine must accept the request.

#### F19 [P2] — `/retro` SSR renders "Loading retros…" with no empty-state fallback (T0519 F19 → T0623 F18 — unresolved)
`web/app/(app)/retro/page.tsx:142` literal "Loading retros…" — no skeleton, no empty-state. Out-of-runbook path; polish.

#### F20 [P3] — `/skills-canvas` flat route 404 (T0519 F20 → T0623 F19 — unresolved)
Flat route doesn't exist; only `/workers/[workerId]/skills/[skillId]/canvas` does. e2e agent prompt + several `streams/` briefs reference `/skills-canvas`. Either redirect or update refs.

### Theme + viewport + console hygiene

Static-source verifications (preview chrome not navigated this run):

- **Theme provider wiring** — confirmed at `web/app/layout.tsx:10,114,123`: imports `ThemeProvider` from `@/components/skycot/ThemeProvider` and mounts it inside root layout. `next-themes` is the underlying provider. Wiring correct; interactive toggle smoke test still owed.
- **Viewport responsiveness markers** — `web/components/marketing/MarketingNav.tsx:110` carries `data-testid="marketing-nav-open"` per T0519 closure F24. Marketing hamburger is wired; mobile (375px) interactive smoke still owed.
- **CSP + security headers** — `curl -sI 'http://127.0.0.1:7400/'` returns: HSTS preload (implied), `X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY`, `Referrer-Policy: strict-origin-when-cross-origin`, `Permissions-Policy: geolocation=(), camera=(), microphone=(), payment=()`. Clean.

---

## 6-MISSION-claim coverage rollup (the load-bearing summary)

Reminder per the function-vs-dressing rule (`docs/skycot/demos/v1-demo-positioning-poc-vs-public.md`): "LIVE in the SPA" means a path the demo camera traverses produces real (non-fixture) data and renders the locked design. "LIKELY-LIVE" means the page renders + backend cold tests pass + same-origin proxy is wired (post-#2429) but at least one wire defect prevents end-to-end verification from the browser.

### Claim 1 — Teammate, not tool 🟡 LIKELY-LIVE (flat vs T0623 🟡)

**Evidence**:
- `/api/v1/team` returns 1 real worker "Maya" (`role.vertical = marketing-intern`, tagline "Marketing intern that drafts and asks before publishing.").
- `/api/v1/workers` STILL returns `{"error":"not found"}` — defect age now 142 min cumulative. SPA workers grid falls back to `sample-workers-detail.ts` (5 fictional teammates).
- Backend cold tests pass (round-19, round-54).
- T0623 shipped #2419 (real-shaped UUID + session-local roster) + #2430 (persona.bio populated) + #2411 (drop fake provider-key) — onboarding theater partially closed.

**Citation**: `curl -s http://127.0.0.1:7400/api/v1/team` → `{"total_count":1,"workers":[{"name":"Maya"...}]}`. `curl -s http://127.0.0.1:7400/api/v1/workers` → `{"error":"not found"}`.

**Demo-recording implication**: Scene 1 narrates "Mariah, Peta, Priya, Jamal, Nori" — fictional roster. Real workspace has "Maya". Either runbook re-scripts to "Maya, your marketing intern", or demo-seed CLI gets run with the 5-persona roster (CLAUDE.md item (e) — TODO). F8 (route alias 404) is the gating defect.

### Claim 2 — Delegation is the unit ❓ NOT-OBSERVABLE-FROM-SPA (flat vs T0623)

**Evidence**:
- No `/handoffs` or `/delegations` UI surface in SPA route tree (`ls web/app/(app)/` confirms).
- Audit page IS the only delegation-visualisation surface — but the audit feed is now 100% SPA self-traffic across 10 of 10 rows (F2, degraded from T0623 5 of 5). So even if the row-detail drawer renders handoff hashes, the rows present are auth pings, not agent turns.
- Backend round-54 confirms claim 2 LIVE (delegate-task fallback-chain inheritance via #2256).

**Citation**: `ls ~/code/argo/web/app/(app)/` shows no delegation route; `curl -s http://127.0.0.1:7400/api/v1/audit?limit=10` → 10/10 self-traffic rows.

**Demo-recording implication**: Scene 6 audit drill-in cannot demonstrate delegation chains because there are no agent-turn rows to drill into. Resolving F2 (filter self-traffic) is a prerequisite to making claim 2 observable.

### Claim 3 — Platform builds workers 🟡 LIKELY-LIVE (flat vs T0623 🟡)

**Evidence**:
- 5 onboarding steps SSR cleanly (per T0519 walk; code paths unchanged).
- `/genesis` studio shell present with `BrandLoudHero` + `HeroInput` + `PreviewCard` + `ProgressStrip` — function is in place; H1 copy mismatched per F17.
- `/api/v1/verticals` STILL returns `{"verticals":[]}` — `/onboarding/pick-vertical` falls back to SEED_VERTICALS fixture (5 entries vs 30+ gateway should seed). Brief `fix-wizard-verticals-rewrite-scope-and-dead-fallback` TODO.
- Backend round-54 confirms claim 3 LIVE.

**Citation**: `curl -s http://127.0.0.1:7400/api/v1/verticals` → `{"verticals":[]}`.

**Demo-recording implication**: Scene 3 recordable but pick-vertical reads fixtures. Same root-cause as Claim 4. One brief unblocks both: `fix-gateway-skills-and-verticals-boot-seed-tenant-resolution`.

### Claim 4 — Skills shared + learned 🔴 BROKEN-IN-SPA (flat vs T0623 🔴)

**Evidence**:
- `/skills` catalog STILL returns `{"skills":[],"total_count":0}` 142 minutes after #2407 + #2431 + the T0519+T0623 syntheses queuing the P0 root-cause brief twice.
- Stems tested at T0739: `research`, `email`, `draft`, `competitor` — all return 0 rows.
- Root-cause brief `fix-gateway-skills-and-verticals-boot-seed-tenant-resolution` TODO since T0519 (140 min) — unmoved through the T0623 audit cycle.
- Backend cold test `claim_4_live_install_invoke` 1m53s PASS (2026-05-25).

**Citation**: `curl -s 'http://127.0.0.1:7400/api/v1/skills?q=competitor'` → `{"skills":[],"total_count":0}`.

**Demo-recording implication**: Scene 4 ("search competitor pricing scrape → 3 results") **STILL UNRECORDABLE**. Single highest-leverage unlock.

### Claim 5 — Any-PC execution ❓ NOT-OBSERVABLE-FROM-SPA (flat vs T0623)

**Evidence**:
- `/workflows/templates` SSR renders 50+ template cards (from `web/lib/workflow-templates.ts`).
- `/api/v1/workflows?vertical=sales` returns 0 workflows.
- Backend round-51 confirms claim-5 substrate 47/47 cold PASS; macOS perms granted 2026-05-26.
- Cold-rerun brief `ops-claim-5-cold-rerun-mcp-and-cu-post-macos-perms` authored at `4585055d3` — TODO.
- T0519 brief `fix-workflow-execute-csrf-token-wiring-verify-before-demo` (P1, TODO) suggests Run-now POST may 403 csrf_missing.

**Citation**: `curl -s 'http://127.0.0.1:7400/api/v1/workflows?vertical=sales'` → `{"total_count":0,"workflows":[]}`.

**Demo-recording implication**: Scene 5 has a viable surface but `customer_research_followup` exists only as a test fixture (not in `/workflows/templates`); runbook may need re-naming (e.g. `competitor_intel_digest`). End-to-end gated on cold-rerun + workflow-execute CSRF verify.

### Claim 6 — Multi-channel + multi-tenant 🟡 LIKELY-LIVE-but-degraded (flat vs T0623, degraded by F2)

**Evidence**:
- `/settings/connectors` + `/settings/notify-channels` + `/settings/providers` all render (file presence + T0519 verification).
- Audit feed serves rows BUT 10 of 10 are SPA self-traffic (F2, degraded from T0623 5/5). Cross-tenant negative test backend cold PASS 52.42s (2026-05-25), but the SPA-side rendering surface has nothing meaningful to display.
- `/settings/security` exists with metadata + content; no current evidence of RBAC enforcement-not-just-copy (carried-over concern from the 2026-05-20→21 25-audit sweep memory).

**Citation**: `curl -s http://127.0.0.1:7400/api/v1/audit?limit=10` → 10/10 worker=unknown verb=http.request.

**Demo-recording implication**: Scene 6 audit drill-in is theatre-empty (clicking any row shows "audit.detail.viewed by unknown"). Resolving F2 is the prerequisite to making claim 6 visible in the demo MP4.

---

## Resolved since prior audit (T0623)

- **#2416 (docs sub-routes stub)** — T0623 F2 `/docs` 6 dead 404 sub-pages CLOSED. 10+ sub-routes now carry `page.tsx` placeholders.
- **No other surface change** — 0 PRs merged in 76 min. Schema v83 unchanged. Same gateway data shape.

Persisting unfixed from T0623:
- F1 (skills+verticals seed empty) — P0, brief queued, TODO since T0519 (140 min)
- F3 (default-title leak) — no consolidated brief yet
- F4/F7 (workers route 404) — partially covered by sidebar-label brief, route alias unbriefed
- F5 (`/sla` no metadata) — no brief yet, 3 audits unresolved
- F9/F12 (`/billing/subscribe` no H1 + no metadata) — P1 brief queued, TODO
- F10 (`/team` 2 H1s) — no brief yet
- F11 (settings sub-page metadata missing) — P1 brief queued (T0623), TODO 76m
- F12-F15 (cohorts/workflows-templates polish, settings shell H1) — partial briefs
- F16 (`/genesis` H1) — no brief yet
- F17/F18 (workflows empty + retro loading) — partial; workflows CSRF brief queued

---

## Orientation score

**59 / 100.** (vs T0623 = 61, T0519 = 63, T0415 = 62)

Reasoning: the score dropped 2 more points because (a) **zero forward motion in 76 minutes** — no PRs merged, no P0 briefs claimed; (b) the audit-feed self-traffic finding degraded from 5/5 (T0623) to 10/10 (T0739) — every new SPA visit adds another self-traffic row, so the longer this goes unfixed the more polluted the audit table becomes; (c) the worker-detail Run-now P0 brief is CLAIMED-zombie — sitting in a half-state that blocks both the original worker and any replacement.

The structural footings remain intact (5 onboarding steps SSR clean, Genesis studio inputs in place, audit endpoint serves rows just the wrong rows, settings sub-tree exists, F1+C2+D3 locked designs are in JSX, theme provider correctly wired). The bottleneck remains the data layer + a brief-queue stall.

**One-bullet recommendation for the operator**: investigate why **zero P0 briefs were claimed in 76 minutes** despite 15 open PRs + active workers per conductor heartbeat. Same recommendation as T0623 — the `fix-gateway-skills-and-verticals-boot-seed-tenant-resolution` brief is the single highest-leverage unlock and it has now been TODO 140+ minutes. After that, release the CLAIMED-zombie `fix-worker-detail-run-now-action-and-endpoint` back to TODO so a fresh worker can pick up the salvageable WIP in `/tmp`.
