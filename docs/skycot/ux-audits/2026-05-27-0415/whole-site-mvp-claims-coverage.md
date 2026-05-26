# Whole-site MVP claims-coverage rollup — 2026-05-27 04:15

**Scope**: every SPA page NOT in the 6-scene runbook's hot path (marketing, `/cohorts`, `/settings/*`, `/brand/*`, `/team`, `/workflows/templates`, `/skills-canvas`, plus theme + viewport behaviour).

**Method**: HTTP probe of all 60+ SPA routes (curl), DOM-attribute grep, gateway endpoint probe with and without auth, CORS preflight check, one live preview snapshot of `/landing`. The Claude Preview chrome wedged after `window.location.assign('/')` and never recovered for this session, so most evidence is server-side HTML + gateway-response inspection rather than interactive screenshots. I did not restart the preview server so as not to disrupt sibling agents.

**Working tree**: branch `claude-fix-skills-marketplace-perpetual-loading`, gateway 7400 (schema v83), SPA 3000.

---

## TL;DR

23 findings: **1 P0** (silent demo-blocker — the entire SPA renders against fixture data because every browser→gateway call is CORS-blocked), **6 P1** (skills marketplace empty, dupe-`<title>` on five `(app)/*` pages, layout default leaks "Approvals" into tab titles, /cohorts empty-state hides the locked D3 design, /workflows omits an Audiowide hero on a brand-loud page, /skills-canvas is a 404 even though briefs reference it), **9 P2** (metadata gaps, dup-h1 on settings shell, SLA page lacks title tag, marketing pages render fine but with placeholder testimonial photography), **7 P3** nits.

**6-MISSION-claim functional verdict from the SPA**: 1 = 🟡, 2 = ❓, 3 = 🟡, 4 = 🔴 (in SPA scope; backend cold-tests pass), 5 = ❓, 6 = 🟡. Detail in §6 below.

**Single biggest fix to unlock more of the demo from the browser**: add `Access-Control-Allow-Origin: http://127.0.0.1:3000` (or `*` for dev) + `Allow-Methods` + `Allow-Headers` to gateway responses, OR route browser→gateway through a Next.js `/api/*` rewrite proxy (which the codebase has elsewhere — see `web/next.config.js`). Without this, the demo MP4 will show the "Demo mode — sample data" yellow banner across every in-app scene, even though the gateway is up and the cold tests pass.

---

## Headline blockers

- **F1 [P0] CORS missing on gateway → SPA permanently in demo-fixture mode.** Every `/api/v1/*` fetch from the browser fails with `net::ERR_FAILED`. Demo banner ("Demo mode — showing sample data. Sign in to see your own workspace.") is visible at the top of every signed-in screen.
- **F2 [P1] `/skills` marketplace renders 0 skill cards.** Gateway endpoint responds 200 but with `{"skills":[],"total_count":0}` — the boot-time seed from commit `2639ed167` is not populating this tenant. Scene 4 of the runbook (search "competitor pricing scrape" → 3 results) is unrecordable.
- **F3 [P1] `/api/v1/workers` returns 404 to anonymous callers while `/api/v1/team` returns 200 from the same handler.** Surfaces with the same shape but divergent dispatch — only visible because the SPA's E4-a path uses `/workers`, which silently fails over to `web/lib/sample-workers-detail.ts`.
- **F4 [P1] `(app)/layout.tsx:20` default title is "Approvals".** Five `(app)/*` pages that don't override metadata (`/brand`, `/brand/components`, `/retro`, `/genesis`, `/orchestration`) render with the tab title "Approvals · SKYCOT" while displaying their real content.

---

## All findings, in order

### Marketing (public-facing pages)

#### F1 [P0] — SPA-to-gateway calls all CORS-blocked; demo banner shown across every signed-in screen
The Claude Preview's network panel after a single page load of `/landing` (which the layout converts to `/team` for signed-in operators) recorded **23 failed gateway requests in <2 s**:
```
GET http://127.0.0.1:7400/api/v1/meta/error-reporting [FAILED: net::ERR_FAILED]
GET http://127.0.0.1:7400/api/v1/workers              [FAILED: net::ERR_FAILED]
GET http://127.0.0.1:7400/api/v1/skills               [FAILED: net::ERR_FAILED]
GET http://127.0.0.1:7400/api/v1/skills/learned-this-month [FAILED]
GET http://127.0.0.1:7400/api/v1/approvals            [FAILED]
... (same five endpoints fanned out four more times each)
```
Direct `curl` to the same endpoints from the shell returns 200 with real payloads. The block is CORS: an `OPTIONS` preflight on `/api/v1/skills` with `Origin: http://127.0.0.1:3000` returns `204 No Content` with zero `Access-Control-Allow-*` headers. Browser-side fetch therefore rejects the response before the SPA sees it.

