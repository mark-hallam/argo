# fix-compare-page-html-payload-bloat-321kb

**Owner**: claude.
**Authored against**: origin/main `56f4ea0b4d84ebf52c14e387d95c4e99e1cca0b7` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-compare-page-html-payload-bloat-321kb`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-27-0415/whole-site-mvp-claims-coverage.md` F8 [P2].

🟢 **P2 — `/compare` returns 321 KB of HTML, > 5× the next-largest marketing page. Suggests a giant inline data structure rendered server-side instead of streamed/tabled client-side. Lighthouse score will hurt; not a POC blocker because the demo camera doesn't drill into `/compare`, but a public-facing performance gap.**

## Root cause

`web/app/compare/page.tsx` (verify path) renders the competitive-intel comparison matrix from a likely-inline TypeScript constant. Each competitor entry probably carries 30-50 fields (feature parity, pricing, logos, descriptions) × 8-15 competitors = a dense object literal serialised into the initial server-render HTML.

Comparison points (per the audit's HTML-size sweep):

- `/landing`: ~55 KB
- `/pricing`: ~50 KB
- `/customers`: ~40 KB
- `/compare`: **321 KB** ← outlier

The size is dominated by inline data, not by hero copy or page structure. Two architectural choices led here:

1. The competitor matrix is server-rendered for SEO (every comparison appears in source HTML — good for Google).
2. The matrix data lives in a single big TS module that gets fully serialised on every render.

## Findings this brief closes

- whole-site-mvp-claims-coverage F8 [P2] — `/compare` page is 321 KB of HTML

## Fix recipe

Two approaches, operator-decisional:

### Option A — split into per-competitor static pages (SEO-friendly, keeps server render)

Generate one static page per competitor at `/compare/<competitor>`. The index `/compare/page.tsx` becomes a 5-card grid linking to each. Each detail page renders only one competitor's feature parity table — likely ~30-50 KB. Bonus: Google indexes individual `/compare/<competitor>` URLs which is better for SEO than a single mega-page.

### Option B — keep the single-page table but lazy-load below-the-fold sections

Convert `/compare/page.tsx` to a server component that streams the first 2-3 competitor tables, then a client component that hydrates the rest on scroll. ~50-100 KB initial payload, ~250 KB lazy.

### Option C — tabular DOM with virtualisation

The 321 KB is probably amplified by deep JSX nesting per row × per competitor. Flatten to a `<table>` with `<thead>` + virtualised `<tbody>` (react-virtuoso or similar). Initial payload ~60-80 KB.

**Recommend Option A** — best SEO, simplest to implement, fits the existing static-marketing-page pattern.

## Acceptance criteria

- [ ] `/compare` (or its replacement) returns < 100 KB of initial HTML.
- [ ] Each competitor's parity table is reachable from `/compare/<slug>` (Option A) or from a single page that hydrates on scroll (Option B/C).
- [ ] Lighthouse performance score on `/compare` ≥ 85 (currently likely < 60 due to the bloat).
- [ ] No regression on competitor content — all matrices still render the same data.
- [ ] If Option A: sitemap regeneration includes `/compare/<competitor>` URLs.

## May-touch

- `web/app/compare/page.tsx` (current implementation)
- `web/app/compare/[competitor]/page.tsx` (NEW under Option A)
- `web/lib/marketing/competitors.ts` (likely where the data lives — verify path)
- `web/playwright/marketing/compare-payload-size.spec.ts` (NEW — asserts HTML size)
- `next-sitemap.config.js` (if Option A — regenerate sitemap)

## Out of scope

- Competitive intel content authoring (separate concern)
- Pricing-page link from `/compare` (not in the scope of this brief)
- Landing-page hero performance (separate brief: `fix-landing-hero-mobile-stack-and-tablet-nav`)

## Effort

Option A: ~200-400 LOC (page split + router setup) + sitemap regen. ~6-8 hours. Option B: ~150-300 LOC. Option C: ~250-400 LOC + virtualisation lib. P2 because public-site polish, not a demo blocker.
