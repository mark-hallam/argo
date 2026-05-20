# Wave-2 Responsive Design Audit — SKYCOT demo + marketing surfaces

**Date:** 2026-05-20
**Auditor:** Wave-2 audit agent #3 (responsive design)
**Method:** Static analysis of `web/app/**` route files + supporting `web/components/skycot/**` primitives. Tailwind responsive prefix coverage (`sm:`/`md:`/`lg:`/`xl:`), inline-CSS `@media (max-width)` queries, hard-coded widths, and React Flow / table layout fallbacks. No runtime browser DevTools used.
**Scope:** 12 demo + marketing surfaces × 4 breakpoints.

## Breakpoint contract

| Tier | Width | Real device |
|---|---|---|
| Mobile portrait | **375px** | iPhone SE / iPhone 13 mini portrait |
| Tablet portrait | **768px** | iPad portrait |
| Small desktop | **1024px** | iPad landscape / small laptop |
| Full desktop | **1440px+** | MacBook 14"+, default design canvas |

## Legend

- POLISHED — looks intentional; brand-loud register holds; nothing collides, truncates badly, or overflows
- WORKS but ugly — readable + usable but visibly cramped, font too big, KPI strip wraps unevenly, etc.
- BROKEN — horizontal scroll on the body, off-screen content, hit-targets <40px, illegible at zoom, nav unreachable, or a control panel covers the canvas

---

## Page-by-page matrix

| Page | 375px | 768px | 1024px | 1440px | Notes |
|---|---|---|---|---|---|
| `/landing` | WORKS but ugly | POLISHED | POLISHED | POLISHED | Nav links hidden <900px with NO hamburger replacement → sign-in + signup CTAs remain visible. Hero 1.1fr+1fr collapses to 1fr at <900px. Trust-strip JetBrains-mono pills wrap to 4-5 rows on 375px. |
| `/compare` | **BROKEN** | **BROKEN** | WORKS but ugly | POLISHED | **5-column competitor + 14-row dimension `<table>` has no horizontal-scroll wrapper.** At 375px the table either overflows the viewport (causing the entire page to horizontal-scroll) or visibly clips. NO header nav. NO marketing chrome (just `.compare-stage`). Cannot return to landing without typing URL. |
| `/pricing` | WORKS but ugly | WORKS but ugly | POLISHED | POLISHED | Tier grid 3→1 columns at <900px is good. **Feature matrix `<table>` (4 cols × 12 rows) has no horizontal-scroll wrapper.** Nav links hidden <700px with NO hamburger. 60px Audiowide h1 → 36px is good. |
| `/customers` | POLISHED | POLISHED | POLISHED | POLISHED | Grid 3→2→1 at 980/640. Hero `cus-h1` 60→36px. KPI strip inside card stays 3-col grid at all widths — could be tight at 375px but readable. Nav links hidden <700px, no hamburger. |
| `/` (Approvals) | WORKS but ugly | POLISHED | POLISHED | POLISHED | `BrandLoudHero` has `@media (max-width: 720px)` polish. `AppShell` swaps sidebar for proper mobile drawer (hamburger top-left). Header sticky bar with filter + bell + hire button — at 375px the "Hire teammate" button text hides (`hidden sm:inline`) keeping just `+` icon. PageHeader filter chips wrap clean. **Issue:** `mx-auto max-w-3xl` containers + 16px padding leave only ~343px content width on 375px viewport — bundle cards + day rows are tight but workable. |
| `/team` | WORKS but ugly | POLISHED | POLISHED | POLISHED | `TeammateGrid` uses `grid-cols-1 sm:grid-cols-2 lg:grid-cols-3` — correct. Hero KPIs reflow well. Status filter chip rail wraps. |
| `/workers/[id]/overview` | WORKS but ugly | POLISHED | POLISHED | POLISHED | `WorkerLayoutShell` PageFrame is `max-w-4xl` with sm/lg padding. `WorkerPersonalityCard` has explicit `@media (max-width: 700px)` collapsing 1fr 1fr → 1fr. `WorkerTabs` has `overflow-x-auto` on the tablist — long names will scroll. |
| `/workflows` | POLISHED | POLISHED | POLISHED | POLISHED | Grid 2→1 at 760px. Cards compact. |
| `/workflows/[v]/[n]/build` | **BROKEN** | **BROKEN** | WORKS but ugly | POLISHED | **React Flow canvas + 360px-fixed side panel + 28px-padded top bar with NO `@media` queries anywhere in the file.** At 375px: canvas is unusable (nodes 280px wide), side panel covers half the screen, top bar wraps with overflow. At 768px: side panel still 360px wide leaving 408px canvas — drag-pan barely works with touch. **This page is desktop-only with zero fallback or gate.** |
| `/audit` | POLISHED | POLISHED | POLISHED | POLISHED | `max-w-5xl px-4 sm:px-6 lg:px-8`. `AuditPageHeader` "Export" label hides at <sm. `ActivityRow` uses `grid-cols-[auto_auto_1fr_auto_auto]` with `truncate` on verb — survives 375px. |
| `/skills` | WORKS but ugly | POLISHED | POLISHED | POLISHED | Featured grid 3→2→1 at 900/600. **Hero `skills-hero` 48px Audiowide has NO mobile breakpoint** → wraps to 3 lines on 375px, looks too dominant. Search row stays 1-line via `flex: 1` on the input. |
| `/hire` (wizard) | POLISHED | POLISHED | POLISHED | POLISHED | `WizardEditorialShell` has explicit `@media (max-width: 800px)` block dropping hero font 68→44px, tightening top bar to 3-col auto/1fr/auto, dropping pip width. The 5 step pages render inside the shell's `.wiz-body` (max-width 720px) so their cards inherit. |
| `/onboarding/*` | POLISHED | POLISHED | POLISHED | POLISHED | Layout is `max-w-3xl px-4 sm:px-6 lg:px-8` with Tailwind responsive class throughout. `pick-vertical` grid is `grid-cols-1 sm:grid-cols-2`. ProgressDots compact. |