The SPA degrades gracefully (`web/lib/sample-workers-detail.ts`, `sample-worker-overview.ts`, etc. supply fixture rosters of Mariah/Peta/Priya/Jamal/Nori) but the demo banner gives the game away. The runbook's Scene 1 talk-track will not match a workspace banner that says "showing sample data."

Repro:
- `curl http://127.0.0.1:7400/api/v1/skills` → `200 OK {"skills":[],"total_count":0}` (server-side)
- `curl -i -X OPTIONS http://127.0.0.1:7400/api/v1/skills -H "Origin: http://127.0.0.1:3000" -H "Access-Control-Request-Method: GET"` → 204 with no `Access-Control-Allow-Origin`
- Open Chrome to `http://127.0.0.1:3000/team`, view the yellow "Demo mode" banner

Likely fix: enable a `tower-http::cors::CorsLayer` (or equivalent in the gateway's response writer at `crates/argo-cli/src/handlers/gateway/boot.rs`) with `allow_origin: http://127.0.0.1:3000` (or env-driven for prod), `allow_methods: GET, POST, PUT, DELETE, OPTIONS`, `allow_headers: content-type, authorization`. Alternative: add `/api/v1/*` to the Next.js rewrites table at `web/next.config.js` so calls go same-origin.

File: gateway response writer is at `crates/argo-cli/src/handlers/gateway/boot.rs`; CORS layer should sit alongside the security headers (`X-Content-Type-Options`, `X-Frame-Options`) the responses already carry.

#### F5 [P2] — `/sla` falls back to the global default `<title>` "SKYCOT — Hire your first AI teammate"
The page renders SLA content correctly (H1: "When you'll hear back."), but doesn't export `metadata`, so the browser tab reads as if it's the marketing landing page. Same issue for `/for/page.tsx` (the index of the `/for/<vertical>` set). The five vertical pages under `/for/<vertical>` DO override metadata and render with rich SEO titles.

File: `web/app/sla/page.tsx`, `web/app/for/page.tsx`

Fix: add `export const metadata = { title: "Service-level agreement" }` and `"Skycot for every team"` respectively.

#### F6 [P3] — `/landing` H1 line break renders "Hire a teammate. (Seriously.)" as three lines with a hard `<br>` between "tool." / "Hire a" / "(Seriously.)"
Server-rendered HTML uses `<br>` inside `<h1>`. On viewports < 480 px the break creates a visually unbalanced tower. Not a demo blocker (the demo MP4 is recorded at desktop width), but worth flagging for the public marketing site.

#### F7 [P3] — Customer logos on `/customers` (Northwind / Beacon / Maple / Apex / Skyline) are stub avatars (initials in coloured circles), not the SVG logos the memory `feedback_logos_from_svgl_app` requires
Per the operator's 2026-05-21 directive, brand/customer logos must be SVGs downloaded from svgl.app and stored at `web/public/logos/<name>.svg`. Five fictional customers appear with `<span>NC</span>`-style placeholder badges. This is dressing-side (the demo doesn't drill into a customer story) — flag only.

File: `web/app/customers/page.tsx`

#### F8 [P2] — `/compare` page is 321 KB of HTML
Over 5× heavier than the next-largest marketing page. Suggests a giant inline data structure being rendered server-side rather than tabled on the client. Lighthouse score will hurt; not a POC blocker because the camera doesn't drill into it during the demo.

#### F9 [P3] — `/changelog` shows 13 entries in reverse chronological order, all 2026 dated. No "Future / Roadmap" cut-line — investors clicking through during the demo would see only a wall of past work. Polish only.

#### F10 [P2] — `/pricing` shows three tiers (Free / Pro / Enterprise) but the in-app `/billing/subscribe` page is not linked from any tier card
Manual probe: there's a "Hire your first teammate" CTA but no "Subscribe" or "Choose Pro" affordance that maps to a checkout. For the POC investor demo this is fine (no Stripe in scope); flagging only because the public Pricing page implies you can buy a plan.

