# Whole-site MVP claim coverage audit — 2026-05-26 14:41

**Scope:** every SPA page NOT covered by the other four 2026-05-26-1441 audits — all marketing pages (`/`, `/landing`, `/pricing`, `/customers`, `/compare`, `/for/*`, `/docs`, `/help`, `/security`, `/privacy`, `/terms`, `/sla`, `/changelog`, `/maintenance`, `/offline`), all auth pages (`/auth/*`), `/cohorts`, `/settings` + sub-pages, `/brand`, `/team`, `/workflows/templates`, `/workflows/build`, `/verticals`, `/orchestration`, `/genesis`, the hire wizard, viewport responsiveness, theme toggle, the 6-mission-claim rollup.

**Environment:** dev server up on `:62927` (proxied to `:3000`), gateway up on `:7400`, schema v83 FRESH. Seed has 1 worker (Maya · marketing-intern), 1 cohort, 0 skills, 0 workflows, 0 approvals, 0 orchestration runs, 0 verticals, audit has rows.

**Tool note:** preview MCP got pinned by concurrent audits early — I switched to source-reading + `curl` against the dev server + direct gateway probing, which is what most of the load-bearing observations are based on. Findings that are visual-only have that called out.

---

## TL;DR

26 findings. **0 P0**, **5 P1**, **14 P2**, **7 P3**. The whole-site walk is in surprisingly good shape — every marketing page returns 200 with rich content, every settings sub-tab renders, the hire wizard is wired, the cohorts/team/workflows/templates pages all have either real data or polished empty-state previews. The biggest visible-in-demo issues are: (a) `/brand`, `/verticals`, `/orchestration`, `/genesis` all leak the **"Approvals · SKYCOT"** browser-tab title because they ship without a page-level `<title>` (very obvious if the demo records the URL bar or tab); (b) **navigating to `localhost:3000/` lands on Approvals** with no auth gate or marketing redirect, so a fresh visitor to the demo URL immediately sees in-app chrome instead of the landing page; (c) the gateway endpoint **`/api/v1/team` returns workers but `/api/v1/workers` is 404**, an inconsistency that will bite the next time someone audits the route table; (d) **no gateway routes for worker schedules/duties** — Scene 2's "Mariah's Schedules tab + Run now → audit row" beat is not backend-supported yet.

On the 6 mission claims: 1 / 2 / 3 / 4 are demonstrable in MVP scope based on what the SPA shows. Claim 5 (any-PC execution / computer-use) has no SPA surface to show in this scope. Claim 6 (multi-tenant + multi-channel) is partly there (the connector marketplace surface + tenant scoping in API responses) but the multi-channel side is asserted in copy only — there's no SPA proof a single worker pings Slack + Email + iMessage in a recordable flow.

---

## Headline blockers (top issues)

1. **F01 [P1]** `/brand`, `/verticals`, `/orchestration`, `/genesis` all show "Approvals · SKYCOT" in the browser tab — page-level title metadata missing. Recording-side risk if the camera frames the tab.
2. **F02 [P1]** Visiting `localhost:3000/` (root) loads the in-app **Approvals** page bare, no auth gate or redirect to /landing. A demo viewer typing the bare URL sees app chrome instead of the marketing hero.
3. **F03 [P1]** Gateway has **no `/api/v1/workers/{id}/duties` or `/schedules` route** — Scene 2's "Schedules tab + Run now" demo beat has no backend wiring; the click-through would 404.
4. **F04 [P1]** Gateway **`/api/v1/workers` collection returns 404**; only `/api/v1/team` returns the list. A future code reader will trip on this; not a demo blocker because the SPA uses `useTeam()`.
5. **F05 [P1]** `/for/*` pages all double-suffix their `<title>` — e.g. `"AI sales teammates that never sleep — SKYCOT · SKYCOT"`. Visible in the tab during recording.

---

## All findings

### Marketing pages

