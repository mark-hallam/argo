# web-responsive-hero-font-floor — mobile font floor for Audiowide hero headings

**Owner**: claude.
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-responsive-hero-font-floor`.
**Priority**: P3 — wave-2 responsive: 48px Audiowide heroes (skills, compare) have no mobile font floor — overflow horizontally on 375px.
**Sizing**: ~50-100 LOC (CSS clamp() per hero + tests).

## Why this brief exists

From the wave-2 responsive audit:

> Secondary findings worth flagging: `skills-hero` 48px Audiowide
> and `compare-hero` 48px Audiowide have no mobile font floor —
> can overflow horizontally on 375px screens.

Audiowide is wide-character-width; 48px at 375px viewport easily
overflows even a short headline.

## Goals

1. Audit every Audiowide hero across marketing + product pages.
2. Replace fixed `font-size: 48px` with `font-size: clamp(28px, 8vw, 56px)` (or similar — choose values per surface).
3. Verify no overflow at 375px on every Audiowide hero.

## Steps

1. `rg "font-family.*Audiowide" web/` → list all callsites.
2. For each, check the current font-size declaration.
3. Replace with `clamp(min, vw-based, max)` matching the surface (heroes can be larger than eyebrows).
4. Visual sanity at 375 / 768 / 1024 / 1440 px.

## may-touch

- Every page or component using Audiowide hero styling
- `web/app/globals.css` only if a shared hero token would help

## must-NOT-touch

- Audiowide loading itself (handled by `web-perf-fonts-consolidation` brief)
- Non-Audiowide text (only the wide-character hero font has this overflow problem)

## Acceptance criteria

- [ ] No Audiowide hero overflows horizontally at 375px.
- [ ] At ≥1440px, heroes look the same as before (max clamp value preserves desktop sizing).
- [ ] `pnpm typecheck && pnpm lint && pnpm test` clean.
- [ ] Visual sanity at 4 breakpoints.

## References

- Audit: `docs/skycot/audit/responsive/2026-05-20-wave-2-responsive.md`
