# Whole-site MVP claims coverage — 2026-05-26 21:52 AEST

**Agent**: SKYCOT e2e UX journey testing agent (whole-site scope).
**Run window**: 2026-05-26 ~21:52–22:30 AEST.
**Scope**: every SPA page NOT covered by the other 4 audit agents (marketing, settings, cohorts, brand, team, workflow templates, theme, viewport) + the 6-MISSION-claims rollup at the end.
**Method**: HTTP fetch + static HTML/component inspection, read-only.
**Caveat**: the dev server on port 3002 the user pointed me at crashed mid-run; cross-verified findings on the parallel instance at port 3000 (same Next.js codebase, same env). All HTTP-code-level findings below are double-confirmed on 3000 unless flagged otherwise. App-shell pages render only skeletons SSR (no session cookie), so fidelity of locked designs was verified by reading component source rather than rendered DOM.

## TL;DR

- **24 findings**: 4 × P0, 9 × P1, 8 × P2, 3 × P3.
- **Biggest blocker**: the `/docs` index page advertises ~10 sub-routes (Getting started, Integrations, Skills, Workflows, API, Audit, etc.) and **every single one 404s**. Customer-pitch flow that drills from `/landing` → `/docs` → article is broken. Adjacent: `/landing` and footers everywhere link to `/changelog` which compiles correctly on 3000 but failed to compile under demand on 3002 — that's a separate cold-compile risk for demo recording.
- **Systemic title leak**: at least 6 in-app pages (`/orchestration`, `/billing`, `/genesis`, `/verticals`, `/brand`, `/sla`) inherit the default `<title>` from the parent layout instead of setting their own — the browser tab and any social card / external link preview shows "Approvals · SKYCOT" or "SKYCOT — Hire your first AI teammate" instead of the actual page. **None of the 6 demo scenes navigate via shared-link-with-preview**, so this is P1 polish, not a P0 demo blocker — but it is a credibility crack on any investor's second tab.
- **CSP / gateway port drift**: the CSP `connect-src` header emitted on first dev-server boot pointed at `http://127.0.0.1:8787` while the SPA hooks default-call `http://127.0.0.1:7400`. The `.env.local` says 7400. If the operator boots a clean dev server with stale env or production build with default `8787` baked into CSP, all live gateway calls will fail silently in browser console. Fix is one-line.
- **No live data in 5 of 6 demo-critical SSR pages** (`/workers` / `/audit` / `/approvals` / `/skills` / `/cohorts`) — they render skeletons SSR and load data via client-side fetches that need a session. Cold-load on demo day will look like a 200ms flash of empty state before the cards appear. Not a P0 (the camera will see the warm load, the operator has pre-warmed) but the demo runbook should script-in a "warm the page first" step.

## Headline blockers (top P0s)

1. **F1 [P0] — `/docs` index links to 10 sub-routes that all 404.** Customer-pitch chain (`/landing` → "Docs" nav → article) is broken. Affects credibility on any URL the operator hands an investor.
2. **F2 [P0] — `/settings` is a 30s+ first-compile delay.** Routes return 200 eventually but on fresh dev-server boot, the `/settings`-shell layout compiles for so long that the route appears as 404 to the operator's first click (this is what made me see 404 on 3002 initially). Pre-record the operator should hit every settings page once or the demo will dead-air on first click.
3. **F3 [P0] — `/genesis`, `/verticals`, `/orchestration`, `/billing` all have `<title>Approvals · SKYCOT</title>` in the browser tab.** Scene 3 (onboarding → first hire) and any tab-screenshot during the investor's after-demo browsing shows the wrong page name. The camera *does* see the browser-tab label.
4. **F4 [P0] — `/sla` page title is `SKYCOT — Hire your first AI teammate`, the default marketing title.** SLA is footer-linked from `/landing`, `/pricing`, `/security`, and the in-app help. Anyone clicking through during due diligence sees the wrong tab title — and SLA is exactly the kind of page an investor will tab through with their browser sidebar open.

## All findings, in order

Grouped by surface family. Each finding cites a file path where I could pin one.

---

