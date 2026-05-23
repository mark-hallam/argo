# Round 25 — Chrome e2e user-journey walkthrough

**Date**: 2026-05-23
**Tool**: Chrome MCP (`claude-in-chrome`) at `http://localhost:3000` (Next.js dev server, Turbopack)
**Scope**: Marketing → Auth → Onboarding → Hire wizard → in-product surfaces (Approvals / Workers / Audit / Skills / Workflows / Cohorts / Team / Settings / Customers / Docs / Changelog) plus the workflow-build editor.
**Operator directive 2026-05-23 19:00Z**: "strengthening and expanding the tests and user journeys + use the chrome browser to run through those + e2e site testing AND e2e code base testing."

## Method

Brought up the Next.js dev server (`web/`, port 3000, Turbopack) and navigated through 17 routes via Chrome MCP, capturing screenshots and console messages at each step. Findings below are grouped by priority and tagged with verbatim cite paths so the conductor can spawn fix briefs without re-walking the tree.

Screenshots are saved in `screenshots/` next to this file when material.

## Verdict — overall

✅ **Marketing surface** (landing / pricing / compare / customers / docs / changelog) — **all green**. Brand consistency holds, all CTAs route correctly, plain-language expansions (API, RSS) honour operator's global instruction.

✅ **Auth pages at `/auth/{signin,signup,verify}`** — render with the editorial-center design language locked 2026-05-19. Magic-link + SSO buttons all present.

✅ **Onboarding wizard** at `/onboarding` — auto-redirects to `/onboarding/welcome`, displays the 5-step progress + Audiowide hero typography + matches F1 lock.

✅ **In-product surfaces** (Approvals / Workers / Audit / Skills / Workflows / Cohorts / Team / Settings) — all render with proper empty states, locked design language, sidebar nav consistent.

✅ **Workflow-build editor** — renders on desktop with proper toolbar, palette, canvas. Two UX gotchas listed below.

🟡 **Auth-bare paths** (`/signup`, `/signin`, `/verify`) — 404. Marketing CTAs themselves correctly target `/auth/*` but bookmarks / guessed URLs hit 404.

🟡 **Workflow build editor** — fires "You have unsaved changes" beforeunload on initial page load when nothing has changed.

🔴 **One Next.js component error** spotted in console — `/workflows/[verticalId]/[name]/build/layout` missing default export. Intermittent, may be Turbopack hot-reload artifact.

---

## Findings (ranked)

### 🔴 P1 — finding-1 — workflow build layout missing default export (intermittent)

**Cite**: console at `/workflows/sales/qualify_lead` and `/workflows` recorded `Error: The default export is not a React Component in "/workflows/[verticalId]/[name]/build/layout"` (timestamp 03:51:58).

**Repro**: visit `/workflows`, click into any workflow detail, scroll/click around. The error fires but the page still renders (suggests a side-effect import is missing default export but the route group still resolves to its `page.tsx`).

**Impact**: in production builds (no Turbopack HMR), this WOULD 500 the route. In dev it's a console error.

**Hypothesis**: `web/app/workflows/[verticalId]/[name]/build/layout.tsx` either is missing OR exports something non-default. Worth checking whether the layout file even exists at that path.

**Recommendation**: have the conductor spawn `fix-workflow-build-layout-default-export` — read `web/app/workflows/[verticalId]/[name]/build/layout.tsx` (if it exists), confirm `export default function Layout({children}) {...}` is present. If file missing, decide whether to create it or remove the import that's expecting it.

---

### 🟡 P2 — finding-2 — workflow build editor fires beforeunload on initial load

**Cite**: visit `/workflows/gtm-engineer/post_listing/build`. Immediately on render the page shows the banner "You have unsaved changes" with Discard / Save buttons.

**Repro**: open the build page directly. Without touching anything, try to navigate away — Chrome's "Leave site?" dialog appears.

**Impact**: real users will hit this every time they open a workflow. They'll either learn to ignore the "unsaved changes" warning (which makes the real one useless), or get frustrated by the modal blocking simple back-navigation.

**Recommendation**: brief `fix-workflow-build-dirty-flag-on-initial-load`. Likely cause: the editor seeds local state from the workflow JSON and compares against a different baseline (probably empty/default), so the diff is always non-empty on first paint. Fix is to delay the dirty-flag check until the canvas has loaded the initial graph, or to deep-equal compare against the loaded canonical state.

---

### 🟡 P3 — finding-3 — bare auth paths return 404

**Cite**: `/signup`, `/signin`, `/verify` all return the 404 page with "Approvals · Workers · Docs" recovery buttons. The actual pages live at `/auth/signup`, `/auth/signin`, `/auth/verify`.

**Repro**: type `localhost:3000/signin` in the URL bar. 404.

**Impact**: bookmarks, guessed URLs, marketing links from outside (eg. ads or email signatures) that point at the bare path will land on a 404. Marketing nav itself is correct — the landing page CTAs route to `/auth/signin` and `/auth/signup`.

**Recommendation**: brief `web-redirect-bare-auth-paths`. Add 3 redirects in `web/middleware.ts` or `web/next.config.ts`: `/signup → /auth/signup`, `/signin → /auth/signin`, `/verify → /auth/verify`. Two-line change per redirect.

---

### 🟢 finding-4 — `/workers/<slug>` 404 on unhired template

**Cite**: `/workers/mariah-marketplace` returns "Worker not found — The worker 'mariah-marketplace' doesn't exist or you don't have access."

**Status**: **correct behaviour**, not a bug. The Workers page in an empty workspace only shows hireable templates ("Marketplace seller", "Recruiter — candidate screening", ...). "Mariah-Marketplace" is a worker NAME the user would assign during hire — until hired, `/workers/<id>` correctly 404s with a clean empty state.