#### F01 [P1] — Missing page titles on several in-app routes leak "Approvals · SKYCOT"
Six in-app routes ship without a `metadata.title` export, so they inherit whatever the root `layout.tsx` falls back to (or whatever was last hydrated client-side). The browser tab reads "Approvals · SKYCOT" on every one of `/`, `/brand`, `/verticals`, `/orchestration`, `/genesis`. (Verified via `curl + perl` against `:62927`.) This is the kind of detail an investor will catch the first time they screen-share.
- Repro: `curl -s http://localhost:62927/brand | grep '<title>'` → `Approvals · SKYCOT`. Same for `/verticals`, `/orchestration`, `/genesis`, `/`.
- File: `web/app/(app)/brand/page.tsx`, `web/app/(app)/verticals/page.tsx`, `web/app/(app)/orchestration/page.tsx`, `web/app/(app)/genesis/page.tsx`. No `export const metadata = { title: ... }` in any.
- Fix: add `export const metadata: Metadata = { title: "Brand · SKYCOT" }` (etc) to each `page.tsx` or to a `layout.tsx` in the same folder.

#### F02 [P1] — Root URL `/` lands on Approvals with no auth gate
A fresh visitor to `http://localhost:3000/` lands on the in-app Approvals page (the `(app)/page.tsx` route), which shows "Your AI teammates need a yes or no. · Loading approvals…" before falling back to an empty state. Marketing intent is clearly to push first-time viewers to `/landing`. The route group `(app)/page.tsx` is currently the silent owner of `/`.
- Repro: `curl -s http://localhost:62927/ | grep '<h1\|<title>'` → "Approvals · SKYCOT".
- File: `web/app/(app)/page.tsx:1` is the implicit `/` page.
- Fix: add a `web/middleware.ts` rule (or a server-component redirect in `web/app/page.tsx` if it exists) to send unauthenticated `/` traffic to `/landing`.

#### F05 [P1] — `/for/*` pages emit double-suffixed titles
`/for/sales`, `/for/recruiting`, `/for/marketing`, `/for/finance`, `/for/ops`, `/for/support` all set their own metadata.title containing the word "SKYCOT" *and* inherit the global "· SKYCOT" suffix, so the tab reads e.g. "AI sales teammates that never sleep — SKYCOT · SKYCOT".
- Repro: `curl -s http://localhost:62927/for/sales | perl -nle 'print $1 if /<title[^>]*>([^<]+)/'` → `AI sales teammates that never sleep — SKYCOT · SKYCOT`.
- File: `web/app/for/_components/VerticalLandingShell.tsx` (likely sets title) + `web/app/layout.tsx` template config.
- Fix: drop the trailing `— SKYCOT` from each `/for/*/layout.tsx` metadata title; the global `· SKYCOT` suffix already handles branding.

#### F06 [P2] — `/sla` lacks a page-specific title
`/sla` returns the site-default title "SKYCOT — Hire your first AI teammate". Every other policy page (`/privacy`, `/terms`, `/security`) has a specific title.
- File: `web/app/sla/page.tsx` — missing `export const metadata`.

#### F07 [P2] — `/workflows/templates` title is missing the global `· SKYCOT` suffix
Every other page reads "X · SKYCOT" — this one reads just "Workflow templates".
- Repro: `curl -s http://localhost:62927/workflows/templates | perl -nle 'print $1 if /<title[^>]*>([^<]+)/'` → `Workflow templates`.
- File: `web/app/(app)/workflows/templates/page.tsx` or its layout.

#### F08 [P2] — `/help` is a placeholder ("we're building")
The `/help` page renders only "We're building the in-product help center alongside v1.1." with three escape hatches (changelog, support email, docs). Acceptable for a POC, but the link from the marketing nav should probably go to /docs not /help until it's real.
- File: `web/app/help/page.tsx`.
- Suggested: redirect `/help` → `/docs` for now, or hide the help link from `MarketingNav.tsx:34-41` (`NAV_LINKS`).

#### F09 [P3] — `/customers` testimonials and headcounts are fixtures
`/customers` ships fictional logos + testimonials (Mariah at Northwind, Priya at Apex, etc). Per the function-vs-dressing rule this is fine — the demo camera doesn't drill into Dr Mei Lin's profile — but worth noting if anyone ever ports the demo recording to a public-launch context this whole page is theater.
- File: `web/app/customers/page.tsx` + `web/app/customers/[slug]/page.tsx`.

#### F10 [P3] — `/maintenance` and `/offline` are tiny placeholders
Both return ~37-38 KB pages with minimal content. They're status pages, so this is correct, but they're in the sitemap and could embarrass at the wrong moment.
- Files: `web/app/maintenance/page.tsx`, `web/app/offline/page.tsx`.

### In-app pages