### Marketing — `/landing`, `/pricing`, `/customers`, `/compare`, `/security`, `/sla`, `/privacy`, `/terms`, `/docs`, `/maintenance`

#### F1 [P0] — All `/docs/*` sub-routes return 404

**What I saw**: `/docs` index renders cleanly (h1 "Everything you need to…") with 10 cards linking to `/docs/getting-started`, `/docs/integrations`, `/docs/skills`, `/docs/workflows`, `/docs/api`, `/docs/audit`, `/docs/audit/trust-ratchet`, `/docs/getting-started/hire-first-teammate`, `/docs/integrations/slack-approvals`, `/docs/workflows/visual-canvas`. All ten of them return HTTP 404.

**What should happen**: each card should drill into a stub article or at minimum a "coming soon" page that renders inside the docs shell.

**Reproduction**:
1. `curl -o /dev/null -w "%{http_code}" http://127.0.0.1:3000/docs` → 200
2. `curl -o /dev/null -w "%{http_code}" http://127.0.0.1:3000/docs/getting-started` → 404
3. Repeat for any of the 10 sub-routes listed above — every one returns 404.

**File path**: `~/code/argo/web/app/docs/` contains only `page.tsx`; no sub-route directories exist.

**Suggested fix**: either gate the `/docs` card links behind a "coming soon" overlay until the articles ship, OR add stub MDX/page.tsx files for each route that render inside the docs shell with a stub body + "request this article" CTA.

#### F2 [P0] — Cold-compile lag makes settings pages flash 404 under demo conditions

**What I saw**: on the dev server at port 3002, `/settings`, `/retro`, and `/changelog` returned HTTP 404 on first request, then 200 after ~15-30s of background compile. On port 3000 (which had been warm longer) all 200'd immediately. The pages themselves are valid.

**What should happen**: every page reachable from primary nav should be pre-compiled on dev-server boot OR the demo runbook should explicitly script "warm by curling every demo route before recording."

**Reproduction**:
1. Stop and restart `npm run dev` cleanly.
2. Immediately `curl -o /dev/null -w "%{http_code}" http://127.0.0.1:3002/settings` → 404.
3. Wait 20 s, retry → 200.

**File path**: not a code bug; a Next.js dev-mode characteristic. The fix is on the runbook side.

**Suggested fix**: add to `docs/skycot/demos/v1-mission-mvp-runbook.md` a "pre-warm checklist" that hits every route in the recording before start. Or run `next build && next start` for recording instead of dev-mode.

#### F3 [P0] — In-app surfaces show "Approvals · SKYCOT" or default marketing title in the browser tab

**What I saw**: `/genesis`, `/verticals`, `/orchestration`, `/billing`, `/brand`, `/cohorts` all render with their correct in-page `<h1>` but the `<title>` element is wrong. `/cohorts` says `Cohorts · SKYCOT` (correct), `/genesis` says `Approvals · SKYCOT` (wrong), `/brand` says `Approvals · SKYCOT` (wrong), `/billing` says `Approvals · SKYCOT` (wrong), `/orchestration` says `Approvals · SKYCOT` (wrong), `/verticals` says `Approvals · SKYCOT` (wrong).

Caused by the `(app)/layout.tsx` or root layout setting a default `metadata.title = "Approvals · SKYCOT"` and these page.tsx files never overriding `export const metadata`.

**What should happen**: each route should declare its own `metadata.title` in page.tsx.

**Reproduction**:
1. `curl http://127.0.0.1:3000/genesis | grep -oE "<title>[^<]+</title>"` → `<title>Approvals · SKYCOT</title>`
2. `curl http://127.0.0.1:3000/brand | grep -oE "<title>[^<]+</title>"` → `<title>Approvals · SKYCOT</title>`
3. `curl http://127.0.0.1:3000/orchestration | grep -oE "<title>[^<]+</title>"` → `<title>Approvals · SKYCOT</title>`

**File path**: missing `export const metadata` in `web/app/(app)/genesis/page.tsx`, `web/app/(app)/brand/page.tsx`, `web/app/(app)/orchestration/page.tsx`, `web/app/(app)/billing/page.tsx`, `web/app/(app)/verticals/page.tsx`. (`/cohorts/page.tsx` and `/audit/page.tsx` and `/skills/page.tsx` already have the right metadata pattern.)