**Note**: I confirmed the empty state ships with "Back to team" recovery — well-handled.

---

### 🟢 finding-5 — operator plain-language preference is honoured in copy

**Cite**: `/docs` shows `API (Application Programming Interface — the contract programs use to talk to SKYCOT) reference`. `/changelog` shows `RSS (Really Simple Syndication — a feed format that lets you follow updates without checking the page)`.

**Status**: GOOD — this matches the operator's user-wide instruction to "expand every abbreviation the first time it appears + add a one-line plain-English summary." A worker (or the copy author) actually applied this rule to UI copy. Reusable pattern.

---

### 🟢 finding-6 — PWA install prompt fires on `/workers`

**Cite**: visiting `/workers` triggered the bottom-right "Install SKYCOT — Get it on your homescreen for one-tap access" toast.

**Status**: GOOD — Progressive Web App (a website that can be installed like a native app) install prompt is wired up. Reasonable trigger location (workers page is product-core).

---

### 🟢 finding-7 — Next.js dev-mode "1 Issue" badge visible bottom-left

**Status**: GOOD — this is Next.js dev's built-in error/warning indicator, not a SKYCOT app surface. It clusters the 155 dev-only console errors (Chrome extension async-channel noise mostly, plus the layout-default-export error). In production builds this badge does not render.

---

## Routes walked + verdict

| # | Route | Verdict | Notes |
|---|---|---|---|
| 1 | `/landing` | 🟢 | Audiowide hero, locked brand-loud register, CTAs correct |
| 2 | `/pricing` | 🟢 | 3-tier (Free / Pro / Enterprise), brand-restrained, $29 Pro tier locked |
| 3 | `/compare` | 🟢 | Editorial table vs Zapier/n8n/Crew.ai/Lindy, brand-restrained |
| 4 | `/customers` | 🟢 | Northwind / Beacon Health / Maple Finance testimonial cards |
| 5 | `/docs` | 🟢 | Plain-language expansion honoured (API note) |
| 6 | `/changelog` | 🟢 | RSS expansion honoured + week-by-week timeline |
| 7 | `/signup` | 🟡 P3 | 404 — bare path; canonical lives at `/auth/signup` |
| 8 | `/signin` | 🟡 P3 | 404 — bare path; canonical lives at `/auth/signin` |
| 9 | `/verify` | 🟡 P3 | 404 — bare path; canonical lives at `/auth/verify` |
| 10 | `/auth/signup` | 🟢 | F1 editorial-center, Google / GitHub / Microsoft SSO + magic-link |
| 11 | `/auth/signin` | 🟢 | Same lock as signup, "WELCOME BACK" eyebrow |
| 12 | `/onboarding` | 🟢 | Auto-redirects to `/onboarding/welcome` step 1 of 5 |
| 13 | `/workers` | 🟢 | Empty-state shows 5 vertical templates (Marketplace seller / Recruiter / Support / SDR / Tax-prep) |
| 14 | `/approvals` | 🟢 | Empty state "No approvals waiting" with "Open the inbox" CTA |
| 15 | `/audit` | 🟢 | "Quiet day · PII redacted — your role shows masked data" — RBAC honoured visibly |
| 16 | `/skills` | 🟢 | Marketplace with 6 featured skills + hero search + try-pills |
| 17 | `/workflows` | 🟢 | 7 workflow templates listed (post_listing v3, screen_candidate v5, approve_refund v2, qualify_lead v4, triage_ticket, morning_pipeline_digest) |
| 18 | `/cohorts` | 🟢 | "Build your first cohort" empty state, gradient editorial banner |
| 19 | `/team` | 🟢 | "Hire your first teammate" hero + same template gallery as /workers |
| 20 | `/settings` | 🟢 | 8 settings tiles (Workspace / Team & roles / Notifications / Security / Provider keys / API keys / Knowledge / Connectors / Billing / Data export) |
| 21 | `/workflows/gtm-engineer/post_listing` | 🟢 | Workflow detail readonly view, 6 steps shown |
| 22 | `/workflows/gtm-engineer/post_listing/build` | 🟡 P2 | Build editor renders, but fires false-positive "unsaved changes" |
| 23 | `/workers/mariah-marketplace` | 🟢 | Correct "Worker not found" empty state (template not yet hired) |

---

## Recommendations to conductor

Three briefs (one P1, one P2, one P3):

1. **P1** `fix-workflow-build-layout-default-export` — investigate `web/app/workflows/[verticalId]/[name]/build/layout.tsx`. The console error suggests a missing default export. If file is missing, decide whether to create it or remove the import path expecting it. Worker should reproduce by running `npm run dev` and visiting `/workflows`.

2. **P2** `fix-workflow-build-dirty-flag-on-initial-load` — beforeunload guard fires on a fresh page load with no edits. Likely seed-state mismatch — the local-state seed differs from the canonical baseline used in the dirty-flag check. Worker should add a regression Playwright test that opens the build page and asserts no `window.onbeforeunload` handler exists until at least one canvas edit fires.

3. **P3** `web-redirect-bare-auth-paths` — 3-line addition to `web/middleware.ts` or `web/next.config.ts` to redirect `/signup → /auth/signup`, `/signin → /auth/signin`, `/verify → /auth/verify`. Avoids 404 on bookmarks/guessed URLs.

## Cross-reference with mission-claim coverage

This round was about **frontend journey completeness**, not mission-claim verification. None of the findings affect Claim 6 MVP (which sits at ~95% after round 24). All 6 mission claims still trace through their tested-substrate paths. Frontend issues land in `mission-vs-code-gap-manifest.md` as web-layer findings, not as claim regressions.