#### F11 [P3] — `/security` page header only mentions "SOC 2" once. Pricing and landing both claim "ISO 27001 controls supported" and "HIPAA workflows" as eyebrow chips, but the dedicated security page doesn't restate those four standards as a SOC-2/ISO/HIPAA/GDPR badge bar. Trust signal opportunity, not a bug.

### Auth + onboarding pages (out of the 6-scene runbook but I walked them)

#### F12 [P2] — All five `/onboarding/<step>` pages fall back to the marketing default `<title>` "SKYCOT — Hire your first AI teammate"
Operators recording Scene 3 will see the same tab label across welcome → workspace → connect-provider → pick-vertical → first-hire. The page content per step is correct and matches the F1 editorial-center lock-in (verified by header H1 "Hire your first teammate.").

File: `web/app/(setup)/onboarding/welcome/page.tsx` (and the four sibling step pages)

Fix: add per-step metadata, e.g. `{ title: "Welcome · Onboarding" }`.

#### F13 [P3] — `/auth/signup` page title reads "Hire your first AI teammate · SKYCOT"
Marketing-rich but ambiguous — users mid-signup don't immediately recognise the tab as the signup screen. `/auth/signin` correctly reads "Sign in · SKYCOT". Polish, not blocker.

### In-app pages (outside the 6-scene hot path)

#### F4 [P1] — `(app)/layout.tsx:20` default title "Approvals" leaks into five sibling pages
Five `(app)/*` pages render with the browser tab "Approvals · SKYCOT" while showing entirely different content:
- `/brand` (H1: "SKYCOT [wordmark]") — should be "Brand · SKYCOT"
- `/brand/components` (H1: "SKYCOT") — should be "Components · SKYCOT"
- `/retro` (H1: "Retros") — should be "Retros · SKYCOT"
- `/genesis` (H1: "Hire your next teammate.") — should be "Genesis · SKYCOT"
- `/orchestration` (H1: "Orchestration") — should be "Orchestration · SKYCOT"

The metadata template at `web/app/(app)/layout.tsx:19-22` is:
```ts
title: {
  default: "Approvals",
  template: "%s · SKYCOT",
}
```
Pages that don't export `metadata` fall through to the literal default "Approvals". For the demo MP4, the camera doesn't drill into the tab bar of these specific pages, so this is P1 not P0. But it's the most-visible polish gap across the in-app shell.

Fix: change the default to a neutral string like `"Skycot"` AND add `export const metadata` to the five orphan pages.

#### F2 [P1] — `/skills` marketplace renders 0 cards. Scene 4 unrecordable.
Page HTML loads correctly with the hero ("Every skill your team…"), `⌘K` search affordance, and the locked A1 marketplace layout. But `<h2 class="skill-card-*">` count is **0**. Gateway returns `{"skills":[],"tenant_id":"019e5057-…","total_count":0}` — the boot-time seed from commit `2639ed167` ("fix: seed demo marketplace skill catalog on gateway boot") is not populating this tenant.

This is the **functional half** of the demo per the function-vs-dressing rule. Scene 4 reads:
> Search "competitor pricing scrape" → 3 results. Click result → install flow.

With zero rows in the table, no result will appear and the demo recording stalls at the empty search state.

Repro:
- `curl http://127.0.0.1:7400/api/v1/skills` → `{"skills":[],"total_count":0}`
- Visit `http://127.0.0.1:3000/skills` → hero + search bar render, marketplace grid empty

Likely root cause: the seed in commit `2639ed167` either (a) runs against a tenant that doesn't match the SPA's default-resolve tenant, or (b) was gated on a startup condition that's not firing on this gateway. Worth cross-checking with the agent who shipped the commit.

File: gateway boot is at `crates/argo-cli/src/handlers/gateway/boot.rs`; seed call to inspect would be in `argo-marketplace` or `argo-store`.

#### F3 [P1] — `/api/v1/workers` returns 404 anon while `/api/v1/team` returns 200 anon
Both routes resolve to the same handler (`crates/argo-cli/src/handlers/gateway/routes_team_list.rs::list_response`). The dispatch in `routes_dispatch_get_api_groups.rs:65-69` registers both. But:
- `curl http://127.0.0.1:7400/api/v1/team` → 200 OK + worker "Maya" payload
- `curl http://127.0.0.1:7400/api/v1/workers` → 404 `{"error":"not found"}`

