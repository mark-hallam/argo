# web-seo-og-image-and-opengraph-route — ship the OG image (the BIG missing SEO asset)

**Owner**: claude.
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-seo-og-image-and-opengraph-route`.
**Priority**: P2 — wave-2 SEO audit "top multi-page fix" + "single most-fixable brand embarrassment". No OG image anywhere; every social-share unfurl is text-only.
**Sizing**: ~150-300 LOC (opengraph-image.tsx + per-page metadata + Audiowide wordmark + tests).

## Why this brief exists

From the wave-2 SEO audit (`docs/skycot/audit/seo/2026-05-20-wave-2-seo.md`):

> Top multi-page fix: Add one root-level OG image at
> `web/public/og-default.png` (1200×630, SKYCOT brand gradient +
> wordmark + tagline) and reference it in the root `openGraph.images`
> array. Single change makes social previews go from blank to
> brand-on across all 10 public pages.
>
> Social-share preview verdict: **Embarrassing.** No OG image
> exists anywhere — no `opengraph-image.tsx`, no PNG in `/public`,
> no `/api/og` route. Slack/LinkedIn/Twitter/iMessage unfurls
> currently render text-only with blank visual.

## Goals

1. Add `web/app/opengraph-image.tsx` (Next.js `next/og` API) — generates a 1200×630 PNG on demand for the root path.
2. Brand-on visual: purple→pink gradient bg + Audiowide "SKYCOT" wordmark + tagline below + small "Hire your first AI teammate" CTA-shape.
3. Per-page OG images for key surfaces: `/landing/opengraph-image.tsx`, `/pricing/opengraph-image.tsx`, `/compare/opengraph-image.tsx`, `/customers/opengraph-image.tsx`. Each with surface-specific tagline.
4. Verify: `curl http://localhost:3000/opengraph-image` returns a real PNG.
5. Verify metadata: visit any public page → its `<head>` has `og:image` pointing to a working URL.

## Steps

1. Read `next/og` API (Next.js docs) — uses `ImageResponse` from `next/og`.
2. Create `web/app/opengraph-image.tsx` (root default).
3. Use `JSX → SVG → PNG` via ImageResponse. Inline Audiowide font (via Google Fonts fetch in the route — Next docs example).
4. Replicate per top-level public page with surface-specific copy.
5. Smoke test: each public page's metadata API exposes the OG URL; the URL returns 200 + image/png.

## may-touch

- New: `web/app/opengraph-image.tsx`
- New: per-public-page `opengraph-image.tsx` files (~5 of them)
- Layout metadata files: confirm `metadata.openGraph` references resolve

## must-NOT-touch

- Other static assets in /public (out of scope)
- Twitter-card / JSON-LD (sibling brief)

## Acceptance criteria

- [ ] `curl -I http://localhost:3000/opengraph-image` returns 200 + content-type image/png.
- [ ] Slack/LinkedIn/Twitter unfurl test on /landing returns brand-on OG image.
- [ ] Per-page OG images render distinct content per public surface.
- [ ] `pnpm build` succeeds with the new dynamic OG routes.
- [ ] `pnpm typecheck && pnpm lint && pnpm test` clean.

## References

- Audit: `docs/skycot/audit/seo/2026-05-20-wave-2-seo.md`
- Next.js docs: `Image Generation with next/og`