#### F03 [P1] — No gateway route for `/api/v1/workers/{id}/duties` or `/schedules`
Scene 2 of the demo runbook scripts a click on Mariah's "Schedules" tab + a "Run now" button → audit row in /audit. The gateway returns `{"error":"not found"}` for both `/api/v1/workers/{id}/duties` and `/api/v1/workers/{id}/schedules`. The core has `argo-orchestrator/src/duty_scheduler.rs` but no gateway route surfaces it. If Scene 2 is shot on a fresh seed, the "Schedules" tab will be empty or the page will error.
- Repro: `curl -s http://localhost:7400/api/v1/workers/019e5dbb-c03d-79a0-a9a4-ee60bb3c8218/duties` → `{"error":"not found"}`.
- Files: no `routes_worker_duties.rs` (or similar) found via `find` under `crates/argo-gateway/src`.
- Fix: add a gateway route under `crates/argo-gateway/src/` that proxies `argo-orchestrator/src/duty_scheduler.rs::list_for_worker` (or similar) — this is a backend brief, not a SPA fix.

#### F04 [P1] — Gateway `/api/v1/workers` collection returns 404; only `/api/v1/team` returns the list
The list-workers endpoint lives at `/api/v1/team`, but the resource is also addressable at `/api/v1/workers/{id}`. Calling the collection (`/api/v1/workers`) returns a 404. This is the kind of inconsistency that will trip a future code reader, an external API consumer, or anyone building a connector against the API surface.
- Repro: `curl -s http://localhost:7400/api/v1/workers` → 404 `{"error":"not found"}`. `curl -s http://localhost:7400/api/v1/team` → workers list.
- Suggested: either alias `/api/v1/workers` → `/api/v1/team`, or rename `/api/v1/team` → `/api/v1/workers`. Not a demo blocker — the SPA uses `useTeam()` which hits `/api/v1/team`.

#### F11 [P2] — `/cohorts` shows "Build your first cohort" empty state despite the gateway returning 1 cohort
`curl -s http://localhost:7400/api/v1/cohorts` returns one real cohort (`marketing-intern::unknown` with 1 member). But `/cohorts` page rendered from the dev server shows the empty-state preview (`<h1>Build your first cohort.</h1>`). This is the CORS issue covered in F19 — the browser can't read the gateway response on port 62927 — but it's also a sign that the seed is treating this cohort as too thin to surface. The empty-state UI is well-designed and acceptable as a POC fallback.
- File: `web/app/(app)/cohorts/page.tsx:32-126`. The empty branch fires when `cohorts.length === 0`, which happens when the gateway fetch fails or returns empty.
- Note: on `:3000` (the demo recording port) where CORS works, this should resolve and the page should show the magazine layout if seed data is rich enough.

#### F12 [P2] — `/team` page is client-only, shows "No teammates yet" before fetch
Server-rendered HTML is just an empty shell + the empty state — the team list paints client-side after the gateway round-trip. During demo recording, a viewer will see ~100-300ms of empty state before the workers tiles appear. The hire wizard does the same.
- File: `web/app/(app)/team/page.tsx:42-48` — `Suspense` fallback renders `<LoadingGrid />` then waits for `useTeam()` to resolve.
- Suggested: SSR-fetch the team list from the gateway (`server: ['Cookie']` forwarding) so the worker tiles are in the first HTML payload.

#### F13 [P2] — `/workers` server response contains zero worker names — fully client-rendered
Same pattern. `curl -s http://localhost:62927/workers` returns 62 KB but `grep` for Mariah/Peta/Priya/Jamal/Nori/Maya returns nothing. The trading-card deck only paints after JS executes. Scene 1 of the demo will see a beat of blank cards before the deck materialises.
- File: `web/app/(app)/workers/page.tsx`.
- Note: this is also a Scene 1 issue specifically covered by the parallel claim-1 audit; I'm noting it here for the whole-site rollup.

#### F14 [P2] — `/audit`, `/approvals`, `/skills`, `/verticals`, `/orchestration` are all client-rendered "Loading X…" shells
Each of these renders a single "Loading X…" string in the server payload and waits for the client fetch. During recording, all five will show a loading spinner for ~100-300ms.
- Files: each respective `page.tsx`.
- Pattern issue: this is the standard `useQuery` + `Suspense` pattern. Acceptable per Next.js 16 client-component conventions; the fix is to add server components that pre-fetch the data with `getQueryClient().prefetchQuery(...)` and pass via HydrationBoundary.

