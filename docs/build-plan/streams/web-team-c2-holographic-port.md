# web-team-c2-holographic-port — port the locked C2 trading-card design to JSX

**Owner**: claude.
**Authored against**: origin/main `546e3072` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-team-c2-holographic-port`.
**Priority**: P1 — closes the wave-5 viral-moments gap (`/team` should be a viral-screenshot moment per the locked C2 design but currently ships a flat list).
**Sizing**: ~400-600 LOC (one big component + per-card props + animation CSS + tests).

## Why this brief exists

The operator-locked Workers-page design is **C2-holographic-showcase**
(per the operator memory `project_workers_page_design_locked.md`):
trading-card deck with gradient banners, animated shine on hover,
huge weight-900 level numbers, brand-loud register. The wave-5
viral-moments audit found:

> `/team` workers grid — `TeammateCard.tsx` is a flat list card;
> the LOCKED C2-holographic-showcase design (trading-card extravaganza
> with gradient banners, huge level numbers, shine sweep on hover)
> is unimplemented. **Fix:** port the locked HTML reference to JSX.

> Core finding: the viral problem is not "we don't have viral
> moments" — it's that 3 of the loudest-promised pages (`/team`,
> `/cohorts`, `/hire/done`) ship the restrained equivalent of their
> LOCKED brand-loud designs. Loud hero + quiet card grid is more
> disappointing than uniformly quiet.

## Goals

1. Replace `web/components/skycot/TeammateCard.tsx` with a C2-holographic-showcase card.
2. Keep the existing `TeammateGrid` outer layout (works with the new card).
3. Hover state: animated shine sweep (reuse the BrandLoudHero shine timing/cubic-bezier).
4. Gradient banner per card (purple→pink, NOT purple→cyan).
5. Big weight-900 level number visible (e.g. "L2", "L4").
6. Per-card trait chips (Patient / Detail-oriented / etc. — already in worker data).
7. Honour `prefers-reduced-motion` (no animated shine when set).

## Steps

1. Find the locked C2 HTML reference (likely under `docs/skycot/design-explorations/.../workers/option-c2.html` or similar).
2. Inventory the design tokens used (gradient stops, font sizes, animation timings).
3. Refactor `TeammateCard.tsx` to the new design.
4. Add a `prefers-reduced-motion` guard (mirror BrandLoudHero pattern).
5. Verify visual diff: open `http://localhost:3000/team` and screenshot before/after.
6. Update or add tests for the new card surface (level number visible; trait chips render; hover state).

## may-touch

- `web/components/skycot/TeammateCard.tsx` (the main refactor)
- `web/components/skycot/TeammateCardSkeleton.tsx` (match the new card shape)
- `web/components/skycot/TeammateGrid.tsx` (only if grid spacing needs adjust)
- `web/components/skycot/__tests__/TeammateCard.test.tsx` (or create)
- Optional: `web/lib/sample-team.ts` if level numbers need adding

## must-NOT-touch

- `web/app/(app)/team/page.tsx` page wrapper (the card replacement is internal)
- `BrandLoudHero` (the hero stays as-is)
- Other locked designs (audit, approvals, settings)

## Acceptance criteria

- [ ] `TeammateCard` renders the C2-holographic-showcase design per the locked HTML reference.
- [ ] Gradient banner uses canonical #8b5cf6 → #ec4899 (NOT #0891b2).
- [ ] Animated shine sweep on hover (or static when `prefers-reduced-motion`).
- [ ] Weight-900 level number visible.
- [ ] Trait chips render from worker data (Mariah: Patient / Detail-oriented / Picture-perfect listings / Polite).
- [ ] tsc clean, lint clean, tests pass.
- [ ] Viral-moments re-audit: `/team` wow-factor moves from current ~5 to 9.

## References

- `~/.claude/projects/-Users-operator-code-argo/memory/project_workers_page_design_locked.md` (the locked design memory)
- `docs/skycot/audit/viral-moments/2026-05-20-wave-5-viral-moments.md` (the motivating audit)