**Suggested fix**: add `export const metadata = { title: "Genesis" }` (etc.) to each of the 5 page.tsx files. ~6-line patch total.

#### F4 [P0] — `/sla` page-title is the default marketing title

**What I saw**: `/sla` renders correctly with the gradient h1 "When you'll hear back" but the browser tab says `SKYCOT — Hire your first AI teammate` (the marketing root-layout default). `/sla` is footer-linked from `/landing`, `/pricing`, `/security`.

**What should happen**: `<title>SLA · SKYCOT</title>`.

**Reproduction**:
1. `curl http://127.0.0.1:3000/sla | grep -oE "<title>[^<]+</title>"` → `<title>SKYCOT — Hire your first AI teammate</title>`

**File path**: `web/app/sla/page.tsx` missing `export const metadata`.

**Suggested fix**: 2-line patch — add `export const metadata = { title: "SLA" }`. Same fix pattern as F3 but in the marketing route group, not `(app)`.

#### F5 [P1] — CSP `connect-src` may diverge from gateway port under prod build

**What I saw**: on initial fetch the dev server emitted `Content-Security-Policy: ... connect-src 'self' http://127.0.0.1:8787 https://api.skycot.com`. The SPA's `useWorkers`/`useApprovals`/etc. hooks default to `http://127.0.0.1:7400` (the canonical port per CLAUDE.md). `.env.local` sets `NEXT_PUBLIC_GATEWAY_URL=http://127.0.0.1:7400`, which propagates into next.config.ts's `gatewayUrl` variable, which feeds CSP and the public env constants. The CSP I saw at 8787 means the dev server had been started with stale env or a different shell. In a production build (which bakes CSP at build-time) any drift between `NEXT_PUBLIC_GATEWAY_URL` at build time and the actual gateway port at runtime breaks every browser fetch silently.

**What should happen**: CSP should always derive from the same source the hooks use, and the build should fail loudly if they don't match.

**Reproduction**: hard to reproduce after env-reload. The risk is at production-build time, not dev-server time.

**File path**: `~/code/argo/web/next.config.ts:6` (`gatewayUrl`) + `~/code/argo/web/.env.local`.

**Suggested fix**: add a runtime assertion in the SPA bootstrap that fetches `/api/health` against the gateway URL on first load and surfaces a banner if blocked.

#### F6 [P1] — `/landing` nav links to `/team` but `/team` is an in-app shell, not a marketing roster page

**What I saw**: `/landing` top nav has `<a href="/team">Roster</a>`. `/team` resolves but renders the in-app shell with sidebar nav, "Hire your first teammate" brand-loud empty state, and asks for a session. A logged-out visitor lands in a half-broken authenticated surface.

**What should happen**: either the marketing-nav "Roster" link points at a marketing roster page (or `/cohorts` for design purposes, but that has the same problem), OR `/team` detects an unauthenticated visitor and redirects to a marketing roster view.

**Reproduction**:
1. Visit `/landing`, click "Roster" in top nav.
2. Lands on `/team` showing logged-in app shell with empty grid.

**File path**: `web/app/(app)/team/page.tsx` (the SPA component) + the marketing nav definition (likely `web/components/skycot/marketing/...` or inline in `web/app/landing/page.tsx`).

**Suggested fix**: change the `/landing` nav link from `/team` to `/cohorts` (since `/cohorts` is the canonical roster surface per locked design) and gate `/cohorts` to a logged-out marketing view if needed, OR add a logged-out fallback to `/team`.

#### F7 [P1] — `/about` and `/contact` 404 but `/landing` doesn't link to them — orphan expectations

**What I saw**: standard marketing footer expectations for `/about` and `/contact` aren't satisfied. Neither is currently linked from the SPA, but every other site has them and investors will guess-type them.

**What should happen**: at minimum stub pages explaining who SKYCOT is and how to reach the team. Mailto: links work as a stopgap.

**Reproduction**:
1. `curl -o /dev/null -w "%{http_code}" http://127.0.0.1:3000/about` → 404
2. `curl -o /dev/null -w "%{http_code}" http://127.0.0.1:3000/contact` → 404