---

## Top 5 BROKEN combinations (priority order)

1. **`/workflows/[v]/[n]/build` @ 375px (mobile portrait)**
   React Flow canvas + 360px-fixed side panel + no media queries. The `.builder-shell` is `height: calc(100vh)` flex column; `.builder-stage` is flex with a 360px hard-coded panel and a flex-1 canvas. On 375px the panel eats 96% of screen width when open. Nodes are 280px wide; React Flow's touch-pan gesture conflicts with native scroll. **Fix path:** add a "Switch to desktop to edit" gate that renders a read-only step list at <1024px with a deeplink CTA.

2. **`/workflows/[v]/[n]/build` @ 768px (tablet portrait)**
   Same root cause. 360px panel + canvas = workable but constrained; touch-pan on React Flow on iPad is acceptable but the side panel is too wide. **Fix path:** convert side panel to a bottom-sheet under 1024px (radix Dialog like `NavDrawer` already uses).

3. **`/compare` @ 375px**
   The competitive-positioning `<table>` is the centerpiece of the page. 5 competitor columns (each declared as 14% via implied equal distribution) × 14 dimension rows. No `overflow-x:auto` wrapper, no `min-width` on table, no horizontal scroll affordance. At 375px the table either causes the entire body to horizontal-scroll (worst) or text wraps to unreadable thin columns. **And** there's no header nav, so the user is stuck without keyboard shortcuts. **Fix path:** wrap `.compare-table-wrap` with `overflow-x:auto; -webkit-overflow-scrolling:touch;` and a `min-width: 720px` on the table. Add the same `.land-top` header from `/landing` so users can navigate.

4. **`/compare` @ 768px**
   Same table issue. iPad portrait is 768px wide; the table needs ~720-820px to read cleanly. **Fix path:** as above. Once you have `overflow-x:auto` with `min-width: 720px`, 768px renders correctly.

