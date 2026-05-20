# Wave-9 Window-Resize Stability Audit

**Date**: 2026-05-21
**Auditor**: Wave-9 audit agent #4 (Claude Opus 4.7)
**Scope**: Demo-path pages — smoothness of layout reflow under window
resize (split-screen with Slack, viewport snapping, browser-window drag).

**Pages audited**:
1. `/landing` — `web/app/landing/page.tsx`
2. `/` (Approvals home) — `web/app/(app)/page.tsx`
3. `/team` — `web/app/(app)/team/page.tsx`
4. `/workers/[id]/overview` — `web/app/(app)/workers/[workerId]/overview/page.tsx`
5. `/workflows` — `web/app/(app)/workflows/page.tsx`
6. `/workflows/.../build` (React Flow canvas) — `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx`
7. `/audit` — `web/app/(app)/audit/page.tsx`

**Methodology**:
- `rg "@container|container-type"` — container queries (none found in repo)
- `rg "ResizeObserver"` — only React Flow's internal use
- `rg "@media (max-width|min-width)"` — 22 files use media queries
- `rg "position: absolute|position: fixed"` — 17 files with abs/fixed
- `rg "overflow-x: auto"` — 15 callsites
- Read each page + shared shell components (`AppShell.tsx`,
  `NavDrawer.tsx`, `BrandLoudHero.tsx`, `TeammateGrid.tsx`,
  `ActivityRow.tsx`, `CommandPalette.tsx`).

---

## Headline numbers

- **Pages with smooth reflow**: 5 of 7
- **Pages with snap-jumps**: 2 of 7
- **Worst page**: `/workflows/.../build` (React Flow canvas)
- **Container queries used anywhere**: 0
- **`ResizeObserver` callsites**: 1 (React Flow vendor code, in
  `components/skycot/canvas/SkillCanvas.tsx` mentioned only as the
  reason for a DOM-fallback test path — production uses RF directly)
- **Single global resize listener**: `NavDrawer.tsx:30` closes the
  mobile drawer when `window.innerWidth >= 768`. Clean.

---

## Per-page summary

### 1. `/landing` — **MODERATE snap-jump**

