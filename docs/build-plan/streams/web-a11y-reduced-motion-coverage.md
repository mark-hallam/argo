# web-a11y-reduced-motion-coverage — honour prefers-reduced-motion on 4 more surfaces

**Owner**: claude.
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-a11y-reduced-motion-coverage`.
**Priority**: P2 — wave-2 a11y found 5 surfaces with infinite animations not gated by `prefers-reduced-motion`. Wave-2 fix landed BrandLoudHero; this brief handles the remaining 4.
**Sizing**: ~50-120 LOC (per-surface CSS media query + tests).

## Why this brief exists

From the wave-2 a11y audit:

> 5 surfaces use keyframes/animations not respected by
> `prefers-reduced-motion`. Wave-1 fixed BrandLoudHero. Still
> outstanding:
>
> 1. `<DataSourceChip>` pulse-dot animation
> 2. /audit timeline progress bar shimmer
> 3. /workflows/templates "Use template" CTA shine
> 4. Auth shell radial-blob background animation
> 5. `<TrustRatchetSection>` ring pulse

WCAG 2.1 AA 2.3.3 (Animation from Interactions) requires no animation
exceeding 5 seconds without a way to pause/disable. Several SKYCOT
surfaces fail this.

## Goals

For each of the 4 remaining surfaces, add a `@media (prefers-reduced-motion: reduce)` guard that either:
- Disables the animation (`animation: none !important;`)
- OR holds at a static frame (`animation-iteration-count: 1; animation-play-state: paused;`)

## Steps

1. For each surface: find the animation declaration + add guard.
2. Visual sanity check: with `prefers-reduced-motion: reduce` set in browser dev tools, the animation pauses.
3. Add a test that asserts the CSS-rule presence (snapshot or styled-jsx scan).

## may-touch

- `web/components/skycot/DataSourceChip.tsx` (already gated to dev — still add motion guard for prod symmetry)
- `web/components/skycot/AuditTimeline.tsx` (shimmer animation)
- `web/app/(app)/workflows/templates/page.tsx` (CTA shine)
- `web/components/auth/AuthEditorialShell.tsx` (radial-blob animation if it has one)
- `web/components/skycot/TrustRatchetSection.tsx` (ring pulse)
- Tests

## must-NOT-touch

- BrandLoudHero (already done in wave-2)

## Acceptance criteria

- [ ] All 4 surfaces honour `prefers-reduced-motion: reduce`.
- [ ] Animations either freeze or disable when the preference is set.
- [ ] axe-core scan reports 0 `prefers-reduced-motion` violations.
- [ ] `pnpm typecheck && pnpm lint && pnpm test` clean.

## References

- Audit: `docs/skycot/audit/a11y/2026-05-20-wave-2-a11y.md`
- Pattern: BrandLoudHero `@media (prefers-reduced-motion: reduce)` block (wave-2 fix)
- WCAG 2.1 AA 2.3.3