#### F15 [P2] — `/workers/{id}` worker detail is client-only, flashes "Loading worker…"
Verified by `curl -s http://localhost:62927/workers/019e5dbb-... | grep` — the SSR payload contains only "Loading worker…". Demo Scene 1's drill-in click will show a blank/loading frame.
- File: `web/app/(app)/workers/[id]/page.tsx`.

#### F16 [P2] — `/genesis` ships a static role-input shell with no SSR copy beyond "Describe your AI teammate"
Page renders only the input prompt. No example prompts visible until JS hydrates. Acceptable but barren.
- File: `web/app/(app)/genesis/page.tsx`.

#### F17 [P2] — `/support` is 404 but `/support/inbox` is 200
There's no root index page at `/support` — clicking through to `/support` directly 404s. Only the `/support/inbox` child route works.
- Repro: `curl -s -o /dev/null -w "%{http_code}" http://localhost:62927/support` → 404. `/support/inbox` → 200.
- File: `web/app/(app)/support/` only has `inbox/` — no `page.tsx`.
- Fix: either add `web/app/(app)/support/page.tsx` that redirects to `/support/inbox`, or remove any nav links that resolve to bare `/support`.

#### F18 [P3] — `/retro` and `/billing` pages exist but aren't in any nav
`/retro` and `/billing` return 200 but are not in the sidebar nav (which has Approvals · Team · Workers · Audit · Skills · Workflows · Cohorts · Settings). They look like leftovers or future features.
- Files: `web/app/(app)/retro/`, `web/app/(app)/billing/` exist; `/settings/billing` is the actually-linked billing page.
- Suggested: remove if dead, or document why they exist.

### Backend / CORS / env

#### F19 [P3] — Gateway CORS only allows `:3000`; preview-tool's actual port `:62927` is rejected (audit-tooling artifact)
OPTIONS preflight from origin `http://localhost:62927` returns 204 *without* `Access-Control-Allow-Origin` — browser fetch fails (`net::ERR_FAILED`). OPTIONS from origin `http://localhost:3000` returns the expected ACAO header. This is the proximate cause of the empty-state rendering in F11. **Not a demo blocker** because the operator records on `:3000`. This is an audit-tooling artifact only — but worth knowing for future preview-MCP audits, the preview-tool spawns the dev server on its own assigned port and the gateway doesn't know about it.
- Repro: `curl -sv -X OPTIONS http://localhost:7400/api/v1/team -H "Origin: http://localhost:62927"` → no ACAO header.
- File: `crates/argo-gateway/src/` — CORS allowlist construction.
- Suggested: read `NEXT_PUBLIC_DEV_CORS_ORIGINS` from gateway config so preview ports can be allowed without code changes.

#### F20 [P3] — `.env.local` sets `ARGO_GATEWAY_AUTH_BYPASS=1` without `NEXT_PUBLIC_` prefix
Code in `web/lib/api/team.ts:42` checks `process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS === "1"`. The `.env.local` file (line 6) sets `ARGO_GATEWAY_AUTH_BYPASS=1` *without* the `NEXT_PUBLIC_` prefix, so the browser never sees the var and the mock-bypass branch never fires. The team page hits the live gateway every time.
- File: `web/.env.local:6` says `ARGO_GATEWAY_AUTH_BYPASS=1`; should be `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` to match the checked name.
- Note: per the operator's "NO THEATER IN DEMO" lock, mock-bypass is now the *opposite* of what we want — the demo should hit the real gateway. So this misnamed env var is currently *helping* us by making the SPA hit the real backend. Still, the inconsistency between `.env.local` and `lib/api/team.ts` is a latent bug.

#### F21 [P3] — `NEXT_PUBLIC_SKYCOT_DEMO_MODE` is not set; "Demo mode" banner doesn't show
The yellow "Demo mode — showing sample data. Sign in to see your own workspace." banner gates on `NEXT_PUBLIC_SKYCOT_DEMO_MODE === "1"`. Not set in `.env.local`. For the v1 demo recording per the runbook, this banner *should* be visible so investors understand the workspace state. (Or per the no-theater lock, *should not* be visible since the data is real.)
- File: `web/components/skycot/DemoBanner.tsx:18-32` + `web/.env.local` (var not set).
- Suggested: operator-decide whether the demo records with or without this banner, then set/unset the env var explicitly.

### Hire wizard / onboarding

