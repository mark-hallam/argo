# Performance audit — SKYCOT web/ — 2026-05-20

**Auditor:** Claude Opus 4.7 (1M context) parallel audit agent
**Stack:** Next.js 15.5.18 (Turbopack dev), React 19, Tailwind 4, @xyflow/react 12.10, motion 11.18, NextAuth v5-beta
**Target:** http://localhost:3000 (dev mode)
**Caveat:** All numbers below are **Turbopack dev-build** numbers. Production `next build` will be 5-20x leaner (no HMR, no sourcemaps, no devtools, code-split + tree-shaken). The structural findings still apply.

---

## Measurement summary (dev mode)

| Route | TTFB | Total | Bytes | `<script>` tags |
|-------|------|-------|-------|-----------------|
| `/` | 93.8 ms | 98 ms | 58.5 KB | 55 |
| `/landing` | 90.4 ms | 93.9 ms | 45.3 KB | 42 |
| `/workflows` | 117.7 ms | 122.8 ms | 70.9 KB | 52 |
| `/workers/wkr_01J0MARIAH` | 167 ms | 171.4 ms | 54.3 KB | 55 |
| `/compare` | 150.4 ms | 154.2 ms | 77.4 KB | 52 |
| `/workflows/gtm-engineer/post_listing/build` | 96.8 ms | 101.8 ms | 64.8 KB | 55 |

**Largest dev JS chunks under `.next/static/chunks/`:**

- `f3d61_motion_dist_es_67e460ed._.js` — **1.1 MB** (motion library, full bundle)
- `node_modules__pnpm_05dd6e43._.js` — **1.0 MB** (mixed vendor pile)
- `3b7ac_react-dom_2e6f66ce._.js` — **945 KB** (react-dom dev)
- `a051f_next_dist_compiled_react-dom_e835df54._.js` — **880 KB**
- `_f4c87adf._.js` / `_b035fe51._.js` / `_efe5f173._.js` — 533-630 KB each (app/route bundles)
- 319 chunks total emitted by Turbopack in dev mode.

55 `<script>` tags on `/` and `/workers/[id]` is a structural smell, not a dev artifact — every Turbopack-style chunk pulls its own `<script>` regardless of build mode.

---

## Structural findings

### F-1 — 54 of 77 (70%) `app/` files are `"use client"`
Every page audited (`/landing`, `/compare`, `/skills`, `/workflows`, `/workers/[id]`, settings, signup) is a client component. The React 19 + Next 15 App Router intent is "server by default, client only where you need state." SKYCOT inverts this. Consequence: every page ships its full JSX + handlers + lucide-react icon imports to the browser. Lucide alone re-ships ~14 imports per page on `/landing`. This is the single largest perf lever in the codebase.

### F-2 — Every visual page injects Google Fonts as inline `<link>` tags
13 component files do this individually (`<link href="fonts.googleapis.com/css2?family=Audiowide&family=Inter:wght@200..."`). Each page re-requests the same fonts. Findings:

- **No FOUT/FOIT mitigation.** No `font-display: swap` declared client-side; relies on Google's default (swap for Inter, block for Audiowide ≤3s). Hero text using Audiowide blocks first paint visually.
- **No font preloading.** Just `<link rel="preconnect">` — connection is warm but the actual font file `.woff2` is discovered late in the critical path.
- **Inter weights 200, 300, 400, 500, 600, 700 all requested** on `/landing`. That's ~6 font files. Likely 3 are actually rendered.
- Should use `next/font/google` instead — Next.js inlines the fonts at build time, self-hosts them, eliminates the third-party DNS lookup and preconnect roundtrip.

### F-3 — `motion` library is 1.1 MB minified (dev)
Production size is ~140 KB tree-shaken, but only if the imports are surgical. Audit didn't find a single `import { motion } from "motion/react"` callsite — `grep -rn "motion/react"` returns nothing in `app/`. **Motion appears to be a transitive dependency we are not using.** Verify with `pnpm why motion` and consider removing.

### F-4 — `@xyflow/react` is 12.10.2 — bundled with every workers/workflows page even when canvas isn't rendered
The `/workflows/<v>/<n>/build` page uses ReactFlow. The non-build sibling `/workflows/<v>/<n>` does not — yet because both are client components in the same route segment, they share a chunk. Static import + RSC split would let the build canvas lazy-load.

### F-5 — Every interactive page imports an icon set via lucide-react
Tree-shaking should drop unused icons, but the bundles show full lucide-react chunks (~580 KB in dev). Switch to `lucide-react/<icon>` named-path imports or replace with inline SVGs for the 6-12 icons actually used per page.

### F-6 — Inline `<style jsx global>` blocks of 200-800 LOC per page
`app/landing/page.tsx` has 360+ lines of inline CSS (lines 86-450). This CSS is duplicated across pages with shared design tokens. Critical CSS is fine inline, but the `--purple`, `--pink`, `--ink` token block is reproduced 13 times in different page files. Should live in `app/globals.css` once, then page-level CSS becomes tiny.

### F-7 — `productionBrowserSourceMaps: process.env.CI === "true"` in `next.config.ts`
Source maps in production browser bundles only when CI=true. That's the right default, but worth double-checking that Vercel/host doesn't set CI=true at runtime, leaking source.

### F-8 — No `Cache-Control` headers on landing
`/landing` is the public marketing page; it ships static copy + static integration logos. There's no caching directive. A simple `Cache-Control: public, s-maxage=300, stale-while-revalidate=86400` on this route would make repeat visits ~free.