5-dim score:
1. **Snap on resize**: yes — three hard breakpoints (700/720/900px)
   each flip multi-col grids to 1-col. Mid-range (~750–890px) the
   hero stays in `1.1fr 1fr` two-column form while the hero-art shrinks
   below its 520px max → bubble overlap (see #3).
2. **Container queries / ResizeObserver**: no — pure viewport media
   queries.
3. **Absolute-positioned alignment**: **broken in two places**
   (see "Top 3 absolute-position bugs" below).
4. **Tables / overflow-x**: `.land-social-strip` uses
   `overflow-x: auto` with hidden scrollbar. Safe.
5. **Min-width breakage**: no explicit floor; mobile breakpoint
   (700px) collapses to single column. Renders down to ~360px
   acceptable, below ~320px the hero-h1 (68→44px) starts wrapping
   awkwardly because long words ("teammate") don't break.

### 2. `/` (Approvals home) — **SMOOTH**

5-dim score:
1. **Snap on resize**: no — Tailwind `max-w-3xl` content column +
   `flex-wrap` on header controls. Search input is `w-full`. Filter
   chips wrap. No grid breakpoints — content is single-column lists.
2. **Container queries / ResizeObserver**: no — but doesn't need any
   (single-column layout).
3. **Absolute-positioned alignment**: only sticky header backdrop,
   search-icon left-3 + clear-button right-2 (translate-y-1/2). All
   anchored to parent `.relative`. Safe.
4. **Tables / overflow-x**: bundle rail uses gap-2 cards; no
   horizontal table. Safe.
5. **Min-width breakage**: filter trigger + Hire button collapse
   via `hidden sm:inline` label. Down to ~360px clean.

### 3. `/team` — **MOSTLY SMOOTH (1 minor)**

5-dim score:
1. **Snap on resize**: `TeammateGrid` (`components/skycot/TeammateGrid.tsx`)
   flips at `sm:grid-cols-2 lg:grid-cols-3`. Two snap points (640px,
   1024px). Each snap is a visible reflow but not jarring because
   cards are square-ish.
2. **Container queries / ResizeObserver**: no.
3. **Absolute-positioned alignment**: hamburger menu slot top-right
   of `TeammateCard` is fixed at `top: 14px; right: 14px`. Anchored
   to card, scales with card width. Safe.
4. **Tables / overflow-x**: no tables.
5. **Min-width breakage**: `BrandLoudHero` flex-wrap with
   `min-width: 260px` on the left block + 720px mobile breakpoint
   for KPI tiles. Clean.

### 4. `/workers/[id]/overview` — **SMOOTH**

5-dim score:
1. **Snap on resize**: `WorkerPersonalityCard` uses `grid-template-columns: 1fr 1fr` →
   `1fr` at 700px. `KpiCard` uses `grid-cols-[1fr_auto] sm:grid-cols-[1.6fr_auto_auto_auto]`.
   `TrustGradeCard` uses `grid gap-3 sm:grid-cols-3`. Single 640px snap.
2. **Container queries / ResizeObserver**: no.
3. **Absolute-positioned alignment**: only the 4px gradient bar on
   `WorkerPersonalityCard::before` — pinned to card edges, scales
   correctly. Safe.
4. **Tables / overflow-x**: `WorkerTabs` uses `flex items-center
   gap-1 overflow-x-auto` — tab bar scrolls horizontally when there
   isn't room. Safe.
5. **Min-width breakage**: `max-w-4xl` content column → below
   ~480px text starts feeling cramped but no overflow.

### 5. `/workflows` — **SMOOTH**

5-dim score:
1. **Snap on resize**: `.workflows-grid` flips
   `repeat(2, 1fr)` → `1fr` at 760px. Single snap.
2. **Container queries / ResizeObserver**: no.
3. **Absolute-positioned alignment**: `.workflow-card-arrow` is
   `right: 18px; bottom: 18px` — inside its card, safe.
4. **Tables / overflow-x**: `.workflows-try-row` uses `flex-wrap` —
   try-chips wrap to new rows. Safe.
5. **Min-width breakage**: `max-width: 1100px` outer + 32px padding.
   Below ~340px the hero "actually <strong>think</strong>" 48px
   Audiowide hero starts wrapping a bit. Acceptable.

### 6. `/workflows/[verticalId]/[name]/build` — **WORST: SNAP-JUMP + ABS-POS BUG**

5-dim score:
1. **Snap on resize**: **yes — severe**. `.builder-stage` is
   `display: flex` with a hard 360px-wide `.builder-panel` on the
   right when a node is selected. There is **no media query** that
   collapses the panel to bottom-stacked when the viewport narrows.
   At 1024px viewport (Slack split-screen): sidebar 256px + panel
   360px + React Flow canvas ~408px. The canvas becomes too narrow
   to see two columns of nodes (each node is 280px fixed-width).
2. **Container queries / ResizeObserver**: React Flow ships its own
   ResizeObserver for the canvas, which is the *only* good
   resize-handling on this page. Page chrome doesn't use either.
3. **Absolute-positioned alignment**: **BUG** — `.builder-add-step`
   FAB at `right: 24px; bottom: 24px` of `.builder-canvas`. When the
   panel opens and the canvas shrinks to <500px, the FAB overlaps
   with React Flow's `Controls` (bottom-left) and `MiniMap`
   (bottom-right). The MiniMap is anchored `bottom-right` of the
   ReactFlow viewport, so they collide on top of each other.
4. **Tables / overflow-x**: panel uses `overflow-y: auto`. Canvas is
   pan/zoom (React Flow). Safe in isolation.
5. **Min-width breakage**: header `.builder-top` uses `flex-wrap`
   so it survives, but `.builder-actions` (View timeline | Test run
   | Save+publish) wraps below 800px causing the title to drop. The
   prompt bar with a long placeholder starts truncating at <600px.

### 7. `/audit` — **SMOOTH**

5-dim score:
1. **Snap on resize**: no — `max-w-5xl` single column. Header uses
   `flex-wrap` for Filter/Range/Export controls. `ActivityRow` is
   `grid-cols-[auto_auto_1fr_auto_auto]` with `min-w-0 + truncate`
   on the verb column — squishes gracefully without snapping.
2. **Container queries / ResizeObserver**: no.
3. **Absolute-positioned alignment**: search icon left-3 +
   filter-trigger-dot `-top-0.5 -right-0.5` — small badges anchored
   to their relative parent. Safe.
4. **Tables / overflow-x**: list-of-rows pattern, no table. The
   action detail page (`/audit/action/[id]`) uses `grid gap-4
   lg:grid-cols-2` — 1024px snap from 1-col → 2-col. Single snap.
5. **Min-width breakage**: down to ~360px clean. Export button
   collapses to icon-only via `hidden sm:inline`.

---

## Top 3 absolute-position bugs

### Bug 1 — Landing hero bubbles overlap centre logo
**File**: `web/app/landing/page.tsx:436-498`
**Symptom**: `.land-art-bubble-1..4` use percentage positions
(`top: 16%; left: 8%`, etc.) inside `.land-hero-art` which is
`max-width: 520px; aspect-ratio: 1`. They have hard-coded
`min-width: 220px; max-width: 280px`. When the hero column shrinks
below 520px (viewport ~700–890px), the bubbles' min-width 220px
extends across most of the container, overlapping `.land-art-center`
(88px square at 50%/50%). A comment at line 446-450 acknowledges a
Wave-6 e2e fix already pinned min/max-width, but didn't address the
intermediate-viewport case.
**Fix**: either narrow the bubbles below 900px, or hide them under
the 900px breakpoint, or convert positioning to use the hero-art
width via a CSS custom property + `calc()` from a `ResizeObserver`
on `.land-hero-art`.

### Bug 2 — Build canvas FAB overlaps React Flow Controls + MiniMap
**File**: `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx:572-589`
**Symptom**: `.builder-add-step` is `position: absolute; right: 24px;
bottom: 24px; z-index: 10` of `.builder-canvas`. React Flow's
`<MiniMap position="bottom-right" />` (line 670) is anchored to the
same corner inside the ReactFlow viewport. At viewport <1024px when
the panel is open and canvas shrinks, the "Add step" 110×46px button
sits **on top of** the MiniMap. `<Controls position="bottom-left" />`
(line 661) collides next when the canvas is <300px wide.
**Fix**: move `.builder-add-step` outside `.builder-canvas` and pin
to `.builder-stage` (parent of canvas + panel), or relocate MiniMap
to `top-right`. Cleaner: move the FAB to `bottom: 80px` to clear the
MiniMap's ~150px footprint.

### Bug 3 — Landing top nav fixed gradient blobs don't reposition on resize
**File**: `web/app/landing/page.tsx:153-174`
**Symptom**: `.land::before` and `.land::after` are
`position: fixed` decorative radial-gradients pinned to
`top: -20%; right: -10%` (and `bottom: -30%; left: -10%`) with
fixed 700×700 / 800×800 pixel dimensions. On resize, they don't
reflow — they stay anchored to the viewport corners, which means
they may shift visually when the user resizes (the percentages
recompute against the new viewport), but the 800px square doesn't.
This is a minor visual jank during a window drag — the blob
re-positions abruptly because percentages snap, not glide. It's the
sort of thing that looks wobbly when split-screening with Slack.
**Fix**: low-priority — convert to `vw`/`vh` units so the blobs
scale with the viewport, or `transform: translate(...)` so the
positional anchor moves smoothly.

---

## Additional observations

- **No container queries used anywhere in the codebase.** Modern
  pattern that would be the right answer for the `/workflows/build`
  canvas (let the canvas respond to its own width, not the viewport's).
- **NavDrawer resize handler is sound** (`NavDrawer.tsx:28-34`) —
  one global listener at the only place it's needed.
- **AppShell's `md:pl-64` is the load-bearing breakpoint** (768px).
  Below 768px the sidebar collapses to a hamburger; above it,
  content gets a 256px left offset. This is the single hard snap
  every page inherits — but it's a clean snap because the layout
  fully reflows at that point.
- **All other "snap" breakpoints across the codebase are single-axis
  grid-template flips** (1→2 col or 2→3 col). No double-snap or
  layout-shift compounding.
- **Tailwind `grid-cols-[auto_auto_1fr_auto_auto]` + `min-w-0 +
  truncate`** (in `ActivityRow`) is the cleanest reflow pattern in
  the codebase — gracefully squishes content without any
  breakpoints. Worth propagating to other rows.

---

## Recommendations (in priority order)

1. **Build canvas**: add `@media (max-width: 1100px)` to collapse
   `.builder-panel` from sidebar to bottom-stacked drawer; fix the
   FAB overlap by moving it out of `.builder-canvas`.
2. **Landing bubbles**: hide `.land-art-bubble-*` below 900px (just
   show the centre logo + gradient), or scale them with the hero-art
   container via container queries.
3. **Globally adopt container queries** for any card/panel that has
   internal layout shifts (the worker `WorkerPersonalityCard`
   1-col↔2-col flip is a candidate — it should respond to *card*
   width, not viewport, because the same card lives in different
   contexts).
4. **Landing fixed gradients**: convert pixel dims to `vw` so they
   scale smoothly during a drag.

---

## File inventory

- `web/app/landing/page.tsx` (1174 LOC)
- `web/app/(app)/page.tsx` (563 LOC)
- `web/app/(app)/team/page.tsx` (462 LOC)
- `web/app/(app)/workers/[workerId]/overview/page.tsx` (31 LOC)
- `web/app/(app)/workflows/page.tsx` (602 LOC)
- `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx` (770 LOC)
- `web/app/(app)/audit/page.tsx` (425 LOC)
- `web/components/skycot/AppShell.tsx`
- `web/components/ui/NavDrawer.tsx` (the one resize listener)
- `web/components/skycot/BrandLoudHero.tsx`
- `web/components/skycot/TeammateGrid.tsx`
- `web/components/skycot/TeammateCard.tsx`
- `web/components/skycot/ActivityRow.tsx`
- `web/components/skycot/audit/AuditPageHeader.tsx`
- `web/components/skycot/audit/AuditDayGroup.tsx`
- `web/components/skycot/worker/WorkerPersonalityCard.tsx`
- `web/components/skycot/CommandPalette.tsx`