5. **`/pricing` feature-matrix table @ 375-540px**
   Same family of bug. 4-column matrix with `width: 130px` on each non-first cell = 390px minimum just for the score columns + the first column's content. Will overflow. The tier-grid section above works fine (3→1 at 900px) but the matrix scrolls horizontally with no affordance. **Fix path:** wrap the `<table className="pri-matrix">` in `overflow-x:auto` + add a `min-width: 580px`.

---

## Cross-cutting findings

### F-1 — Marketing pages have no mobile navigation
`/landing`, `/customers`, `/pricing` all hide `.land-nav` / `.cus-nav` / `.pri-nav` at <900px (or <700px) with `display: none` and provide **no hamburger / drawer replacement**. The brand mark + sign-in + signup CTAs stay visible, but users on a phone cannot reach `/compare`, `/customers`, `/pricing`, `/landing#how`, `/landing#features` from inside the marketing pages. They have to type URLs.

`/compare` has no header at all.

The `(app)` authed surfaces handle this correctly via `NavDrawer` (hamburger top-left, bottom-sheet drawer). The marketing surfaces are unaware that pattern exists.

### F-2 — Tables without horizontal-scroll wrappers
Three tables fail the same way:
- `/compare` `.compare-table` (5 competitor cols)
- `/pricing` `.pri-matrix` (4 tier cols)
- `/landing` has no table — safe

Standard fix is `<div style="overflow-x:auto;"><table style="min-width:Xpx">…</table></div>`.

### F-3 — Workflow builder is desktop-only with no gate
The single most-broken page. Used in the v1.0 demo on desktop but anyone landing on a tablet or phone gets a non-functional canvas + panel that obscures itself. The competitive talking point ("visual builder that beats n8n") is unreachable on tablet — and a procurement reviewer doing first-look diligence on iPad will see this and bounce.