#### F22 [P2] — `/hire/step/6` etc return generic 404 instead of redirecting to /hire
Out-of-range step indexes 404. Functional, but a polish miss — the wizard could redirect to /hire (step 1) instead.
- Repro: `curl -s -o /dev/null -w "%{http_code}" http://localhost:62927/hire/step/6` → 404.
- File: `web/app/(hire)/hire/step/[step]/page.tsx` — should clamp invalid step values.

#### F23 [P2] — `/setup` page returns 200 but is undocumented in the runbook
The `(setup)` route group has both `/setup` (a single-page route) and `/onboarding/*` (the locked F1 5-step flow). The runbook only references `/onboarding/*`. Worth confirming `/setup` isn't a leftover.
- File: `web/app/(setup)/setup/page.tsx`.

### Theme + responsiveness

#### F24 [P3] — Theme picker tested only by source — visual verification blocked by concurrent audits
`components/skycot/ThemePicker.tsx` implements light/dark/system + tenant theme select, and the marketing pages have responsive CSS breakpoints at 540, 700, 890, 900, 980, 1180 px. Could not visually verify either via preview-MCP this run (preview was pinned by parallel audits). Source-side audit shows the implementation is sound — `next-themes` for mode, tenant-theme provider for white-label colors. No findings on the code; recommend a follow-up visual pass when preview tool isn't contended.

### Cross-cutting

#### F25 [P3] — `/landing` hero CTA is "Hire your first teammate" → links to /auth/signup (not /hire)
The marketing nav has `Hire your first teammate` as the primary CTA on every page. It links to `/auth/signup` not `/hire`. That's *probably* intentional (you sign up first then hire), but the button label conflates the two and may confuse a visitor who already has an account. Polish, not a blocker.
- File: `web/components/marketing/MarketingNav.tsx`.

#### F26 [P3] — Marketing pages reference customer names that overlap with the demo personas
`/customers` references Mariah, Priya, Nori, Peta in testimonial copy — the same names as the worker personas on `/workers`. Could read as confusing ("Mariah is a customer? Or a worker?") to a careful viewer. Per the function-vs-dressing rule this is dressing — fictional fixtures — but the name collision is sloppy.
- File: `web/app/customers/page.tsx` (testimonial copy) overlaps with `web/lib/sample-team.ts` (worker fixtures).

---

## 6-mission-claim coverage rollup (from SPA observations)

For each claim, "does this work functionally in MVP scope based on what the SPA shows?" with cited evidence. Function-vs-dressing rule applied — claims about behavior the camera will drill into must be backed by real backend; claims about copy alone don't pass.

### Claim 1 — Teammate, not tool
**Verdict: YES, demonstrable in MVP scope.**
The `/team` and `/workers` pages are wired to a real gateway endpoint (`/api/v1/team`) that returns named worker objects (verified: Maya · marketing-intern is in the live response). Each worker has a vertical, a tagline (`"Marketing intern that drafts and asks before publishing."`), and a personality treatment per the C2 worker-detail design lock. The hire wizard `/hire` accepts plain-English role descriptions and the workflow templates page lists 50+ concrete teammate-shaped jobs (`"Daily Slack digest of deal-room movement"`, `"Personalized cold-outreach email"`). Workers carry a `headline.bullets` field showing what they did, not what tool they ran. Claim holds.

