# web-perf-fonts-consolidation — consolidate 19 duplicate Google-Fonts `<link>` blocks via next/font

**Owner**: claude.
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-perf-fonts-consolidation`.
**Priority**: P2 — wave-2 perf audit found 19 duplicate Google-Fonts `<link>` tags across 13 pages + 6 components. Eliminates ~18 render-blocking roundtrips on the demo path + self-hosts the fonts (no third-party DNS+TLS).
**Sizing**: ~150-250 LOC (centralize next/font, delete 19 dup link blocks).

## Why this brief exists

From the wave-2 perf audit:

> 19 duplicate `<link rel="stylesheet"
> href="fonts.googleapis.com/css2?family=Audiowide..."` blocks
> across 13 page files and 6 components — each one is a
> render-blocking external request. Centralize via `next/font/google`
> in `app/layout.tsx`; self-host the font; delete the 19 duplicates.
> Single change eliminates ~18 round-trips on the demo path.

## Goals

1. Add Audiowide + JetBrains Mono via `next/font/google` in `web/app/layout.tsx` (Inter already loaded that way).
2. Expose CSS variables: `--font-display-audiowide`, `--font-mono-jetbrains` (mirroring the existing `--font-sans-loaded` + `--font-mono-loaded`).
3. Delete the 19 duplicate `<link>` blocks across pages and components.
4. Any inline `font-family: "Audiowide"` or `"JetBrains Mono"` references swap to `var(--font-display-audiowide)`.

## Steps

1. Update `web/app/layout.tsx` to load Audiowide + JetBrains Mono via `next/font/google` (alongside the existing Inter + JetBrains-Mono).
2. Verify the html element gets the new font-family CSS variables.
3. Grep for `fonts.googleapis.com/css2?.*Audiowide` and remove each `<link>` block.
4. Grep for `font-family.*"Audiowide"` and swap inline references to `var(--font-display-audiowide)`.
5. Smoke test: visit /landing, /team, /workflows, /workers/wkr_01J0MARIAH/overview — Audiowide hero typography still renders correctly.
6. Verify network panel: zero `fonts.googleapis.com` requests; fonts served from /_next/static/media/.

## may-touch

- `web/app/layout.tsx` (next/font config)
- Every page.tsx with a Google-Fonts `<link>` (delete the block)
- Every component with the same (delete)
- Global CSS (only if a token consolidation helps)

## must-NOT-touch

- Inter font loading (already done correctly via next/font)
- Per-page Audiowide use in JSX (the className stays; the source changes from Google CDN to next/font)

## Acceptance criteria

- [ ] Zero `fonts.googleapis.com` references in `web/app/` and `web/components/`.
- [ ] Audiowide + JetBrains Mono loaded via next/font in `app/layout.tsx`.
- [ ] `pnpm build` succeeds; bundle has the self-hosted fonts.
- [ ] Demo-path pages still render Audiowide on heroes (visual diff null).
- [ ] Lighthouse perf score on /landing improves by ≥10 points (measure before/after).
- [ ] `pnpm typecheck && pnpm lint && pnpm test` clean.

## References

- Audit: `docs/skycot/audit/perf/2026-05-20-wave-2-perf.md`
- Next.js docs: `next/font/google` API
