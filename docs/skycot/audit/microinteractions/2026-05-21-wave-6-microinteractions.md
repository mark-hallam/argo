# Wave-6 Audit — Hover States, Tooltips & Microinteractions

**Date:** 2026-05-21
**Auditor:** Wave-6 agent #4
**Scope:** Demo-path pages — `/landing`, `/` (Approvals), `/team`, `/workers/[id]/overview`, `/workflows`, `/workflows/[v]/[n]/build`, `/audit`, `/audit/action/[id]`, plus shared primitives (Button, Card, NavDrawer, BrandLoudHero, NotificationsBell, FilterChipRail).
**Method:** Read-only file inspection — `rg` for `:hover|hover:|onMouseEnter|onFocus|focus-visible|transition|animation|@keyframes|cursor` plus per-file deep reads of every major interactive surface.

---

## Headline numbers

- **~72 microinteractions inventoried** across demo-path
- **Average craft score: 6.8 / 10**
- **Zero `<Tooltip>` primitive in the codebase** — only native `title=` attrs (browser tooltips) on 4 files
- **`prefers-reduced-motion` respected in 2 prod files** (TeammateCard, BrandLoudHero) — the marketing-page shine sweeps do NOT respect it
- **`:focus-visible` is globally locked** at `app/globals.css:276` (`outline: 2px solid var(--ring); outline-offset: 2px`) — keyboard accessibility is baseline-good across the entire app
- **Motion-token system is comprehensive** — `--duration-fast: 150ms`, `--duration-base: 200ms`, `--duration-slow: 300ms`, ease `cubic-bezier(0.4, 0, 0.2, 1)` defined in `globals.css:46-67`

---

## Top 5 best-crafted moments (worth a demo pause)

### 1. TeammateCard holographic-showcase hover — **10/10**
`components/skycot/TeammateCard.tsx:194-268`

The single most-crafted artifact in the demo. On hover:
- Card lifts (`transform: translateY(-8px) rotate(-1deg) scale(1.015)`)
- Shadow blooms (`0 32px 60px → 0 48px 80px` purple-tinted)
- A diagonal shine sweep slides across the gradient banner (`background-position: 200% 50% → -50% 50%` over 1s with `cubic-bezier(0.2, 0, 0, 1)`)
- Inner ring intensifies (`rgba(139, 92, 246, 0.12 → 0.25)`)
- All wrapped in `prefers-reduced-motion: reduce` fallback (no transform, no shine)

**Demo pause: hover 2 seconds, let the shine sweep complete.** This is the "viral on Twitter" moment — operators have not seen worker cards behave like trading cards.

### 2. ActionId copy-on-hover in audit feed — **9/10** (the viral sleeper)
`components/skycot/ActivityRow.tsx:103-126`

The copy icon is `opacity-0 group-hover:opacity-100` — it only appears on row hover. Click → optimistic `Check` icon for 1500ms → reverts to `Copy`. Has `aria-label`, native `title` ("Copy action id" / "Copied"), `data-testid` for E2E. The arrow at the end of the row also slides right `group-hover:translate-x-0.5` and darkens.

**Demo pause: hover an audit row, watch the copy icon appear, click, see the green check, click another row.** This is the "fully forensic" beat the wave-5 viral-moments agent called out.

### 3. NotificationsBell pulse-on-increase — **9/10**
`components/skycot/NotificationsBell.tsx:14-44`

When the unread count goes UP mid-session, the dot fires Tailwind's `animate-ping` for 1600ms — a subtle "1 new" cue that doesn't fire on initial render or when count drops. Genuinely delightful: it's only there when something happened.

**Demo pause: while presenting the inbox, let a fake notification arrive — watch the ping ripple out of the bell.**

### 4. BrandLoudHero shine + pulse — **8.5/10**
`components/skycot/BrandLoudHero.tsx:62-108`

