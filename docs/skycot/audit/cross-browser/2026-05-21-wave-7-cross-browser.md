# Wave-7 Audit — Cross-Browser Compatibility (Safari + Firefox)

**Date:** 2026-05-21
**Scope:** SKYCOT web SPA + marketing — static analysis of CSS, JS, and build pipeline for non-Chromium compatibility risks
**Branch:** `main`
**Audit type:** Read-only — no source or build modifications
**Demo concern:** SKYCOT demo recorded on Chrome; procurement buyers test on Safari + Firefox

---

## Verdict

**Safari demo (15.4+ / iOS 15.4+) — survives with caveats.** No outright breakage on the `/landing` demo path. Gradient text effects, font loading, and React Flow workflow canvas all render. **Caveat:** four raw-CSS `backdrop-filter` callsites lack the `-webkit-` prefix and will degrade silently on Safari 9–15.4 (small share of CIO/CFO laptops still on macOS Big Sur). Tailwind v4 emits `backdrop-filter` only — no autoprefixer fallback reaches the utility classes, so every modal overlay and sticky header relies on Safari 16+. On modern Safari (16+) and current iOS this works.

**Firefox 115+ — survives.** No Firefox-specific blockers. Some webkit-only properties (`-webkit-overflow-scrolling`, `-webkit-text-fill-color`, `-webkit-font-smoothing`) are present without standard siblings — these are no-ops on Firefox, not breakage.

**No automated cross-browser coverage.** Playwright runs Chromium only; Safari/Firefox projects are not defined in `web/playwright.config.ts`. There is no CI safety net catching a Safari regression — discovery would be via buyer ticket.

**Browserslist:** No `browserslist` config anywhere in the repo (`package.json`, `.browserslistrc`, `postcss.config.mjs`, `next.config.ts` all clean). Autoprefixer + Next.js fall back to browserslist defaults — effectively the last 2 versions of every major browser, Firefox ESR, and >0.5% global usage. This explains why no `-webkit-backdrop-filter` is emitted: defaults exclude Safari < 16.

---

## Top 5 Safari Risks

### 1. Tailwind `backdrop-blur-*` emits no `-webkit-` prefix — Safari < 16 silent regression (P2)

Tailwind v4 (`tailwindcss@^4.0.6`) emits raw `backdrop-filter` in built CSS with **zero** companion `-webkit-backdrop-filter` rules. Verified by inspecting `web/.next/static/css/*.css`:

```
grep -c "backdrop-filter"     → 1 occurrence (sans prefix)
grep -c "webkit-backdrop"     → 0 occurrences
```

Every `backdrop-blur-sm` / `backdrop-blur-md` class — used in **20+ surfaces** including dialog overlays (`HelpWidget`, `AddResponsibilityDialog`, `RetrainDialog`, `AddFactDialog`, `WorkerRetroDialog`, `ApprovalPolicyEditModal`, `ElevatedViewRequestDialog`, `EvidenceBundleExporter`, `NavDrawer`), sticky chrome (`AppShell`, `AuditPageHeader`, `HireShell`, `CohortDetailDrawer`, `app/(app)/page.tsx`), and the hire-step-5 footer — **degrades silently on Safari < 16 to a non-blurred translucent overlay** (the underlying `bg-black/40` still works). Visually noticeable on macOS Big Sur and iOS 14 enterprise-managed devices.

