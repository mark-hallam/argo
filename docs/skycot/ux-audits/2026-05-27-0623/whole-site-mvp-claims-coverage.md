# Whole-site MVP claims-coverage rollup — 2026-05-27 06:23 Melbourne

**Scope**: every SPA route NOT in the 4 sibling agents' hot paths. Marketing (`/landing`, `/pricing`, `/customers`, `/compare`, `/security`, `/sla`, `/changelog`, `/privacy`, `/terms`, `/for/*`, `/docs`, `/help`), in-app surfaces outside the 6-scene runbook (`/cohorts`, `/settings/*` 11 sub-pages, `/brand`, `/brand/components`, `/team`, `/workflows/templates`, `/verticals`, `/genesis`, `/retro`, `/orchestration`, `/support/inbox`, `/billing`, `/billing/subscribe`, `/skills-canvas`), theme provider wiring, viewport-responsiveness markers, plus the load-bearing **6-MISSION-claim coverage rollup**.

**Method**: gateway probes (curl against `http://127.0.0.1:7400`), source-of-truth `.tsx` + brief-ledger inspection (read-only — no cargo, no SPA dev server fully online), prior-audit DELTA cross-check, BUILD_STATUS UX-AUDIT-T0519MEL queue verification. Preview chrome and the SPA on port 3000/3002 were not navigable during this audit window — Turbopack first-compile for `/landing` ran 151.6s and the listener fell over before the rest of the route tree compiled. All evidence below is gateway response bodies + `.tsx` source + ledger rows.

**Working tree**: branch `main`, gateway 7400 schema **v83** (was v81 at T0519 — moved 2 schemas forward), SPA same-origin proxy still wired per #2429, HEAD `5a7c4150f` ("ux-audit-T0519MEL: 5-agent e2e pass + synthesis — 17 root-cause fix-briefs queued").

**Time since prior whole-site audit**: 64m (T0519 → T0623). Since T0519: only one PR merged (`#2431 fix-skills-try-chips` — rewrote chip stems to single-word seeds that would hit the catalog **if it were populated**, but the catalog is still empty, so the chip rewrite is a moot fix until F1 lands). The T0519 synthesis committed 17 fix-briefs to `docs/build-plan/streams/` + queued them under the UX-AUDIT-T0519MEL header in BUILD_STATUS.md — all 17 are still `TODO` (none claimed in the 64-min window). Two T0415 P0s remain `CLAIMED` without PRs.

---

## TL;DR