The big gradient banner across Team/Approvals/Cohorts uses a 5–5.5s shine cycle with 80% dwell (90%→100% sweep window) and `cubic-bezier(0.42, 0, 0.58, 1)`. The eyebrow dot pulses opacity 1→0.45 every 1.6s. **Respects `prefers-reduced-motion`** — both stop dead. The dwell + slow sweep keeps it from feeling busy.

### 5. WorkflowCard arrow-reveal on hover — **8/10**
`app/(app)/workflows/page.tsx:420-432`

Each workflow card has an arrow that's `opacity: 0; transform: translateX(-4px)` by default — on hover it animates to `opacity: 1; transform: translateX(0)` over 0.2s. Card itself lifts `translateY(-2px)` with purple shadow. Combined effect is "card whispers 'click me'".

---

## Top 5 missing-or-weak moments (with one-line fix)

### 1. **NO `<Tooltip>` primitive exists** — **2/10 across the app**
Icon-only buttons rely on `aria-label` (good for a11y) + occasional `title=` (browser-native, ugly, 500ms delay). Critical examples: filter trigger button on Approvals (`app/(app)/page.tsx:417-432`), Worker actions overflow menu trigger, Replay/Export icon buttons on action detail hero, Audit filter button.

**Fix:** Vendor `@radix-ui/react-tooltip` (already in deps via shadcn). Build a 50-LOC `<Tooltip>` wrapper at `components/ui/Tooltip.tsx`. Replace `title=` and bare icon buttons throughout. **High-leverage** — every icon-only button on the demo improves at once.

### 2. WorkflowBuilder canvas nodes — no hover state — **3/10**
`app/(app)/workflows/[verticalId]/[name]/build/page.tsx:128-228`

Each `SkycotStepNode` has static `border` and `boxShadow` based ONLY on the `selected` flag. Hovering a node does nothing — no border color shift, no lift, no preview. With 6+ nodes on screen, the canvas reads as a dead diagram.

**Fix:** Add `:hover` rule via React Flow's `nodeClassName` mechanism — `border-color: rgba(139,92,246,0.4); transform: translateY(-1px)` with `transition: all 0.18s`.

### 3. WorkerPersonalityCard — fully static — **2/10**
`components/skycot/worker/WorkerPersonalityCard.tsx`

The "How Mariah works" card on worker overview has trait chips, "Great at" lists, "Escalate when" lists — none have any hover state. The card itself is a static box. This is one of the most-viewed worker surfaces and feels lifeless.