Worth a real Rust look — possibly a workspace-tenant resolution path returns store_unavailable() differently between the two entry points, or the dispatch is mounted in two passes and only one is reached.

This is what causes the SPA workers grid (Scene 1) to fall over to the fixture roster even though `/team` could provide real data. The fixture has Mariah/Peta/Priya/Jamal/Nori; the real tenant has only "Maya". The demo will read fictional teammates the operator never hired.

Repro:
- `curl http://127.0.0.1:7400/api/v1/team` → 200 with `{"workers":[{"name":"Maya",…}]}`
- `curl http://127.0.0.1:7400/api/v1/workers` → 404
- Both routes share `routes_team_list::list_response` per `routes_workers_list.rs:14`

File: `crates/argo-cli/src/handlers/gateway/routes_workers_list.rs` (the delegate) + the dispatch chain in `routes_dispatch_get_api.rs::dispatch_api_get`.

#### F14 [P1] — `/cohorts` shows empty-state "Build your first cohort." instead of the locked D3 magazine tiles + 6 dept tabs
The locked design (memory `project_cohorts_page_design_locked`) calls for 6 dept tabs (Sales, Support, Marketing, Recruiting, Finance, Project & Ops) with a 1 lead-tile + 2 stacked + 4 compact magazine layout. The page implementation (`web/app/(app)/cohorts/page.tsx`) imports `CohortDeptTabs`, `CohortMagazineGrid`, and `dept-registry.ts` correctly — but the data hook (`useCohorts`) returns empty, so the page renders the empty-state.

This connects directly to F1 (the data hook is CORS-blocked from reaching `/api/v1/cohorts`, which does return 200 server-side with a real payload).

Repro:
- `curl http://127.0.0.1:7400/api/v1/cohorts` → 200 (real data)
- Open `http://127.0.0.1:3000/cohorts` → "Build your first cohort." empty-state, no dept tabs visible

Fix: F1 resolves this.

#### F15 [P1] — `/workflows` page renders 50 workflow-template cards but no Audiowide hero gradient (brand-restrained vs brand-loud mismatch)
The page H1 is `<h1 class="workflows-hero">` with the text "How your teammates…" — but inspecting the rendered styles, the hero is plain. The locked workflow-goldens design (memory `project_workflow_goldens_page_design_locked`) calls for the brand-loud register on the catalog. Compare against `/skills` and `/cohorts`, both of which render their `BrandLoudHero`. Could be intentional — `/workflows` may be the magazine-tile catalog and individual workflow detail pages are the brand-loud surface. Flag as P2 polish at most.

#### F16 [P2] — `/team` page renders TWO `<h1>` elements on the same page
First H1: `<h1 class="text-xl font-semibold tracking-tight">Team</h1>` (likely the nav-shell breadcrumb).  
Second H1: `<h1 class="skycot-brandloud-title">Hire your first teammate.</h1>` (the brand-loud hero in the empty state).  
Plus a third "Team" H1 lower in the document tree.  
Multiple H1s break semantic HTML and screen-reader-tab order. The duplicate "Team" H1 is from the app shell + page header both labeling themselves H1.

File: `web/app/(app)/team/page.tsx` + `web/components/skycot/AppShell*.tsx`

#### F17 [P2] — `/settings/*` sub-pages each render two H1s: a sticky shell "Settings" H1 and the page-specific H1
Same pattern as F16 — verified on `/settings/security` ("Settings" + "Security") and `/settings/api-keys` ("Settings" + "API keys"). For accessibility this is a soft violation; for visual design it's expected (the "Settings" label is a nav heading, the second H1 is the section heading). Recommend demoting the shell to `<h2>` or using `role="banner"` only.

#### F18 [P3] — `/settings/security`, `/settings/api-keys` H1s carry `<h2>` "Production ready" and "Every change leaves a trail" sub-headings that look like marketing copy on a settings page
The H2 text ("Production ready", "Every change leaves a trail", "Destructive changes are gated") matches a marketing/landing-style register. Inside `/settings/<tab>` this feels off — settings pages should be utilitarian. Either the H2s belong on the marketing `/security` page (where they already exist) or they should be demoted to body-copy on the in-app variants.

File: `web/app/(app)/settings/security/page.tsx`