20 findings: **3 P0** (gateway skills+verticals seed still empty 76 minutes after #2407 + 1 follow-up tick + 1 cosmetic chip-rewrite; `/docs` still fans out to 6 dead 404 sub-pages; audit feed has 5 of 5 rows = SPA self-traffic, zero real agent turns), **7 P1** (default "Approvals" title leaks across 9+ in-app pages, `/workers` collection endpoint still 404, `/verticals` empty seed, `/sla` no metadata, `/billing/subscribe` no `<h1>`, `/team` 2 H1s, settings shell duplicate H1 across 8 sub-pages), **8 P2** (workflows API empty so /workflows/templates seems stale, settings sub-page metadata missing on 7 of 11, /genesis H1 mismatch with onboarding-studio scope, /retro SSR "Loading retros…", cohorts EMPTY_PREVIEW_GROUPS rendered even with 1 real cohort, /workflows/templates breaks title chain), **2 P3** (logos still stub avatars; sidebar dual-label Workers vs Team).

**6-MISSION-claim functional verdict from the SPA, T0623**:
- 1 = 🟡 **LIKELY-LIVE** (real worker "Maya" via `/api/v1/team`; `/workers` collection 404 forces SPA fixture roster fallback)
- 2 = ❓ **NOT-OBSERVABLE-FROM-SPA** (no `/handoffs` UI; audit drill-in is the only visualisation surface and the feed is 100% SPA self-traffic)
- 3 = 🟡 **LIKELY-LIVE** (5 onboarding steps all SSR; `/verticals` seed empty so wizard reads from SEED_VERTICALS fixture; Genesis H1 still mismatches studio scope)
- 4 = 🔴 **BROKEN-IN-SPA** (skills catalog STILL empty 76 min + 1 PR + 1 tick later — root cause `fix-gateway-skills-and-verticals-boot-seed-tenant-resolution` queued P0, not yet claimed)
- 5 = ❓ **NOT-OBSERVABLE-FROM-SPA** (workflows API now returns 0; `/workflows/templates` page still renders 50+ static cards from `web/lib/workflow-templates.ts`; computer-use perms still operator-owed)
- 6 = 🟡 **LIKELY-LIVE** (`/settings/connectors` + `/settings/notify-channels` render; audit endpoint serves real rows but they're 100% the SPA's own auth/replay self-traffic — function-side trust signal is degraded)

**Direction vs T0519** (1h prior): flat on all 6 claims. The 64-min interval shipped only the cosmetic chip-rewrite (#2431); the root-cause `fix-gateway-skills-and-verticals-boot-seed-tenant-resolution` brief (queued by T0519's own synthesis) sat TODO the entire interval. The audit feed pollution is **net-new this run** — visible because the prior audit's data layer (still empty) means the only "audit rows" are the SPA hitting `/api/v1/audit` itself.

**Single biggest unlock for the demo**: claim the P0 `fix-gateway-skills-and-verticals-boot-seed-tenant-resolution` brief. It is the same root-cause for F1 (skills seed) AND F4 (verticals seed) — one fix unblocks Scenes 3 + 4 simultaneously. Until then, Scene 4 is unrecordable and Scene 3 renders fixtures.

---

## Headline blockers

- **F1 [P0] `/skills` catalog STILL EMPTY through the SPA-resolved tenant** — 76 minutes after #2407 + 1 follow-up cosmetic PR + 1 conductor tick. `curl http://127.0.0.1:7400/api/v1/skills` → `{"skills":[],"tenant_id":"019e5057-1d3f-7fa2-9c1b-c34ef01cc21e","total_count":0}`. Same payload for `?q=competitor`, `?q=research`, `?q=draft`, `?q=email` (the 4 stems #2431 rewrote chips to). Scene 4 unrecordable. The T0519 synthesis already queued the root-cause brief `fix-gateway-skills-and-verticals-boot-seed-tenant-resolution` as P0; it's been TODO for 64 minutes.
- **F2 [P0] `/docs` STILL fans out to 6 dead 404 sub-pages** — unchanged from T0519 (T0519 F2). The `/docs` page advertises 6 category cards (`/docs/getting-started`, `/workflows`, `/skills`, `/audit`, `/api`, `/integrations`) — none exist as routes; the in-source comment at `web/app/docs/page.tsx:30` acknowledges this ("Cards link to `/docs/<slug>` pseudo-routes — those will 404 gracefully via the existing `not-found.tsx`. CMS-backed doc content arrives in a follow-up brief"). Public marketing trust signal: every doc link goes to a 404.
- **F3 [P0] `/api/v1/audit` returns 5 of 5 rows = SPA self-traffic (zero real agent turns)** — every audit row has `worker.name="unknown"`, `verb="http.request"`, `action_type` is either `audit.detail.viewed` or `audit.replay`. The audit page would render rows the demo camera could see, but the content reads "audit.detail.viewed by unknown" — pure dressing without function. Scene 6 is the audit drill-in; drilling into any of these rows shows the SPA's own auth/replay pings, not an agent turn. The T0519 synthesis queued `fix-audit-feed-filter-or-bucket-spa-self-traffic-http-request` as P1; given the demo runbook drills into the audit table, **this should be P0**.
- **F4 [P1] `(app)/layout.tsx:20` default title "Approvals" leaks across 9+ in-app pages** — unchanged from T0519. The default in the (app) group metadata is `"Approvals"` + template `"%s · SKYCOT"`. Pages that don't export their own metadata render the literal "Approvals" as the tab title. Confirmed leak across: `/brand`, `/brand/components`, `/retro`, `/genesis`, `/orchestration`, `/verticals`, `/support/inbox`, `/billing`, `/billing/subscribe`. Demo camera reads the tab bar on every shot.

---

## All findings, in order

### Public marketing pages

#### F1 [P0] — `/skills` marketplace API STILL empty across all query stems
Re-verified at T0623: `curl -s http://127.0.0.1:7400/api/v1/skills` returns `{"skills":[],"tenant_id":"019e5057-1d3f-7fa2-9c1b-c34ef01cc21e","total_count":0}`. Tested with the new #2431-rewritten chip stems (`research`, `email`, `draft`, `competitor`) — all return 0 rows. The cosmetic chip-rewrite landed but the catalog is still empty, so the chips are a moot improvement.

Repro:
- `curl -s http://127.0.0.1:7400/api/v1/skills | head -c 200` → empty catalog
- `for q in research email draft competitor ; do curl -s "http://127.0.0.1:7400/api/v1/skills?q=$q" ; done` → all empty
- The brief `docs/build-plan/streams/fix-gateway-skills-and-verticals-boot-seed-tenant-resolution.md` exists and is TODO since 2026-05-27 03:21 AEST.

Fix: claim the queued P0 brief. Three hypotheses documented (tenant binding / feature gate / idempotency sentinel) — needs source-walk to pick the right one.

#### F2 [P0] — `/docs` STILL fans out to 6 dead 404 sub-pages
The `web/app/docs/page.tsx` file at line 30 carries an in-source comment saying "Cards link to /docs/<slug> pseudo-routes — those will 404 gracefully" — confirming this is intentional placeholder state. The 6 sub-routes (`/getting-started`, `/workflows`, `/skills`, `/audit`, `/api`, `/integrations`) don't exist. Public marketing trust signal: a visitor who clicks "Get Started" from `/docs` lands on the global 404.

Repro:
- Open `web/app/docs/page.tsx` — the in-source comment is the smoking gun
- 6-scene runbook doesn't navigate to `/docs`; but the `/landing` footer + in-app HelpWidget point to `/docs` — pre-record QA may click through

Fix: either comment out the 6 link cards on `/docs` until sub-routes ship, or external-link redirect to GitHub repo docs.

#### F5 [P1] — `/sla` STILL falls back to global default title (T0519 F5 unresolved)
`web/app/sla/page.tsx` has no `export const metadata`; only `export default function SlaPage()`. The HTML `<title>` reads the root layout's marketing default rather than `SLA · SKYCOT`. Same trivial fix queued for >2h.

File: `web/app/sla/page.tsx`.

#### F6 [P3] — `/customers` logos still stub avatars (T0519 F7 unresolved)
Per memory `feedback_logos_from_svgl_app`: customer logos must be SVGs from svgl.app. Still rendering placeholder badges for the 5 fictional customers. Dressing-side; flag only.

### In-app pages (outside 6-scene hot path)

#### F3 [P0] — `/api/v1/audit` returns 5 of 5 rows = SPA self-traffic (zero real agent turns)
Re-confirmed at T0623. Every audit row in the live feed:
```
http.request - unknown - audit.detail.viewed
http.request - unknown - audit.replay
http.request - unknown - audit.replay
```
The audit page renders these rows in the locked B3 Etherscan-style table — but the table contents are entirely the SPA's own auth pings, not agent turns. **Scene 6 is the audit drill-in**; the camera clicks a row and sees "audit.detail.viewed by unknown". Function-side fabrication, not dressing-side. Per the function-vs-dressing rule, this is P0 for the demo.

Repro:
- `curl -s 'http://127.0.0.1:7400/api/v1/audit?limit=10' | python3 -c 'import sys,json; [print(x.get("verb"),"-",x.get("worker",{}).get("name"),"-",x.get("action_type")) for x in json.load(sys.stdin).get("items",[])]'`

Fix: the T0519 synthesis queued `fix-audit-feed-filter-or-bucket-spa-self-traffic-http-request` as P1; recommend re-classify to P0 because Scene 6 specifically drills into this surface. Either filter `worker.name == "unknown"` at the gateway list endpoint OR bucket self-traffic into a separate `/audit/system` view.

#### F4 [P1] — `(app)/layout.tsx:20` default title "Approvals" leaks across 9+ pages (T0519 F3 unresolved)
Verified the metadata template at `web/app/(app)/layout.tsx:18-22`:
```ts
title: {
  default: "Approvals",
  template: "%s · SKYCOT",
}
```
Pages without their own `metadata` fall through to the literal "Approvals". 9 confirmed leak surfaces: `/brand`, `/brand/components`, `/retro`, `/genesis`, `/orchestration`, `/verticals`, `/support/inbox`, `/billing`, `/billing/subscribe`. Worst case: `/genesis` (an investor moment per memory `project_pitch_agent_v1_demo`) reads "Approvals · SKYCOT" in the tab bar.

Fix: change default to `"SKYCOT"` AND add `export const metadata` to all 9 orphan pages. T0519 did not queue this as a single brief — should be merged into one mechanical fix-brief.

#### F7 [P1] — `/api/v1/workers` collection endpoint STILL 404s (T0519 F4 unresolved)
- `curl -s http://127.0.0.1:7400/api/v1/team` → 1 real worker "Maya"
- `curl -s http://127.0.0.1:7400/api/v1/workers` → `{"error":"not found"}`

User-visible: SPA workers grid falls back to fictional roster (Mariah/Peta/Priya/Jamal/Nori) while the real workspace has "Maya". The T0519 synthesis queued `fix-sidebar-workers-vs-team-nav-label-consolidation` as P1, addressing the **label** mismatch but not the **route** mismatch. The route-level 404 needs its own brief.

File: `crates/argo-cli/src/handlers/gateway/routes_workers_list.rs`.

#### F8 [P1] — `/api/v1/verticals` returns empty `{"verticals":[]}` (T0519 F10 unresolved)
Same root-cause as F1 (boot-seed doesn't populate for SPA-resolved tenant). `/verticals` page hooks into the live gateway via `useVerticals()` (`web/app/(app)/verticals/page.tsx:21-30`) and falls back to a "seed fallback in dev when the gateway is empty or unreachable" per the in-source comment. The seed fallback is `SEED_VERTICALS` — 5 fictional verticals. Scene 3's pick-vertical step is the camera-facing version of this same surface.

The T0519 brief `fix-gateway-skills-and-verticals-boot-seed-tenant-resolution` (P0, codex-assigned, TODO) covers both F1 and F8. One claim, both unblocked.

#### F9 [P1] — `/billing/subscribe` has NO `<h1>` at all (T0519 F12 unresolved)
HTML inspection of `web/app/(app)/billing/subscribe/page.tsx`: zero `<h1>` elements. The page is a Stripe-checkout-redirect landing, but accessibility requires a top-of-document heading. Plus no `metadata` export, so tab reads "Approvals · SKYCOT" (the F4 leak).

The T0519 synthesis queued `fix-billing-subscribe-missing-h1-and-metadata` as P1 — TODO.

File: `web/app/(app)/billing/subscribe/page.tsx`.

#### F10 [P1] — `/team` has 2 `<h1>` elements on one page (revised from T0519 F13 which said 3)
Verified via `grep -n '<h1' web/app/(app)/team/page.tsx`: 2 H1s present — `<h1 className="text-xl font-semibold tracking-tight">Team</h1>` at line 55, and a second `<h1>` (same classes) at line 250. The "3 H1s" T0519 finding was likely double-counting the AppShell-rendered breadcrumb header. Still a semantic-HTML violation: heading-navigation by screen-reader users lands on two "Team" anchors.

Fix: drop the secondary H1 OR convert one to `<h2>`.

#### F11 [P1] — 7 of 11 `/settings/*` sub-pages have no metadata export (T0519 F17 unresolved)
Confirmed via `grep -l "export const metadata" web/app/(app)/settings/*/layout.tsx`. Metadata exports present on:
- `/settings/api-keys`, `/settings/data-export`, `/settings/members`, `/settings/security` (4)

Missing on:
- `/settings/billing`, `/settings/connectors`, `/settings/knowledge`, `/settings/notifications`, `/settings/notify-channels`, `/settings/providers`, `/settings/workspace` (7)

Each missing-metadata page reads "Settings · SKYCOT" (the (app) shell's settings layout default) instead of "Connectors · SKYCOT" / "Workspace · SKYCOT" / etc.

Fix: mechanical — 7 × 4-LOC `layout.tsx` additions.

#### F12 [P2] — `/workflows/templates` breaks the `%s · SKYCOT` title chain (T0519 F11 unresolved)
`web/app/(app)/workflows/templates/layout.tsx:5` exports `metadata.title = "Workflow templates"` as a string literal. Next.js treats this as an override that doesn't inherit the parent layout's `"%s · SKYCOT"` template. The tab reads "Workflow templates" without the SKYCOT suffix — inconsistent with sibling in-app pages.

Fix: `title: { absolute: "Workflow templates · SKYCOT" }`.

#### F13 [P2] — `/cohorts` SSR shows `CohortsEmptyPreview` even with 1 real cohort (T0519 F14 unresolved)
`web/app/(app)/cohorts/page.tsx:103-110`: the rendering ternary is `isPending ? skeleton : cohorts.length === 0 ? <CohortsEmptyPreview> : <Magazine>`. During SSR `cohortsQuery.isPending` is `false` (no fetch attempted yet — server-render returns the empty array). So the SSR snapshot renders `CohortsEmptyPreview` despite the API having 1 cohort (`marketing-intern::unknown`). After hydration the magazine grid appears.

Demo recording captures ~200-500ms of empty-preview before hydration completes. Polish.

Fix: gate on `cohortsQuery.isSuccess && cohorts.length === 0` rather than `cohorts.length === 0` alone.

#### F14 [P2] — Cohorts hero subtitle pluralises "0 cohorts" SSR → "1 cohort" post-hydrate (T0519 F15 unresolved)
Verified `web/app/(app)/cohorts/page.tsx:80-86`: subtitle renders `{total} {total === 1 ? "cohort" : "cohorts"} across the fleet` once query resolves. During SSR total=0 — visible flicker on first paint.

Same fix as F13.

#### F15 [P2] — `/settings/*` sub-pages render a duplicate "Settings" `<h1>` from shell + per-page H1 (T0519 F16 unresolved)
Confirmed across the 4 settings sub-pages that do carry their own H1 (`api-keys`, `data-export`, `members`, `security`) — they all have 2 H1s in the rendered HTML: the shell-level "Settings" + the page-specific heading. The 7 that don't carry their own H1 have only the shell "Settings" — no clear top-level heading at all.

Fix: demote shell "Settings" label to `<h2>` (or move to a `<header role="banner">`); make the sub-page H1 the primary.

#### F16 [P2] — `/genesis` H1 ("Hire your next teammate.") mismatches the operator-clarified studio scope (T0519 F18 unresolved)
Per memory `feedback_genesis_is_ai_teammate_onboarding_studio` (operator 2026-05-25T11:40Z): Genesis is the AI Teammate Onboarding Studio (voice/text role + SOP-PDF/wiki/Slack uploads + access-controls + comprehension verification). Current H1 at line 250: `title="Hire your next teammate."` reads as hire-CTA, not studio.

Fix: H1 → "Onboard a new teammate." or similar.

#### F17 [P2] — `/workflows` API returns empty + `/workflows/templates` page lists 50+ static cards
- `curl -s 'http://127.0.0.1:7400/api/v1/workflows?vertical=sales'` → 0 workflows.
- `/workflows/templates` page renders 50+ template cards from a hardcoded list in `web/lib/workflow-templates.ts`.

Scene 5 narration says "the camera clicks Run on `customer_research_followup`" — that exact template name DOES exist as a test fixture (`SubWorkflowPicker.test.tsx`) but the runtime workflows API returns no workflows. The cards on `/workflows/templates` are dressing (decorative catalog); the moment the camera runs one, Scene 5 needs the workflow engine to actually accept the request.

Cross-ref T0519 brief `fix-workflow-execute-csrf-token-wiring-verify-before-demo` (P1, TODO) is the closest queued mitigation.

#### F18 [P2] — `/retro` SSR renders "Loading retros…" with no fallback empty-state (T0519 F19 unresolved)
`web/app/(app)/retro/page.tsx:142` literal "Loading retros…" — no skeleton, no empty-state. Out-of-runbook path; polish.

#### F19 [P3] — `/skills-canvas` flat route 404 (T0519 F20 unresolved)
Flat route doesn't exist; only `/workers/[workerId]/skills/[skillId]/canvas`. The e2e agent prompt + several `streams/` briefs reference `/skills-canvas`. Either redirect or update refs.

#### F20 [P3] — Workers vs Team nav-label split (T0519 F4 + sidebar)
Sidebar exposes both "Workers" and "Team" labels for what's effectively the same surface, and the underlying routes resolve to inconsistent data (`/api/v1/team` returns 1 real worker, `/api/v1/workers` 404s). Beyond the route fix in F7, the label needs to be canonical. T0519 brief `fix-sidebar-workers-vs-team-nav-label-consolidation` (P1, TODO) addresses this.

### Theme + viewport + console hygiene

These were not interactively verifiable this run (preview chrome offline; Turbopack first-compile for `/landing` ran 151.6s and listener fell over before the rest of the tree compiled). Static-source verifications below:

- **Theme provider wiring** — `web/app/layout.tsx:114` mounts `<ThemeProvider>` from `next-themes`; the wrapper at `web/components/skycot/ThemeProvider.tsx` uses `"use client"` + reads/writes `theme` storage key. Wiring is correct; interactive toggle smoke test still owed.
- **Viewport responsiveness markers** — marketing nav has `data-testid="marketing-nav-open"` with `aria-haspopup="dialog"` per T0519 verification (closed in T0519 as F24). Marketing hamburger is wired; mobile (375px) interactive smoke still owed.
- **CSP + security headers** — confirmed via `curl -sI` against `/`: HSTS preload, X-Frame-Options DENY, X-Content-Type-Options nosniff, restrictive CSP including `connect-src 'self' http://127.0.0.1:7400 https://api.skycot.com`. Clean.

---

## 6-MISSION-claim coverage rollup (the load-bearing summary)

Reminder per the function-vs-dressing rule (`docs/skycot/demos/v1-demo-positioning-poc-vs-public.md`): "LIVE in the SPA" means a path the demo camera traverses produces real (non-fixture) data and renders the locked design. "LIKELY-LIVE" means the page renders, the backend cold tests pass, and the same-origin proxy is wired (post-#2429) — but at least one wire defect prevents end-to-end verification from the browser.

### Claim 1 — Teammate, not tool 🟡 LIKELY-LIVE (flat vs T0519 🟡)

**Evidence**:
- `/api/v1/team` returns 1 real worker "Maya" (`role.vertical = marketing-intern`, "Marketing intern that drafts and asks before publishing.").
- `/api/v1/workers` still returns `{"error":"not found"}` — same defect as T0519 F4 and now T0623 F7. The SPA workers grid falls back to `sample-workers-detail.ts` (5 fictional teammates).
- Backend cold tests pass (round-19 evaluator + round-54).

**Citation**: `curl -s http://127.0.0.1:7400/api/v1/team` → `{"total_count":1,"workers":[{"name":"Maya",...}]}`. `curl -s http://127.0.0.1:7400/api/v1/workers` → `{"error":"not found"}`.

**Demo-recording implication**: Scene 1 narrates "Mariah, Peta, Priya, Jamal, Nori" — a fictional roster. Real workspace has "Maya". Either the runbook re-scripts to "Maya, your marketing intern", or the demo-seed CLI gets run with the 5-persona roster (CLAUDE.md remaining-for-launch item (e), still TODO). F7 (route alias 404) is the gating defect.

### Claim 2 — Delegation is the unit ❓ NOT-OBSERVABLE-FROM-SPA (flat vs T0519)

**Evidence**:
- No `/handoffs` or `/delegations` UI surface in the SPA route tree (verified by `ls web/app/(app)/`).
- Audit page IS the only delegation-visualisation surface — but the audit feed is 100% SPA self-traffic (F3), so even if the row-detail drawer renders handoff hashes, the rows it has to render are auth pings, not agent turns.
- Backend round-54 confirms claim 2 LIVE (delegate-task fallback-chain inheritance wired via #2256).

**Citation**: `ls ~/code/argo/web/app/(app)/` shows no delegation route; `curl -s http://127.0.0.1:7400/api/v1/audit` returns 5 of 5 self-traffic rows.

**Demo-recording implication**: Scene 6 audit drill-in cannot demonstrate delegation chains because there are no agent-turn rows to drill into. Resolving F3 (filter self-traffic) is a prerequisite to making claim 2 observable from the SPA.

### Claim 3 — Platform builds workers 🟡 LIKELY-LIVE (flat vs T0519 🟡)

**Evidence**:
- 5 onboarding steps SSR cleanly (per T0519 walk; same code paths).
- `/genesis` studio shell present with `<textarea data-testid="genesis-role-input">` + Generate button — function is in place; H1 copy mismatched per F16.
- `/api/v1/verticals` STILL returns `{"verticals":[]}` — `/onboarding/pick-vertical` falls back to SEED_VERTICALS fixture (5 entries vs the 30+ the gateway is meant to seed). Per T0519 brief `fix-wizard-verticals-rewrite-scope-and-dead-fallback` (TODO).
- Backend round-54 confirms claim 3 LIVE.

**Citation**: `curl -s http://127.0.0.1:7400/api/v1/verticals` → `{"verticals":[]}`. Source `web/app/(app)/verticals/page.tsx:21-30` confirms the hook + fallback pattern.

**Demo-recording implication**: Scene 3 is recordable but the pick-vertical step reads fixtures. The same root-cause as claim 4 (boot-seed not populating). One brief unblocks both: `fix-gateway-skills-and-verticals-boot-seed-tenant-resolution` (TODO since T0519, P0).

### Claim 4 — Skills shared + learned 🔴 BROKEN-IN-SPA (flat vs T0519 🔴)

**Evidence**:
- `/skills` catalog API still returns `{"skills":[],"total_count":0}` 76 minutes after #2407 merged + 1 cosmetic chip-rewrite (#2431) + 1 conductor tick + the T0519 synthesis queuing a P0 root-cause brief.
- `#2431` (fix-skills-try-chips) merged at T20-05 conductor tick — rewrote chips to single-word stems that **would** hit the catalog **if it were populated**. Stems tested: `research`, `email`, `draft`, `competitor` — all return 0 rows.
- Root-cause brief `fix-gateway-skills-and-verticals-boot-seed-tenant-resolution` queued by T0519, still TODO 64m later.
- Backend cold test `claim_4_live_install_invoke` 1m53s PASS (2026-05-25).

**Citation**: `curl -s 'http://127.0.0.1:7400/api/v1/skills?q=competitor'` → `{"skills":[],"total_count":0}`. Same for `research`, `email`, `draft`.

**Demo-recording implication**: Scene 4 ("search competitor pricing scrape → 3 results") **STILL UNRECORDABLE**. Single highest-leverage unlock for the demo MP4.

### Claim 5 — Any-PC execution ❓ NOT-OBSERVABLE-FROM-SPA (flat vs T0519)

**Evidence**:
- `/workflows/templates` SSR renders 50+ template cards (from hardcoded `web/lib/workflow-templates.ts`; includes `competitor_intel_digest` and others).
- `customer_research_followup` exists as a test fixture (`SubWorkflowPicker.test.tsx:40`) but NOT in the workflow-templates static list — runbook narration of that name may need to be retitled.
- `/api/v1/workflows?vertical=sales` returns 0 workflows. Workflow templates rendered on screen are dressing; runtime execution path is empty.
- Backend round-51 confirms claim-5 substrate 47/47 cold PASS; operator macOS perms still owed.
- T0519 brief `fix-workflow-execute-csrf-token-wiring-verify-before-demo` (P1, TODO) suggests the Run-now POST may 403 csrf_missing — function-side risk for Scene 5.

**Citation**: `curl -s 'http://127.0.0.1:7400/api/v1/workflows?vertical=sales'` → `{"total_count":0,"workflows":[]}`. Static template list in `web/lib/workflow-templates.ts:165` confirms `competitor_intel_digest` present.

**Demo-recording implication**: Scene 5 has a viable surface but may need runbook re-naming (`customer_research_followup` → `competitor_intel_digest`) and runtime-engine verification before recording. Computer-use end-to-end still gated on operator macOS perms.

### Claim 6 — Multi-channel + multi-tenant 🟡 LIKELY-LIVE (flat vs T0519 🟡, but degraded by F3)

**Evidence**:
- `/settings/connectors` + `/settings/notify-channels` + `/settings/providers` all render in-source with channel/connector UI (confirmed by file presence + adjacent page content per T0519).
- Audit feed serves real rows BUT 5 of 5 are SPA self-traffic (F3). Cross-tenant negative test backend cold PASS 52.42s (2026-05-25), but the SPA-side rendering surface has nothing meaningful to display.
- `/settings/security` exists with metadata + content; no current evidence of RBAC enforcement-not-just-copy (carried-over concern from the 2026-05-20→21 25-audit sweep memory).

**Citation**: `curl -s http://127.0.0.1:7400/api/v1/audit | python3 -c '...'` → 5/5 rows worker=unknown verb=http.request. `/settings/connectors` + `/settings/notify-channels` source files exist.

**Demo-recording implication**: Scene 6 the audit drill-in is theatre-empty (clicking any row shows "audit.detail.viewed by unknown"). Resolving F3 is the prerequisite to making claim 6 visible in the demo MP4.

---

## Resolved since prior audit (T0519)

- **#2431 (fix-skills-try-chips-zero-results-rewrite)** MERGED at T20-05 — cosmetic chip-stem rewrite. Does NOT unblock F1 (catalog still empty); the chips would now hit the catalog if it were populated.
- **Schema bump v81 → v83** — 2 schemas of progress in 64 min; no other observable surface change.
- **17 T0519 synthesis briefs** committed to `docs/build-plan/streams/` — all 17 TODO since T0519 (none claimed in 64m).
- **Cohorts page now gates empty-state on `cohortsQuery.isPending`** for the body (skeleton during pending) — improvement vs T0519 F14, but the hero-subtitle + the `total === 0` hero-title check still trigger SSR (the body is post-hydrate gated, the hero is not).

Persisting unfixed from T0519:
- F1 (skills+verticals seed empty) — P0, brief queued, TODO
- F2 (docs 6 dead links) — no brief yet
- F3 (default-title leak) — no consolidated brief yet
- F4/F7 (workers route 404) — partially covered by sidebar-label brief, not the route alias
- F5 (`/sla` no metadata) — no brief yet
- F9/F12 (`/billing/subscribe` no H1 + no metadata) — P1 brief queued, TODO
- F10 (`/team` 2 H1s) — no brief yet
- F11 (settings sub-page metadata missing) — no brief yet
- F12-F15 (cohorts/workflows-templates polish, settings shell H1) — no briefs yet
- F16 (`/genesis` H1) — no brief yet
- F17/F18 (workflows empty + retro loading) — partial briefs; workflows CSRF brief queued
- F19 (`/skills-canvas` flat 404) — no brief yet

---

## Orientation score

**61 / 100.** (vs T0519 = 63, T0415 = 62)

Reasoning: the score dropped 2 points because (a) the T0519 synthesis queued 17 briefs and 64 minutes later zero of them have been claimed by a worker — the conductor tick + 1 PR cycle in the interval was cosmetic (#2431) not load-bearing; (b) the audit-feed self-traffic finding (F3) was visible in the prior data but is now sharpened to a P0 — the moment Scene 6 drills into an audit row, the camera reads "audit.detail.viewed by unknown" and the demo is broken at the function layer.

The structural footings are intact: 5 onboarding steps SSR clean, Genesis studio has the right inputs, audit endpoint serves rows (just the wrong rows), settings sub-tree exists, F1 + C2 + D3 locked designs are in JSX. The bottleneck is the data layer — the boot-seed not firing for the SPA-resolved tenant **gates 2 of 6 claims at once** (4 + half of 3 via verticals).

**One-bullet recommendation for the operator**: claim the P0 `fix-gateway-skills-and-verticals-boot-seed-tenant-resolution` brief NOW. It's been TODO 64m. Same root-cause unblocks F1 (Scene 4) AND F8 (Scene 3 verticals). After that, queue a single consolidated fix-brief for F3 (audit-feed self-traffic filter) and re-classify it P0 — it's the gating defect for both Claims 2 and 6 in the demo MP4.