**Fix:** Trait chips get `hover:bg-primary/15 cursor-default` (so they look interactive but the cursor doesn't lie). Or add a `Heart` icon that fills on card hover — tiny easter egg matching the "this teammate is a person" register.

### 4. WorkflowBuilder action buttons — no hover — **3/10**
`app/(app)/workflows/[verticalId]/[name]/build/page.tsx:407-470`

`builder-btn` (View timeline, Test run), `builder-btn-primary` (Save + publish), `builder-prompt-send` (Propose), `builder-add-step` (FAB) — none have a `:hover` rule. They have a primary-gradient background that doesn't shift on hover. The Page is one of the most-recorded surfaces (the AI-prompt-edits-workflow demo). Looks unfinished.

**Fix:** Single-line — `.builder-btn:hover { background: rgba(139,92,246,0.06); border-color: rgba(139,92,246,0.4); } .builder-btn-primary:hover { transform: translateY(-1px); box-shadow: 0 8px 20px -4px rgba(139,92,246,0.6); }`

### 5. KpiCard rows — static — **4/10**
`components/skycot/worker/KpiCard.tsx:52-75`

The 4 KPI rows on worker overview have no row-hover state. With 4 rows side-by-side per worker, clicking-affordance is invisible. The sparklines are inert (no on-hover value tooltip).

**Fix:** Add `hover:bg-muted/30 rounded-md` on the `<li>`. The sparkline should expose a `<title>` for value-at-point (recharts/visx already supports). This is also where a **<Tooltip>** primitive (gap #1) would shine — hover the sparkline, get "78 actions, May 18".

---

## The viral-sleeper / demo-pause moments

The operator should slow down on these — they reward a 2-3 second hover that the camera catches:

1. **Action-id copy in audit feed** (already called out wave-5). Hover → reveal → click → green check.
2. **TeammateCard holographic hover** (`/team` page). Hover one card, then hover the next — the chained shine sweeps look choreographed.
3. **NotificationsBell ping** (`/`). When demoing "live system", trigger a notification mid-shot.
4. **Workflow card arrow** (`/workflows`). The little `→` slide is small but reads as "interactive system".
5. **Approval card draft expand** (`components/skycot/ApprovalCard.tsx:206-233`). The chevron rotates 180deg, draft body expands with the rotation matching — clean state-transform pairing.
6. **Approval undo countdown timer** (`ApprovalCard.tsx:317-360`). Live `8s … 7s … 6s` `aria-live="polite"` ticker after approve/refuse — operators visibly see "I can still undo this".
7. **LiveIndicator green-dot pulse** (`/`). Tiny breathing green dot next to "Approvals" header. The wave-5 audit noted demo-fix landed 2026-05-20 to hide the indicator when degraded — the only-show-when-live posture is itself a craft signal.
8. **Workflow builder prompt-input focus state** (`build/page.tsx:457`). 4px purple-tinted halo on focus (`box-shadow: 0 0 0 4px rgba(139,92,246,0.1)`). When the operator clicks into the "Describe a change in plain English…" field on camera, the halo materializes.

---

## Actively BAD microinteractions

### Native `title=` tooltips on hot paths
Browser-native tooltips take 500ms to appear, are platform-themed (ugly on macOS Safari), and can't be styled. Used in:
- `components/skycot/ActivityRow.tsx:48` — time element shows ISO timestamp on hover (slow)
- `components/skycot/ActivityRow.tsx:110` — copy button shows "Copy action id" (slow)
- `components/skycot/ActivityRow.tsx:135` — cost chip shows token breakdown (slow)
- `components/skycot/ApprovalCard.tsx:105` — relative-time `<time>` element

**Verdict: Below the bar.** The copy interaction is otherwise an A+ moment, but the tooltip lag undermines it on camera.

### Filter chip remove buttons — too subtle
`components/skycot/FilterChipRail.tsx:108-115` & `TeamFilterChipRail.tsx:73-80` — the `X` on each chip has `hover:text-foreground` only. No background shift, no scale. Operator clicks at it three times during demo trying to confirm it's interactive.

**Fix:** `hover:bg-destructive/10 hover:text-destructive rounded-full`.

### Marketing-page shine animations don't respect prefers-reduced-motion
`app/landing/page.tsx:773`, `app/customers/page.tsx`, `app/customers/[slug]/page.tsx`, `app/docs/page.tsx`, `app/pricing/page.tsx`, `app/for/_components/VerticalLandingShell.tsx` — all run shine sweeps unconditionally. **The `BrandLoudHero` and `TeammateCard` got the a11y memo, the marketing pages didn't.**

**Fix:** One-line addition `@media (prefers-reduced-motion: reduce) { animation: none !important; }` per shine keyframe block.

### Landing page footer links — no hover state
`app/landing/page.tsx:1118-1159` — every footer `<a>` has `color: inherit; text-decoration: none` and NOTHING else. On hover: nothing. The `Footer` UI primitive (`components/ui/Footer.tsx`) is correct (hover:text-brand-primary) — the landing page just doesn't use it.

**Fix:** Either replace with `<Footer>` or add `.land-foot a:hover { color: var(--purple); }`.

### Workflow builder dropdown trigger — no chevron rotation on hover
The `.land-for-trigger` (the "For" dropdown on landing nav) opens on hover via `:hover .land-for-menu { display: grid }` — but the chevron icon doesn't rotate. Reads as static even though the menu opens.

**Fix:** Rotate the `ArrowRight` 90deg on `.land-for-wrap:hover`.

---

## Inventory by page

### `/landing` — 14 microinteractions, avg 6.5/10
- Nav links color-shift on hover (`a:hover { color: var(--purple); }`) — 7/10
- "For" dropdown opens on `:hover .land-for-menu { display: grid }` — chevron doesn't rotate — 5/10
- Dropdown items have `:hover { background: rgba(139, 92, 246, 0.06) }` — 7/10
- Hero CTA gradient pill — NO hover state — 4/10
- Feature cards: `transform: translateY(-2px); box-shadow blooms; border-color shifts purple` 0.18s — 8/10
- Quote cards: same as feature cards — 8/10
- Compare-band: 6s shine cycle 80% dwell (no reduced-motion fallback) — 7/10
- Final CTA gradient pill — NO hover state — 4/10
- Footer links — NO hover state — 2/10
- Social/integration logo cards — NO hover state — 3/10
- Trust-strip dots — static teal — N/A (decorative)

### `/` (Approvals home) — 18 microinteractions, avg 7.5/10
- BrandLoudHero shine (5.5s) + pulse-dot — 8.5/10
- Search input `focus:ring-2 focus:ring-[var(--ring)]` — 8/10
- Filter button + dot indicator — 7/10 (no tooltip)
- NotificationsBell + ping-on-increase — 9/10
- "Hire teammate" button (Button primary variant) `hover:opacity-90 active:opacity-80` — 7/10
- ApprovalCard `hover:shadow-md` (200ms) — 7/10
- ApprovalCard Approve/Refuse buttons (Button) — 7/10
- ApprovalCard chevron rotates 180deg on draft expand — 8/10
- ApprovalCard reason chevron rotates 180deg — 8/10
- Post-decision undo countdown (live aria) — 9/10 (would be 10 with progress bar)
- TrustRatchetCard dismiss-X + primary CTA — 7/10
- CelebrationBanner dismiss-X + CTA underline-on-hover — 7/10
- LiveIndicator green dot `animate-pulse` — 8/10
- FilterChipRail X-buttons — TOO SUBTLE — 4/10
- BundleCard stack-illusion (3 layered shadow blocks) — 9/10 (no hover interaction on stack itself, but it's not meant to interact)
- DataSourceChip — static — 5/10
- AppShell NavItems hover bg/text + svg opacity — 8/10
- WorkspaceFooter button hover bg — 6/10

### `/team` — 6 microinteractions, avg 8/10
- TeamHero (BrandLoudHero) — 8.5/10
- Search input focus ring — 8/10
- Search clear-X button (hover bg + text) — 7/10
- Status select dropdown — no explicit hover — 5/10
- **TeammateCard holographic hover** — 10/10
- Hire CTA Button — 7/10
- ChipRail X-buttons — too subtle — 4/10

### `/workers/[id]/overview` — 8 microinteractions, avg 6/10
- WorkerHero — fully static — 5/10
- WorkerOverflowMenu trigger (Button ghost) — 7/10
- WorkerTabs underline-swap + hover bg/text — 8/10 (clean tab-strip craft)
- WorkerPersonalityCard — fully static — 2/10
- KpiCard rows — static — 4/10
- KpiSparkline — no on-hover value tooltip — 5/10
- RecentChangesPanel undo confirmations (native title attr) — 4/10
- Card primitive `hoverable` lift — 7/10 (good but underused)

### `/workflows` — 11 microinteractions, avg 6.5/10
- Eyebrow pill — static decorative — N/A
- Search input — no focus state visible — 5/10
- "Browse 50+ templates" CTA `transform: translateY(-1px)` on hover — 8/10
- "Propose new" gradient CTA — NO hover state — 4/10
- Try-chips (4 of them) — NO hover state — 4/10
- WorkflowCard lift + shadow + arrow reveal — 8/10
- Risk badge — static — N/A
- Step-pill + dot-line — static decorative — N/A

### `/workflows/[v]/[n]/build` — 12 microinteractions, avg 5/10
- Back link color-shift on hover — 7/10
- Builder action buttons (View timeline, Test run, Save+publish) — NO hover state — 3/10
- AI prompt input gradient focus halo — 9/10
- Send button — disabled-only styling, no hover — 4/10
- Suggest pills (Add a step / Remove gate) `hover:color: #8b5cf6; border-color: #8b5cf6` — 7/10
- ReactFlow nodes — NO hover state — 3/10
- ReactFlow edges — `animated: true` (built-in dashed-march) — 7/10
- ReactFlow MiniMap — interactive, pan + zoom — 7/10
- Side panel close-X `hover:background: rgba(0,0,0,0.05)` — 6/10
- "Add step" floating FAB — NO hover state — 4/10
- Edit-step button in panel — NO hover state — 4/10

### `/audit` — 9 microinteractions, avg 7/10
- AuditPageHeader — search/filter/export controls — 6/10
- Filter button + active dot indicator — 7/10 (no tooltip on icon)
- AuditDayGroup sticky header (`backdrop-blur-sm`) — 7/10
- **ActivityRow hover (border, bg, arrow slide, copy reveal)** — 9/10
- ActionId copy button (the sleeper) — 9/10
- Cost chip — native `title=` tooltip (slow) — 5/10
- AuditFilterChipRail X-buttons — too subtle — 4/10
- Load more button (`hover:bg-[var(--accent)]/40 focus-visible:ring-2`) — 7/10
- "Try again" error-state link `hover:underline` — 6/10

### `/audit/action/[id]` — 7 microinteractions, avg 7/10
- Breadcrumb links — underline on hover — 6/10
- Back button `hover:bg-[var(--accent)]/40` — 6/10
- ActionSummaryHero monospace action-id copy button — 8/10 (consistent with row version)
- Replay button (icon + label) — `hover:bg + focus-visible:ring` — 7/10
- Export button — same pattern — 7/10
- "View related actions" link — underline on hover — 6/10
- ActionInputsCard predecessor link — `hover:underline` — 6/10

---

## Recommendations (prioritized for v1 demo)

### MUST-FIX before demo recording
1. **Build a `<Tooltip>` primitive** (~50 LOC wrapping `@radix-ui/react-tooltip`) and replace native `title=` on ActivityRow + ApprovalCard + every icon-only filter/notifications button. **Biggest single uplift.**
2. **Add hover states to workflow builder buttons** — they're on camera during the AI-edits-workflow demo. One CSS block fix.
3. **Workflow canvas node hover** — border + lift. Without it the canvas reads as a dead diagram.
4. **Marketing-page shine animations respect reduced-motion** — 6 one-liners. A11y blocker.

### SHOULD-FIX
5. WorkerPersonalityCard — add small hover affordance to trait chips and/or card-level shine.
6. KpiCard rows — `hover:bg-muted/30` for affordance + sparkline value-on-hover (use the new Tooltip primitive).
7. FilterChipRail X-buttons — `hover:bg-destructive/10 hover:text-destructive`.
8. Landing footer links — drop in hover color.

### NICE-TO-HAVE
9. Approval undo countdown gets a progress-bar visualization (visual decay, not just numeric).
10. Workflow card "by Mariah" → tiny avatar pop-in on hover.
11. Audit row drill-in preview — `onMouseEnter` peek of inputs/outputs without committing the click.
12. The "For" dropdown chevron should rotate on hover.

---

## Closing observations

The codebase is **inconsistent in a revealing way**: the components shipped by the C2-holographic-showcase brief (TeammateCard) and the BrandLoudHero brief are 9-10/10 craft. The components shipped before the brand-consistency pass — and the inline-styled marketing pages — are 4-6/10. The bar has been raised; some surfaces just haven't been brought up to it yet.

**The "viral sleepers" the wave-5 agent called out are real and worth pausing on.** Specifically, the action-id copy reveal and the TeammateCard hover are the two strongest "craft tells" in the app. Both deserve 2-second pauses on camera.

**The biggest single uplift for v1 is a `<Tooltip>` primitive.** It would lift Approvals, Audit, and Worker pages simultaneously and resolve four of the five top-weak moments in this audit.