#### F19 [P1] — `/skills-canvas` is a 404 (no top-level route exists)
The route only exists at `/workers/[workerId]/skills/[skillId]/canvas`. Briefs (search `streams/` for "skills-canvas") and the operator's e2e-agent system prompt both reference `/skills-canvas` as if it were a marketplace-level surface. Either:
- (a) The flat `/skills-canvas` page was never created, or
- (b) The reference is stale and the canvas is correctly nested under a specific worker+skill.

If (a), this is a P1 — at least one brief points users at a 404. If (b), update references.

#### F20 [P3] — `/brand` and `/brand/components` are reachable through the URL but not linked from the app nav
Sidebar navigation lists Approvals / Team / Workers / Cohorts / Skills / Workflows / Audit / Settings. Brand + components live only at the URL level. Useful for designers + dev; not visible to operators. Probably intended.

#### F21 [P3] — `/genesis` page H1 reads "Hire your next teammate." (brand-loud Audiowide gradient) — but the page is the Genesis onboarding studio, not a hire flow
Operator-clarified scope (memory `feedback_genesis_is_ai_teammate_onboarding_studio`) is voice/text role + SOP uploads + access controls + comprehension. The current H1 over-promises the hire wizard and under-promises the studio. Suggest "Onboard a new teammate." or "Define a role." instead.

File: `web/app/(app)/genesis/page.tsx`