### F-4 — Audiowide hero fonts at 48-68px need mobile floors
- `landing-hero-h1` 68→44px at 700px (good)
- `cus-h1` 60→36px at 700px (good)
- `pri-h1` 60→36px at 700px (good)
- `compare-hero` 48px **with no mobile floor** (bad — but compare's bigger issue is the table)
- `skills-hero` 48px **with no mobile floor** (bad)
- `wiz-hero` 68→44px at 800px (good)

### F-5 — Trust-strip / KPI strip pills wrap untidily on 375px
On the landing hero, the 5-pill `land-trust-strip` (NO CREDIT CARD · SOC 2 TYPE II · ISO 27001 · GDPR HIPAA AFSL · 2,847 teams) wraps to 4-5 rows on 375px. Cosmetic, not broken — but feels uncurated.

### F-6 — `BundleCard` absolute-positioned ghost
The `BundleCard` uses `absolute inset-x-0 top-0` on a stacked-card ghost element. Confirmed inset-x-0 not fixed-px so it scales. Not a bug.

### F-7 — `ActivityRow` 5-cell grid
`grid-cols-[auto_auto_1fr_auto_auto]` with `truncate` on the verb. Holds together at 375px but is dense. Cost chip + status pill in the third-from-right cell could break to a second line. Acceptable for forensic-density.

---

## Recommended fix order (1-2 days of work)

### Top fix for mobile (375px) — one change that helps the most pages
**Add `<div className="overflow-x-auto -mx-4 px-4">` (or equivalent inline CSS wrapper) around every `<table>` on `/compare`, `/pricing`, and the FAQ matrix.** Add `min-width: 720px` (compare) and `min-width: 580px` (pricing matrix) to the tables themselves. Estimated <40 LOC across two files. Fixes 3 of the top-5 BROKEN combos.

### Top fix for tablet (768px) — one change that helps the most pages
**Convert the workflow builder's right-side panel to a bottom-sheet at <1024px.** Use the same Radix Dialog pattern that `NavDrawer` already implements. The canvas keeps full width; the panel slides up from below when a node is clicked. Estimated ~80 LOC in `build/page.tsx`. Fixes the workflow-builder-at-tablet broken combo and makes the page usable for iPad demos.

### Pages that should be desktop-only with a "view this on desktop" gate
**Only `/workflows/[v]/[n]/build`.** Add a `<MobileGate>` wrapper that renders a step-list summary + "Open the builder on desktop to edit" CTA at <1024px. The read-only viewer at `/workflows/<v>/<n>` (linked from the builder back-button) already works on mobile.

Every other surface should fit at every breakpoint. The marketing pages need hamburger nav and table wrappers, not a "desktop only" gate.

---

## Overall responsive score

**5.5 / 10**

- The authed surfaces (`AppShell` + `NavDrawer` + `BrandLoudHero` + grid components) are correctly mobile-first and demonstrate the team knows how to build responsive layouts.
- The marketing surfaces (`/landing`, `/customers`, `/pricing`, `/compare`) are 60-70% there: clear `@media (max-width: 900px)` breakpoints for hero columns and feature grids, BUT they lack mobile navigation entirely and have unwrapped wide tables that overflow.
- The workflow builder is a 1/10 on mobile/tablet — totally broken.

Subtract 2 points for the broken builder (it's the strategic differentiator), 1.5 points for the table overflows on `/compare` and `/pricing` (it's the conversion path), and 1 point for marketing-page missing hamburger nav. Add 0 bonus points — the authed surfaces are at acceptable baseline, not exceeding.

**With the two recommended fixes above (~120 LOC total), this would jump to 8/10.**

---

## File references

| Surface | Primary file | Responsive style live in |
|---|---|---|
| Landing | `web/app/landing/page.tsx` | inline `<style jsx global>` (`@media (max-width: 900px)` + `max-width: 700px`) |
| Compare | `web/app/compare/page.tsx` | inline `<style jsx global>` (only ONE `@media (max-width: 980px)`) |
| Pricing | `web/app/pricing/page.tsx` | inline `<style jsx global>` (`@media (max-width: 900px)` + `max-width: 700px`) |
| Customers | `web/app/customers/page.tsx` | inline `<style jsx global>` (980/640/700) |
| Approvals | `web/app/(app)/page.tsx` + `web/components/skycot/BrandLoudHero.tsx` | Tailwind responsive classes + BrandLoudHero `@media (max-width: 720px)` |
| Team | `web/app/(app)/team/page.tsx` + `web/components/skycot/TeammateGrid.tsx` | Tailwind `grid-cols-1 sm:grid-cols-2 lg:grid-cols-3` |
| Worker overview | `web/app/(app)/workers/[workerId]/WorkerLayoutShell.tsx` + `web/components/skycot/worker/WorkerPersonalityCard.tsx` | `max-w-4xl px-4 sm:px-6 lg:px-8` + `@media (max-width: 700px)` |
| Workflows index | `web/app/(app)/workflows/page.tsx` | inline `<style jsx>` `@media (max-width: 760px)` |
| Workflow builder | `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx` | **NONE — desktop-only with no `@media` queries** |
| Audit | `web/app/(app)/audit/page.tsx` + `web/components/skycot/audit/AuditPageHeader.tsx` + `web/components/skycot/ActivityRow.tsx` | `max-w-5xl px-4 sm:px-6 lg:px-8` |
| Skills | `web/app/(app)/skills/page.tsx` | inline `<style jsx>` `@media (max-width: 900px)` + `max-width: 600px` |
| Hire wizard | `web/app/(hire)/hire/page.tsx` + `web/components/auth/WizardEditorialShell.tsx` | `@media (max-width: 800px)` |
| Onboarding | `web/app/(setup)/onboarding/layout.tsx` + each step page | Tailwind `max-w-3xl px-4 sm:px-6 lg:px-8` + `grid-cols-1 sm:grid-cols-2` |
| App shell mobile drawer | `web/components/ui/NavDrawer.tsx` | Tailwind `md:hidden` / `hidden md:flex` paired with Radix Dialog |