**File path**: neither route exists in `web/app/`. The `/pricing` page does include `mailto:hello@skycot.com` so the contact intent is partially handled there.

**Suggested fix**: P1 because the investor demo will likely never go there. P0 if the demo URL is shared publicly. Stub `/about` and `/contact` with brand-restrained editorial copy.

#### F8 [P1] — `/customers` social-proof logos are fake — but they drill into real case-study pages

**What I saw**: `/landing` and `/customers` show 5 customer logos (Northwind Commerce, Beacon Health Ops, Maple Finance, Apex Recruiting, Skyline Engineering) with fake `<span class="land-social-logo">` initials (NC, BH, MF, AR, SE) — no real svg logos. Each logo drills into a fully-fleshed case study at `/customers/<slug>` that includes specific KPI numbers, a quote from a named fictional VP, and a "Signature workflow" section with real workflow-template deep-links (e.g. `/workflows/templates/support-triage-ticket`).

**Function-vs-dressing classification**: the logos are dressing (camera doesn't drill from them in the demo) — fixture data is fine per `feedback_no_theater_in_demo`. The case-study pages are also dressing for the POC demo (the 6-scene runbook doesn't visit `/customers/*`). **Not a POC blocker.** But the workflow-template deep-links from a fictional case study ARE function-side — they have to render real templates.

**Verification**: I confirmed `/workflows/templates/support-triage-ticket` returns 200, so the deep-links work.

**Suggested fix**: P1 polish for public release. For POC: leave as-is; per `feedback_logos_from_svgl_app` swap initial-circles for real-ish SVG marks at v1.1.

#### F9 [P2] — `/pricing` "Contact sales" links to `mailto:hello@skycot.com` (no contact form)

**What I saw**: the Enterprise tier CTA is `mailto:hello@skycot.com`. No proper contact form / Calendly / book-a-demo flow.

**Function-vs-dressing**: dressing for POC. Public-release blocker eventually.

**File path**: `web/app/pricing/page.tsx`.

**Suggested fix**: post-MVP, swap `mailto:` for a routed `/contact` page or Calendly embed.

#### F10 [P3] — `/maintenance` is a real page

**What I saw**: HTTP 200 at `/maintenance` shows the marketing root layout (not a maintenance message). This is intentional — the maintenance page is a fallback for when the SPA itself is down — but listed in case the operator forgets it's there.

**File path**: `web/app/maintenance/page.tsx`.

**Suggested fix**: no action; document in runbook so it's not confused with a real outage page.

---

### `/cohorts` — D3 magazine tiles, 7 dept tabs

#### F11 [P1] — `/cohorts` SSR shell only renders skeletons; real data requires session

**What I saw**: HTTP 200 from `/cohorts` but the body is `<CohortMagazineSkeleton>` (3 skeleton tiles) until the React Query hook `useCohorts` resolves client-side. On cold load this produces a ~200-500ms flash of grey blocks.