#### F22 [P3] — `/retro` page is a stub: H1 "Retros" then nothing of obvious substance in the server-rendered HTML
May be client-rendered (Next.js client component) so the snapshot misses interactive content. Not a demo blocker (the runbook doesn't enter `/retro`).

### Theme + viewport + console hygiene

#### F23 [P2] — Theme toggle button exists in the app-shell ("Switch to light theme" / "Switch to dark theme") and `ThemeProvider` is wired in `web/app/layout.tsx`
Confirmed at the HTML level. Could not interactively test the toggle because the Preview chrome wedged. Trust the implementation; one quick smoke before recording (open `/audit` in dark, take screenshot, click toggle, confirm light) would close this out.

#### F24 [P3] — Marketing pages don't appear to have a separate mobile nav drawer
The in-app shell has `data-testid="navdrawer-open"` for mobile. Marketing pages were not inspected at 375 px because the preview was stuck. The HTML for `/landing` includes a `<sectionheader>` with desktop nav links — no obvious hamburger trigger. Worth a manual phone-width pass before publishing the public marketing site.

---

## 6-MISSION-claim coverage rollup (the load-bearing summary)

Reminder: per the function-vs-dressing rule, "LIVE in the SPA" means a path the demo camera traverses produces real (non-fixture) data and renders the locked design. "LIKELY-LIVE" means the page renders and the backend cold tests pass, but I couldn't observe end-to-end from the browser this session (CORS blockage; preview wedged).

### Claim 1 — Teammate, not tool 🟡 LIKELY-LIVE (backend) / 🔴 BROKEN-IN-SPA (browser)
**Evidence**:  
- `/workers` and `/team` pages render the C2 personality / trading-card-deck layout (H1 + worker cards inspected in HTML).  
- BUT the workers visible on screen come from `web/lib/sample-workers-detail.ts` fixtures (Mariah/Peta/Priya/Jamal/Nori), not from the real `/api/v1/workers` (which 404s — F3) or `/api/v1/team` (which 200s with the actual workspace's single worker "Maya" — but the SPA doesn't fall through to /team).  
- The demo banner ("Demo mode — showing sample data") is visible at the top of every signed-in screen, contradicting the claim that "these are your colleagues."  
**Demo-recording implication**: the fictional roster matches the runbook narration, but the visible "Demo mode" banner gives the game away. Either disable the banner for the demo recording (env-var toggle) or fix CORS so a real seeded workspace is read.

### Claim 2 — Delegation is the unit ❓ NOT-OBSERVABLE-FROM-SPA
**Evidence**:  
- No `/handoffs` or `/delegations` SPA route. The audit page IS designed to render handoff chains (`current_handoff_id` per backend memory `Status` block) but with the API CORS-blocked I couldn't drive a handoff and verify the UI renders it.  
- Backend round-54 evaluator confirmed claim 2 LIVE (delegate-task fallback-chain inheritance wired); SPA presentation layer for handoffs not directly tested in scope.  
**Demo-recording implication**: Scene 6 says audit page renders ≥10 rows with hash-chain hover. The /audit endpoint server-side returns real data; whether the SPA visualises handoff edges is unverified.

### Claim 3 — Platform builds workers 🟡 LIKELY-LIVE
**Evidence**:  
- `/onboarding/welcome` → `/workspace` → `/connect-provider` → `/pick-vertical` → `/first-hire` all return 200 with the F1 editorial-center layout (server-rendered H1 + correct page structure per locked design).  
- `/hire` page returns 200 with title "Hire — SKYCOT · SKYCOT".  
- `/genesis` page renders the studio shell (Audiowide H1, brand-loud Hero) — though the H1 copy ("Hire your next teammate.") is mismatched against the operator-clarified studio scope (F21).  
- Backend round-54 evaluator confirmed claim 3 LIVE.  
**Demo-recording implication**: Scene 3 is recordable — the five F1 pages render. Whether the `/api/v1/onboarding/first-hire` POST works through the browser is gated on F1 (CORS). The runbook may want to set up the seeded "Maya" worker as the hire result.

### Claim 4 — Skills shared + learned 🔴 BROKEN-IN-SPA
**Evidence**:  
- `/skills` page renders the A1 marketplace hero + search bar but zero skill cards (F2).  
- Gateway endpoint `/api/v1/skills` returns 200 with `{"skills":[],"total_count":0}` — the boot-seed from `2639ed167` is not running for this tenant.  
- `/skills-canvas` flat route 404s (F19).  
- Backend cold rerun on 2026-05-25 confirmed claim 4 LIVE for the `claim_4_live_install_invoke` test.  
**Demo-recording implication**: Scene 4 (search "competitor pricing scrape" → 3 results) is unrecordable until the seed lands and CORS is fixed. This is the biggest gap I found.

### Claim 5 — Any-PC execution ❓ NOT-OBSERVABLE-FROM-SPA
**Evidence**:  
- `/workflows/templates` lists 50 templates, none named `customer_research_followup` but plenty that combine MCP-style + computer-use steps (`qualify_lead`, `competitor_intel_digest`, `book_demo`, etc).  
- Workflow detail pages exist at `/workflows/[verticalId]/[name]` but I didn't drive a run from the browser (CORS-blocked and the operator still owes macOS permissions per CLAUDE.md "Remaining for v1.0 launch" item (c)).  
- Backend evaluator round-51 confirmed claim 5 substrate 47/47 cold cargo PASS.  
**Demo-recording implication**: Scene 5 is recordable in theory but unverified from the SPA side this run. Macos cu-permissions are still operator-only blockers.

### Claim 6 — Multi-channel + multi-tenant 🟡 LIKELY-LIVE
**Evidence**:  
- `/settings/notify-channels` returns 200 with the channel-config UI (verified at HTTP level).  
- `/settings/connectors` lists 8+ integrations server-rendered (Airtable, Asana, Calendly, ClickUp, Discord, Figma, Fireflies, Google Calendar — likely more below the fold).  
- `/audit` page exists; cross-tenant negative pin shipped via #2257 per CLAUDE.md.  
- Backend cold rerun 2026-05-25 confirmed `claim_6_live_cross_tenant` PASS.  
**Demo-recording implication**: Scene 6 audit-page drill-in is recordable once CORS is fixed and the audit endpoint returns rows in the SPA (server-side it already does — sampled 1+ audit row in `/api/v1/audit` response).

---

## Resolved since last audit
n/a — this is the first whole-site rollup at this scope.

## Orientation score
**62 / 100.**

Reasoning: the design system, locked-design implementation, and gateway backend are all in great shape (cold tests pass, server-side endpoints return real data, the F1/D3/C2 designs are in JSX). What's broken is the **wire between SPA and gateway in the browser** — CORS missing + skills seed empty + workers route 404. That single class of bug downgrades every claim's user-visible verdict from LIVE to LIKELY-LIVE-or-BROKEN. Fix CORS + re-run the skills seed and the same code base would score 85+. Most of the remaining P2/P3 items are SEO/metadata polish that doesn't affect the demo MP4 at all.

**One-bullet recommendation for the operator**: spawn a single agent task scoped to "add `tower-http::cors::CorsLayer` to the gateway and verify the boot-time skills seed runs against the workspace tenant the SPA resolves to." Recording the demo is functionally blocked until that lands.