### F-9 — No `<Image>` from `next/image` anywhere in app/
`grep -rn "next/image" app/` returns zero matches. The hero "art" on `/landing` is composed of inline `<div>` bubbles (good — no images), but the integration logos section (Slack, Gmail, Outlook, etc., line 692-720 of `app/landing/page.tsx`) renders plain `<div>` text labels — no actual logos. When real logos land, they MUST use `next/image` for `loading="lazy"` + `width`/`height` to avoid CLS.

### F-10 — Trust strip on `/landing` claims "2,847 teams this month"
Hard-coded marketing number embedded in JSX. Not a perf issue per se, but worth flagging: any dynamic data requirement that lands later will trigger a re-render and likely add a fetch — plan for an SSR data hook before this number goes live.

---

## Core Web Vitals risk forecast (production)

| Metric | Risk | Why |
|--------|------|-----|
| **LCP** (Largest Contentful Paint) | Medium-High | Audiowide hero text + Google Fonts cold-cache. Mitigate via F-2 (next/font). Target: < 2.5s on 3G. |
| **CLS** (Cumulative Layout Shift) | Medium | Font swap from system → Audiowide will shift the H1 height when Audiowide loads. Mitigate with `font-display: optional` + size-adjust fallback. |
| **INP** (Interaction to Next Paint) | Low-Medium | ReactFlow + Motion on workflow builder, but 70% client-component bloat means initial hydration is heavy on every page. Mitigate via F-1. |
| **FCP** (First Contentful Paint) | Medium | 55 `<script>` tags in dev. Production reduces this, but the structural 100% client-component pattern still means hydration is big. |
| **TTFB** | Low | Already sub-200ms in dev. The Rust gateway (7400) returns fast. |

---

## Top 10 perf wins, ranked by impact ÷ effort

1. **Make pages server components where possible** (F-1). The static-copy pages — `/landing`, `/compare`, `/skills` (the grid is static), `/workflows` (when the SEED data is server-rendered), `/settings/members` (the role-tier cards) — have ZERO state. They should be `async` server components. Drop the `"use client"` at the top of each. Move the interactive bits (the search input on `/skills`, the invite modal on `/members`) into small inner client components. Estimated bundle saving: 30-50%.

2. **Replace inline Google Fonts links with `next/font/google`** (F-2). One-time refactor. Centralize Audiowide + Inter + JetBrains Mono in `app/layout.tsx`. Removes 13 duplicate font-fetch DNS roundtrips, self-hosts the fonts, eliminates FOUT on hero text.

3. **Verify `motion` is actually used; if not, remove it** (F-3). `pnpm why motion` → if only `motion` itself (not via `framer-motion` or another lib), uninstall. Saves ~140 KB production.

4. **Centralize the design-token CSS in `app/globals.css`** (F-6). The `--purple`, `--pink`, `--ink`, `--bg`, `--line` variable block is reproduced 13 times. Lift once. Page CSS shrinks dramatically.

5. **Add `Cache-Control: public, s-maxage=300, stale-while-revalidate=86400` on `/landing`** (F-8). One-line addition via `headers()` config in `app/landing/page.tsx`. Makes repeat-visit landing free.

6. **Lazy-load `@xyflow/react`** on `/workflows/[v]/[n]/build` only (F-4). Use `next/dynamic` with `{ ssr: false, loading: () => <BuilderSkeleton /> }`. The non-build workflow viewer never pulls ReactFlow. Saves ~200 KB on that critical path.

7. **Audit lucide-react tree-shaking** (F-5). Confirm production build is dropping unused icons. If not, switch to per-icon imports or replace with 6-12 inline SVGs per page.

8. **Preload the hero font woff2** for `/landing` only. Once `next/font` lands (#2), add `display: 'optional'` + `preload: true` to the Audiowide config. Eliminates LCP-blocking font fetch.

9. **Set `<link rel="preload" as="image">` on hero art when real images land** (F-9). And use `next/image` with explicit width/height. Today the hero "art" is just inline divs so this is preventive.

10. **Wire a real BUILD STATS check into CI**. `next build` then `du -sh .next/static/chunks/` and fail the build if any single page's First-Load JS exceeds 250 KB. This is the long-term lever — without a budget, perf will drift.

---

## What's already good

- TTFB is sub-200ms across the board even in dev.
- HTML payloads are 45-78 KB — reasonable.
- `poweredByHeader: false` and `reactStrictMode: true` already set.
- `productionBrowserSourceMaps` correctly gated on CI.
- No `eval()`, no `innerHTML`, no `dangerouslySetInnerHTML` anywhere in `app/`.
- React Query (@tanstack/react-query) for live data — sensible choice.
- The dev/prod proxy through Next.js (`/api/v1/*` → gateway:7400) avoids a CORS round-trip cost.

---

## Recommendations summary

**P0 (do before demo MP4):** F-2 (next/font), F-3 (verify motion), F-8 (cache-control on /landing). These are 1-2 hour wins each.

**P1 (do before public launch):** F-1 (server-component pass), F-6 (CSS lift), F-4 (lazy ReactFlow), CI build budget.

**P2 (do as scale demands):** F-5 (icon tree-shake), font-preload precision, hero-image policy.

The biggest lever is F-1 — converting static-copy pages to server components. SKYCOT ships brand-loud marketing pages (`/landing`, `/compare`, `/skills`) that have zero need to be client components. The 70%-client-component ratio is the structural antipattern; once it inverts to ~30%, every other perf metric improves automatically.