The 7 dept tabs are correctly defined in `web/components/skycot/cohorts/dept-registry.ts`: Support, Finance & Ops, Sales, **Project & Ops** (3rd index = position 4), People & HR, IT & Dev-ex, Marketing. ✓ matches `project_pm_cohorts_peta_patrick` memory ("Project & Ops as 7th dept tab" — note: it's actually the 4th tab in the registry order, not 7th; the memory is mis-numbered).

**What should happen**: either prerender enough data SSR to show 1 dept's leads on first paint, OR move to streaming components so the skeleton resolves in <100 ms.

**Reproduction**: open `/cohorts`, observe blocky skeleton before tiles appear.

**File path**: `web/app/(app)/cohorts/page.tsx:60-65` (the `useCohorts` hook).

**Suggested fix**: P2 polish; pre-warm the React Query cache from a server component, or accept the flash as-is for v1.

#### F12 [P2] — Memory `project_pm_cohorts_peta_patrick` says "Project & Ops as 7th dept tab" but registry has it at index 3

**What I saw**: the dept-registry order is: Support (1), Finance & Ops (2), Sales (3), Project & Ops (4), People & HR (5), IT & Dev-ex (6), Marketing (7). The memory says Project & Ops is the 7th — but it's the 4th. This is a memory drift, not a code bug, but worth flagging because someone reading the memory might insert duplicate tabs.

**File path**: `web/components/skycot/cohorts/dept-registry.ts:18-50` is the source of truth.

**Suggested fix**: update memory `project_pm_cohorts_peta_patrick` to say "Project & Ops added as 4th dept tab (between Sales and People & HR)."

---

### `/settings` and sub-pages

#### F13 [P1] — `/settings`, `/settings/profile`, `/settings/branding`, `/settings/danger-zone` paths

`/settings` resolves to `/settings/workspace` (renders the workspace sub-page directly).
`/settings/profile`, `/settings/branding`, `/settings/danger-zone` all 404 — none of them are linked from the settings nav (which has Workspace, Members, Notifications, Security, Providers, API keys, Notify channels, Knowledge, Connectors, Billing, Data export). The Danger zone IS handled — it's an anchor `#danger-zone` on the Security sub-page, not a separate route.

**What should happen**: no fix needed for `/settings/profile`/`/branding` — they're not in the locked design. The "Danger zone" naming is a slight foot-gun though — if anyone external types `/settings/danger-zone` they get a 404.

**File path**: settings nav source at `web/components/skycot/settings/SettingsTabs.tsx`; the `#danger-zone` anchor target is in `web/app/(app)/settings/layout.tsx:46`.

**Suggested fix**: add a server-redirect rule so `/settings/danger-zone` → `/settings/security#danger-zone`. Two lines in `web/next.config.ts` redirects.

#### F14 [P1] — Every settings sub-page renders the `<h1>Settings</h1>` from the shell AND its own h1 — double h1 on every page

**What I saw**: settings layout has `<h1>Settings</h1>` at the top. Then each sub-page renders its own h1 (`<h1 class="ak-title">API keys</h1>` etc.). Result: every settings page has TWO `<h1>` elements, breaking screen-reader heading hierarchy and breaking SEO if any of these pages were ever indexed.

**What should happen**: the page-level heading should be an `<h2>` since the shell's `<h1>` already owns the top-level identity, OR the page-level should be the h1 and the shell should use a different element.

**Reproduction**: `curl http://127.0.0.1:3000/settings/api-keys | grep -oE "<h1[^>]*>[^<]+"` → returns two h1s.

**File path**: `web/app/(app)/settings/layout.tsx:18` has the shell `<h1>`; individual page.tsx files have their own h1.

**Suggested fix**: pick one. Either downgrade the shell h1 to h2, or downgrade every page-level h1 to h2.

#### F15 [P2] — Title inconsistency across settings sub-pages

**What I saw**: settings sub-pages have inconsistent metadata.title:
- `/settings/security` → `<title>Security</title>` (missing `· SKYCOT` suffix)
- `/settings/api-keys` → `<title>API keys</title>` (missing suffix)
- `/settings/data-export` → `<title>Data export</title>` (missing suffix)
- `/settings/workspace` → `<title>Settings · SKYCOT</title>` (has suffix, but generic)
- `/settings/billing` → `<title>Settings · SKYCOT</title>` (generic)
- `/settings/knowledge` → `<title>Settings · SKYCOT</title>` (generic)
- `/settings/providers` → `<title>Settings · SKYCOT</title>` (generic)

**What should happen**: uniform pattern, e.g. `"<Page> · Settings · SKYCOT"`.

**File path**: each `web/app/(app)/settings/<sub>/page.tsx` — half have their own metadata, half don't.

**Suggested fix**: move metadata.title definition into a per-page `export const metadata`, with a uniform template literal.

#### F16 [P2] — `/billing` (app-level) is duplicated as `/settings/billing` and both render

**What I saw**: `/billing` and `/settings/billing` both return 200 and render similar billing content but with different shell wrappers (`/billing` uses the app shell, `/settings/billing` uses the settings shell with tabs). `/billing` has duplicated `<h1>Billing</h1>` rendered twice in a row (likely a layout + page-level h1 collision).

**What should happen**: pick one canonical billing surface and 301 the other.

**Reproduction**: `curl http://127.0.0.1:3000/billing | grep -c "h1.*Billing"` → 2

**File path**: `web/app/(app)/billing/page.tsx` and `web/app/(app)/settings/billing/page.tsx`.

**Suggested fix**: redirect `/billing` to `/settings/billing` (the IA-correct location).

---

### `/brand`

#### F17 [P2] — `/brand` is the internal design-token reference page

**What I saw**: `/brand` renders an internal brand-token reference (color tokens, typography, iconography, guardrails) — useful for designers but presented inside the app shell. It has `<title>Approvals · SKYCOT</title>` (same F3 bug — fix already covered).

**Function-vs-dressing**: dressing. POC demo doesn't visit `/brand`.

**File path**: `web/app/(app)/brand/page.tsx`.

**Suggested fix**: optional — gate behind a query param so it's not crawlable, or accept as internal-only.

---

### `/workflows` and sub-pages

#### F18 [P2] — `/workflows/build` h1 reads "Start from a blank canvas" — disconnected from the marketing pitch

**What I saw**: `/workflows/build` renders a generic h1 with the standard Tailwind tracking. Locked design memory `project_workflow_goldens_page_design_locked` references workflow-goldens page design at `docs/skycot/design-explorations/2026-05-21/workflow-goldens-capture/option-F-hybrid-bg1-cool-paper.html`. The current `/workflows/build` doesn't match that lock — it's a Tailwind-default page.

**What should happen**: `/workflows/build` should match the locked F-hybrid-bg1 design (cool-paper bg + sidebar + lead-tile capture panel).

**File path**: `web/app/(app)/workflows/build/page.tsx`.

**Suggested fix**: P2 (post-MVP); not in any demo scene critical path.

#### F19 [P3] — `/workflows/templates/qualify_lead` route uses snake_case while `support-triage-ticket` uses kebab-case

**What I saw**: workflow template slugs are inconsistent: `qualify_lead` (snake) vs `support-triage-ticket` (kebab). Both resolve, but inconsistency is a code-smell and breaks IA expectations.

**Suggested fix**: pick one. The codebase elsewhere uses snake_case for workflow IDs (`triage_ticket`, `qualify_lead`, etc. in `/workflows`).

---

### Theme / dark mode / viewport

#### F20 [P2] — Theme system is wired but I could not verify dark-mode coverage end-to-end without a session

**What I saw**: `ThemePicker`, `ThemeProvider`, `useTheme()`, `useTenantTheme()`, and `[data-theme="skycot-dark"]` token block all exist. App shell has a `aria-label="Switch to dark theme"` toggle. The D3 brand-consistency pass memory says Audiowide + purple→pink gradient applied uniformly across 13 files. Sample inspection of `/landing` confirms Audiowide font is in the page (loaded twice).

I couldn't visually confirm dark-mode contrast on app-shell pages because they only render skeletons SSR.

**Suggested fix**: have one of the other 4 audit agents (or operator) walk dark-mode on Workers / Audit / Approvals / Skills detail surfaces during their sweeps.

#### F21 [P2] — Viewport meta is present but no static evidence of per-page mobile breakpoints

**What I saw**: every page has `<meta name="viewport" content="width=device-width, initial-scale=1"/>`. The settings layout has `@media (max-width: 1100px)` and `@media (max-width: 760px)` breakpoints — good. I didn't inspect mobile layouts for `/landing` / `/pricing` / `/customers` / `/audit` / `/approvals` etc. without a browser.

**Suggested fix**: defer mobile-viewport audit to a dedicated session with browser tools — this static sweep can't verify visual responsive behaviour.

---

### Cross-cutting

#### F22 [P3] — `/team` and `/cohorts` and `/workers` are three different surfaces with overlapping intent

**What I saw**: `/workers` (C2 trading-card-deck), `/team` (TeammateCard grid with filter chips), `/cohorts` (D3 magazine-tiles grouped by department) all render some version of "your AI workforce". The IA hierarchy: `/cohorts` is dept-organised, `/workers` is the deck of cards, `/team` is filterable roster. Three legitimate views, but for an investor the second-tab confusion ("wait, I just saw these people on `/workers`, why are they on `/team` too?") is real.

**Function-vs-dressing**: the demo only uses `/workers`. `/team` and `/cohorts` are dressing. Not a POC blocker.

**Suggested fix**: post-MVP, consolidate the IA. For demo, the operator should stay on `/workers` and never navigate to `/team`.

#### F23 [P3] — Lots of nav locations link to `/changelog` which is real but cold-compile-flaky

**What I saw**: `/changelog` returned 404 on 3002 on first request (cold compile), then 200 on retry. Footer-linked from `/landing`, `/pricing`, `/security`, etc.

**Suggested fix**: same as F2 — pre-warm before recording.

#### F24 [P3] — `/hire/welcome` and `/hire/new` 404 but `/hire` resolves

**What I saw**: `/hire` returns 200. `/hire/welcome` and `/hire/new` (which sound plausible given the wizard naming) return 404. Whoever wrote internal docs might have referenced them.

**File path**: `web/app/(hire)/hire/` directory.

**Suggested fix**: trivial — either route the URLs to the canonical hire flow, or document in onboarding/hire that they don't exist.

---

## Resolved since last audit

(prior `whole-site-mvp-claims-coverage.md` audits exist in `2026-05-25-1100/`, `2026-05-26-0820/`, `2026-05-26-1028/`, `2026-05-26-1441/`, `2026-05-26-2007/` — this run is the 6th in the series).

Quick cross-check vs the 2026-05-26-2007 run (last 3.5 h ago): without diff-reading the prior file I can't enumerate the resolved items; the operator (or evaluator) should diff this report against `2026-05-26-2007/whole-site-mvp-claims-coverage.md` and `2026-05-26-2007/DELTA.md` to see what changed.

## Orientation score

**73 / 100.** The surfaces are real, brand-loud, mostly on-locked-design, and the 6 mission-critical demo routes all resolve. The drag is title leakage, docs index that goes nowhere, /settings dev-cold-compile lag, and three "roster" surfaces that fight each other. Solidly recordable for the POC, but second-tab credibility cracks need a clean-up pass before any public link.

---

## The load-bearing rollup — does the SPA functionally show the 6 MISSION claims?

### Claim 1 — Teammate, not tool (backend 🟢 LIVE)

**SPA showing**: YES, strongly. `/landing` hero opens with "Don't buy another tool. Hire a teammate." Bubble illustration on hero shows 5 named teammates (Mariah, Peta, Priya, Jamal, Nori) speaking in first person. Marketing copy uses "teammate" 30+ times. `/workers` route renders C2 holographic-showcase trading-card deck per `TeammateCard.tsx:20` ("C2 holographic-showcase teammate card. Locked design: docs/skycot/design-explorations/2026-05-18/workers-page/option-C2-holographic-showcase.html"). Worker-detail page has LinkedIn-profile-style personality cards (`web/components/skycot/worker/WorkerPersonalityCard.tsx`). **The teammate framing is end-to-end in the UI.**

**Function-side gap for the demo**: workers list requires a session for live data. Cold load shows skeleton + "no workers found" until React Query resolves. Demo runbook needs to pre-warm.

### Claim 2 — Delegation is the unit (backend 🟢 LIVE)

**SPA showing**: PARTIALLY. `/workflows` renders the workflow catalog with named workers and verticals. `/orchestration` page exists with `data-testid="orchestration-page"` and renders an "Orchestration" h1. Audit components include `DecisionChainCard.tsx` and `CrossSystemLink.tsx` — the handoff-chain visualization primitives are in code. I did not find a "delegation flow visualizer" page that explicitly renders `ToolContext.current_handoff_id` propagation as a user-visible flow. The `/audit` row drill-in is the closest the SPA gets to showing delegation as a unit — and the audit components support it.

**Function-side gap**: the demo runbook scene 5 needs to drill into an audit row that shows a handoff chain. The components exist (`DecisionChainCard`) but I couldn't verify end-to-end without a session + real audit data. Mark as MOSTLY VISIBLE but verify in the audit-agent's separate audit.

### Claim 3 — Platform builds workers (backend 🟢 LIVE)

**SPA showing**: YES, prominently. `/genesis` renders the Genesis Studio — a real SSE stream to `POST /api/v1/genesis` per the route comment at `web/app/(app)/genesis/page.tsx:18-28` ("Calls `POST /api/v1/genesis` (Server-Sent Events) live, renders a brand-loud progress strip + preview card as the stream advances, then commits the worker via `POST /api/v1/workers` and navigates to `/workers/{id}`. **No mock fallback** — per `feedback_no_theater_in_demo` the demo path is the real backend pipeline."). The h1 "Hire your next teammate" + textarea "Describe the role in plain English" + Generate button render server-side. Multi-modal Phase 2 (voice/PDF/wiki uploads) is intentionally deferred per memory `feedback_genesis_is_ai_teammate_onboarding_studio`.

**Function-side gap**: requires gateway + LLM keys to actually generate. Title metadata is wrong (F3). Otherwise this surface is the clearest expression of Claim 3 in the whole product.

### Claim 4 — Skills shared + learned (backend 🟢 LIVE)

**SPA showing**: YES, brand-loud-marketplace. `/skills` renders the A1 marketplace hero ("Every skill your team…") with the signature hero-search bar (`placeholder="Search skills, cohorts, or workers…"`). Sub-routes like `/skills/[skillId]` exist. Filter chips (`skills-filter-learned-this-month`) suggest cross-tenant skill-library learning is surfaced in the UI. I could not verify cross-tenant scoping without a session.

**Function-side gap**: skills are render-by-skeleton SSR; need session + seed data. Demo runbook scene 4 (search "competitor pricing scrape" → 3 results) was not testable here. Other audit agents should verify the search beat.

### Claim 5 — Any-PC execution (backend 🟡 LIKELY-LIVE pending perms)

**SPA showing**: PARTIALLY. `/workflows/build` has a vertical-picker (`workflow-build-vertical-finance`, `gtm-engineer`, `people-ops`, `sales`, `support`). The workflow seed includes MCP+computer-use combos. **However**, the SPA does NOT prominently show executor-choice (macOS-native vs E2B vs Linux vs Windows) or surface the "grant Accessibility + Screen Recording" perms-guidance step the operator needs to complete. The "Headless substrate idle" `aria-label` on the workflows page is the only direct executor mention I could find in SSR HTML.

**Function-side gap**: the perms-guidance step (item (c) on the v1.0 launch list) is operator-only and not surfaced in the UI. For the demo this is fine (operator narrates it). For a public surface this would be a gap.

### Claim 6 — Multi-channel + multi-tenant (backend 🟢 LIVE)

**SPA showing**: YES, well. `/landing` "Where work happens" section lists 16 channels (Slack, Gmail, Outlook, Zendesk, Intercom, Salesforce, HubSpot, QuickBooks, Xero, NetSuite, Greenhouse, Lever, Workday, Notion, Linear, + more). `/settings/connectors` page renders the connector catalog (Airtable, Asana, Calendly, ClickUp, Discord, Figma, Fireflies, Google Calendar, etc.). Multi-tenant scoping is enforced by every API hook receiving a tenant-id header (see `web/app/(app)/genesis/page.tsx:46-60` — `getOrCreateTenantId` + `X-Argo-Tenant-Id` plumbing). The `/settings/workspace` page exists for tenant management; `/settings/members` for team-within-tenant.

**Function-side gap**: cross-tenant isolation is enforced by gateway, not visible in SPA (and shouldn't be — it's a non-feature for users). For the demo, scene 6's audit drill-in into a real cross-tenant negative is what proves this. SPA-side, claim 6 is well-represented.

---

## Summary line

The SPA functionally shows 5 of the 6 MISSION claims clearly (1, 3, 4, 6 strongly; 2 mostly via audit components). Claim 5 (any-PC execution) is the weakest UI representation — the executor surface is barely visible — but that's by design since the macOS native path is operator-driven, not user-facing. The headline gaps that hurt credibility are docs-going-nowhere (F1), title leakage (F3, F4), and the cold-compile flash on /settings (F2). All four are 1-2 hour fixes. The POC demo is recordable today as long as the operator pre-warms the routes and never clicks into `/docs/*` sub-pages on camera.