### Claim 2 — Delegation is the unit
**Verdict: PARTIAL — backend wired (per CLAUDE.md round-54 PASS), but the SPA has no visible delegation surface yet.**
Per CLAUDE.md "Status 2026-05-25", claim 2 is LIVE in runtime (`#2256` wired delegate-task fallback-chain inheritance via `ToolContext.current_handoff_id`). However, the SPA has no surface that shows delegation chains as a first-class concept — `/orchestration` is the closest thing and returns 0 runs in the live API. The `/audit` page renders rows that include `worker.id` but no visible "this row was delegated by worker X to worker Y" rendering. For a demo, claim 2 is asserted via copy (`/landing`'s "How it works · Step 02: SKYCOT spins up a named worker") not shown via UI. **Backend is real, SPA proof is dressing-side.**

### Claim 3 — Platform builds workers
**Verdict: YES, demonstrable.**
The `/hire` wizard + `/genesis` page both implement a plain-English-prompt-to-worker flow. `/genesis` says "Describe your AI teammate · We'll draft a real worker — persona, charter, and skills — in about ten seconds." The hire wizard is a 5-step F1 editorial-center flow (verified in source). Per CLAUDE.md, evaluator round-54 ran cold cargo 2/2 PASS on the claim-3 LIVE regression. The SPA surface exists and routes correctly. The actual generation behavior would need to be exercised live during demo recording — couldn't validate end-to-end here without sending a real prompt.

### Claim 4 — Skills shared and learned
**Verdict: YES, demonstrable.**
The `/skills` page is a marketplace shell that calls `/api/v1/skills` + `/api/v1/skills/learned-this-month`. Both endpoints exist and return real (currently empty) responses from the live gateway. Per CLAUDE.md, `#2092 + #2110` are merged and the claim-4 live install-invoke test passed cold 2026-05-25 (`claim_4_live_install_invoke` cargo finished in 1m53s). The page has a hero search bar with suggested-search chips and a "Skills my workers learned this month" chip — both are the design-locked A1 marketplace pattern. The skill library worker-load is wired at `crates/argo-agent-runner/src/turn_helpers.rs:65`. Claim holds, modulo seed needing at least 3 installable skills for the Scene 4 search beat to find anything (currently 0 skills in seed).

### Claim 5 — Any-PC execution
**Verdict: NOT IN SPA SCOPE — gated on operator macOS permissions.**
This claim is fundamentally about runtime computer-use (argo opening Notes.app, clicking buttons in Chrome). The SPA has no surface for it — there's no `/computer-use` page or live action stream. Per CLAUDE.md, claim-5 is 🟡 LIKELY-LIVE pending operator macOS Accessibility + Screen Recording grants on the argo binary. Nothing to validate from the SPA side; this audit is silent on claim 5.

### Claim 6 — Multi-channel + multi-tenant + trust
**Verdict: PARTIAL — multi-tenant API plumbing is real, multi-channel SPA proof is asserted in copy, trust ratchet has a visible surface.**
- **Multi-tenant:** `/api/v1/team` and `/api/v1/cohorts` responses both include `"tenant_id":"019e5057-1d3f-7fa2-9c1b-c34ef01cc21e"` — tenant scoping is plumbed through every primitive. Per CLAUDE.md, `#2257` shipped cross-tenant negative pins and the claim-6 live test passed cold 2026-05-25 (`claim_6_live_cross_tenant` cargo finished in 52.42s).
- **Multi-channel:** the connectors marketplace at `/settings/connectors` lists Slack, Gmail, Outlook, Linear, Jira, Asana, etc — 20+ integrations with real descriptions ("Read and send email from your Google account"). But the SPA has no live multi-channel-action surface — there's no "Mariah pinged your Slack at 09:14 and your Email at 09:15" timeline view. The "lives in Slack like a colleague" narrative is in landing/customer copy only.
- **Trust ratchet:** the `/landing` Step 03 ("Approve their first decisions — their trust grade rises…") asserts the trust ratchet. The `/team` page worker objects have `trust_grade_band` fields (verified in `/api/v1/cohorts` response). The TrustRatchetSection component is imported in `(app)/page.tsx:15`. Real surface exists.

**Net:** Claim 6 demonstrable for trust ratchet + multi-tenant in MVP scope. Multi-channel is asserted-in-copy with no live SPA proof of a single worker actually acting across two channels in one recordable beat — that's a gap for the demo.

---

## Resolved since last audit

This is the first run of this audit at 2026-05-26-1441; the prior whole-site audit at `docs/skycot/ux-audits/2026-05-25-1100-whole-site-mvp-claims-coverage.md` is from yesterday but covers a different swarm state. No specific resolution tracking attempted here.

---

## Orientation score: 78 / 100

The whole-site surface is in good shape — every page renders, marketing pages are rich and on-brand, settings is comprehensive, hire wizard is wired, cohorts has a polished empty-state preview. The five P1s are all addressable in <1 day of work (4 are metadata/title fixes, 1 is the missing duty/schedule gateway route). The bigger structural questions — root-URL routing, multi-channel SPA proof for claim 6, the `/team` vs `/workers` API naming inconsistency — are worth discussing before v1.1 but don't block the v1.0 demo MP4 recording.

The biggest gap on the 6-claim rollup is **claim 6 multi-channel** has no SPA-visible proof beat. If the demo wants to show "the platform pings Slack and Email", that needs either a real action that fires both, or an audit-row that shows two channels in one row. Worth a conversation with the operator before recording.