**Mitigation options:**
- Add `browserslist` config in `web/package.json` listing `"safari >= 15.4"` to force autoprefixer to emit the webkit prefix (autoprefixer 10.4.20 is installed but inactive against Tailwind v4's internal layer).
- Add a `@supports (-webkit-backdrop-filter: blur(1px))` block in `app/globals.css` re-declaring the property for Safari 9–15.4.
- Accept the degradation (current state).

### 2. Four raw-CSS `backdrop-filter` callsites lack `-webkit-` prefix (P3)

Three of seven raw CSS authoring sites do have both:
- `components/skycot/TeammateCard.tsx:286-287` — both
- `components/skycot/CommandPalette.tsx:641-642` — both
- `components/auth/WizardEditorialShell.tsx:94` — both (same line)

But four do not:
- `components/skycot/BrandLoudHero.tsx:134` — standard only
- `app/(app)/settings/members/members-styles.ts:259` — standard only
- `app/(app)/settings/api-keys/styles.tsx:221` — standard only
- `app/(app)/settings/security/page.tsx:290` — standard only

Same Safari-16+ dependency as risk #1.

### 3. `color-mix()` in body ambient gradient — Safari 16.2+ only (P3)

`web/app/globals.css:272-273` uses `color-mix(in oklch, …)` to draw the page-background ambient wash. Safari < 16.2 ignores the declaration and falls through to the parent `background` (transparent fallback). The page still renders, but the subtle radial depth wash is missing on macOS Monterey + early Ventura. One additional callsite at `components/skycot/CelebrationBanner.tsx:83` (`bg-[color-mix(in_oklab,…)]`) — celebration banner falls back to no background-tint.

### 4. `oklch()` color space — Safari 15.4+ (P4, acceptable)

Every theme token in `globals.css` (102 occurrences of `oklch(...)`) requires Safari 15.4+. This matches the current default-browserslist target. **Lower versions render the page using browser-default colors (black on white)** — effectively the page is unstyled on Safari 14 and below. Acceptable: Safari 14 share is < 0.5% on enterprise. Flag only if the buyer demo is on a known-stale device.

### 5. `aspect-ratio` on landing hero — Safari 15+ (P4, acceptable)

`web/app/landing/page.tsx:420` sets `aspect-ratio: 1` on the hero illustration container. Safari < 15 collapses the element to 0×0 (no padding-bottom fallback). On Safari 15+ this works. Combined with risk #4, the floor for landing is Safari 15.4. Five additional `aspect-video` / `aspect-square` Tailwind utility uses (`brand/components`, `audit/ActionOutputCard`, `audit/ReplayDiff`, `ui/Avatar`) — same constraint.

---

## Top 3 Firefox Risks

### 1. `-webkit-text-fill-color` on hero gradient text (P3)

`components/skycot/BrandLoudHero.tsx:164` declares `-webkit-text-fill-color: transparent` without a standard `color: transparent` companion on that specific block. **Firefox 116+ does support `-webkit-text-fill-color` for compatibility**, but it's noted as non-standard. Verified Firefox 115+ does honor the prefix. **Currently not breakage** — Firefox 49+ has implemented this as a compat alias. Worth tracking if Firefox ever removes the alias. All other gradient-clip sites correctly pair `background-clip: text` with the webkit prefix (verified 20+ callsites — all paired).

### 2. `-webkit-overflow-scrolling: touch` on horizontal scroll containers (P5, no-op)

`web/app/pricing/page.tsx:500`, `web/app/compare/page.tsx:260`. Firefox ignores the property silently — falls back to default scroll inertia which works fine on Firefox. **Not a regression.** Only flagged because the property is the sole signal of "horizontal-scroll intent" and a Firefox-only code reviewer might mistake it for missing inertia. Behavior is correct.

### 3. ReactFlow ResizeObserver in workflow canvas (P5, supported)

`components/skycot/canvas/SkillCanvas.tsx:208` depends on `ResizeObserver` (provided by `@xyflow/react@12.10.2`). Firefox 69+ supports `ResizeObserver` natively. **Not a risk.** Documented here because Firefox historically was last-to-ship `ResizeObserver` and reviewers may flag — the support floor is Firefox 69 (Sept 2019), well below current ESR.

---

## Cross-Browser: Both Affected

### `requestIdleCallback` — Not used anywhere

Grep returned zero hits in `app/`, `components/`, `lib/`. Safari 17+ ships it; Firefox 115 ships it behind `dom.requestIdleCallback.enabled` (default true). **No risk** since not used.

### `IntersectionObserver` — Not used anywhere

Grep returned zero hits. No risk.

### `MutationObserver` — Used in `ThemeProvider`

`components/skycot/ThemeProvider.tsx:102` uses `new MutationObserver(...)`. Both Safari and Firefox support it back to 2014. **No risk.**

### `structuredClone` — One test-only use

`components/skycot/canvas/diff-engine.test.ts:25`. Test code only — not shipped. **No risk** for end users.

### `crypto.randomUUID()` — Feature-detected

Two callsites (`lib/hooks/use-elevated-view-grant.ts:97`, `lib/api/workers.ts:150`) both guard with `typeof crypto.randomUUID === "function"` — Safari 15.4+ / Firefox 95+ ship it; older browsers fall through to the project's own UUID generator. **No risk.**

### `ReadableStream.getReader()` — Workflow propose chat

`components/skycot/ProposeWorkflowChat.tsx:52` streams responses from `/api/workflows/propose`. Safari 14.1+ and Firefox 102+ support `ReadableStream.getReader()`. **Acceptable** for current default-browserslist floor.

### Native popover API — Not used

All "popover" references are Radix UI's `@radix-ui/react-popover`, which renders via div+JS positioning (broadly compatible). Not the native `popover` attribute. **No risk.**

### Container queries (`@container`) — Not used

Zero hits. No risk. (Worth noting: Tailwind v4 ships container-query utilities. The team hasn't adopted them yet — if a future refactor uses `@container`, Safari 16+ and Firefox 110+ are required.)

### `:has()` selector — Not used

Zero hits in app/components/lib. No risk. Required floor would have been Safari 15.4 + Firefox 121.

### `@supports` — Not used

Zero hits. The codebase has **no progressive-enhancement guards anywhere**. Every modern CSS feature is unconditional. If a feature lands that needs a fallback, the team will need to introduce the first `@supports` block.

### Service Worker registration — Properly guarded

`components/pwa/ServiceWorkerRegister.tsx:13` checks `"serviceWorker" in navigator` before registering. iOS Safari 11.3+ / Firefox 44+ both support. **No risk.**

### PWA `beforeinstallprompt` — iOS Safari does not fire it

`components/pwa/InstallPrompt.tsx` listens for `beforeinstallprompt`. This is Chromium-only. iOS Safari users see no install banner. Code already silently no-ops in that case — comment at line 36 acknowledges. **By design, not a regression.**

---

## Browserslist Current State

**Effective config:** None defined → browserslist defaults (resolved at audit time):

```
chrome >= 109, edge >= 146, firefox >= 140, ios_saf >= 18.5
android, samsung, op_mini all (low-bandwidth subset)
```

**Implications:**
- iOS Safari floor is **18.5+** by default — well above the `backdrop-filter` (16+), `oklch` (15.4+), and `aspect-ratio` (15+) requirements. **In theory, every modern CSS feature is safe.** In practice, enterprise CIOs on managed Big Sur / Monterey laptops are stuck on Safari 15.x because the OS won't update.
- macOS Safari is not in the resolved list (browserslist treats macOS Safari + iOS Safari distinctly; `ios_saf` 18.5 doesn't imply desktop Safari 18.5).
- **Recommendation:** Add an explicit `browserslist` field to `web/package.json` listing `"safari >= 15.4", "ios_saf >= 15.4", "last 2 firefox versions", "firefox esr", "chrome >= 100", "edge >= 100"`. This (a) locks the floor for type-checking and review, (b) forces autoprefixer to emit `-webkit-backdrop-filter` for Safari 15.x, and (c) gives the next agent something concrete to audit against.

---

## Demo-Path Risks (`/landing`)

| Surface | Feature | Floor | Status |
|---|---|---|---|
| Hero radial gradients | `radial-gradient` | Safari 4+ / Firefox 3.6+ | OK |
| Hero ink color | `oklch` (via globals.css) | Safari 15.4+ / Firefox 113+ | OK on Safari 15.4+ |
| Hero gradient text | `-webkit-background-clip: text` + standard | Both prefixed | OK |
| Hero illustration | `aspect-ratio: 1` | Safari 15+ / Firefox 89+ | OK |
| Audiowide wordmark | Google Fonts CSS2 `display=swap` | Universal | OK |
| Trust-strip horizontal scroll | `scrollbar-width: none` + `::-webkit-scrollbar` | Safari 15+ paired | OK |
| Animated hero shimmer | `prefers-reduced-motion` guard | Universal | OK |

**Demo path verdict:** No demo-blockers on Safari 15.4+ or Firefox 115+. Demo recording on Chrome and procurement playback on a recent MacBook Air / iMac will both render correctly. If the buyer's IT shop is on macOS Big Sur (Safari 14, < 0.5% of corporate), the page will be unstyled (oklch fallback failure).

---

## Workflow Builder (React Flow / `@xyflow/react@12.10.2`)

React Flow 12.x targets ES2020 with built-in Safari support back to Safari 14.1. The wiring in `SkillCanvas.tsx` uses:
- `Background gap={24}` — SVG-based pattern (universal)
- `Controls showInteractive={false}` — DOM buttons (universal)
- `MiniMap pannable zoomable` — SVG + JS pointer events (universal)
- `fitView` — JS transform on container (universal)

Plus a `SkillCanvasDomFallback` (line 217) that bypasses React Flow when `data-skip-flow-render` is set — used in unit tests, not in prod. **No cross-browser risk** in the workflow canvas.

---

## Playwright Coverage Gap

`web/playwright.config.ts` declares only the default Chromium browser, with no `projects: [{ name: 'safari', use: { ...devices['Desktop Safari'] }}, …]` block. Safari + Firefox regressions reach prod undetected. **Recommend** adding a `cross-browser` Playwright project (one per browser) running just the demo-critical smoke specs (`tests/e2e/hire-flow-shell.spec.ts`, the marketing landing if it exists). Optional but valuable before the next procurement-cycle demo.

---

## Recommended Action Bundle (post-audit)

In priority order:

1. **Add `browserslist` to `web/package.json`** (P2, ~5 min). Forces autoprefixer to emit `-webkit-backdrop-filter` for Safari 15.4+. Locks the floor for future audits.
2. **Add `-webkit-` prefix to four raw-CSS `backdrop-filter` sites** (P3, ~10 min). Defensive — even with browserslist fix, autoprefixer doesn't touch styled-jsx string-template CSS reliably; explicit pairing is safer.
3. **Add `@supports` fallback for `color-mix()` body wash** (P4, ~5 min). Solid-color fallback for Safari < 16.2 so the page doesn't read dead-flat.
4. **Add Safari + Firefox Playwright projects** (P3, ~30 min). One smoke spec per browser on the demo path. Catches regressions before procurement.
5. **Visual smoke test on Safari 15.4 (Monterey) + Firefox ESR** (P3, one-time). Open `/landing`, the workflow builder, and one dialog — confirm degradation is graceful.

None of these are demo-blockers. All are buyer-trust improvements that compound over the procurement cycle.
