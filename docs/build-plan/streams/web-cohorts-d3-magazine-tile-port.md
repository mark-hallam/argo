# web-cohorts-d3-magazine-tile-port — port locked D3 magazine-tile design to JSX

**Owner**: claude.
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-cohorts-d3-magazine-tile-port`.
**Priority**: P2 — wave-5 viral-moments: /cohorts ships plain `<Card>` instead of the operator-locked D3 magazine-tile editorial layout.
**Sizing**: ~400-600 LOC (page rewrite + 3 tile variants + tests).

## Why this brief exists

From the wave-5 viral-moments audit + operator memory
`project_cohorts_page_design_locked.md`:

> /cohorts magazine-tile layout — LOCKED D3 asymmetric editorial
> layout is unimplemented; ships as plain `<Card><CardHeader>`.
> Fix: lead/stacked/compact asymmetric grid.

> D3 magazine tiles (6 dept tabs + asymmetric lead/stacked/compact
> layout). Locked 2026-05-19; Bloomberg-feature editorial aesthetic,
> 1 lead tile + 2 stacked + 4 compact, brand-loud-editorial register.

## Goals

1. Replace plain Card grid with asymmetric magazine layout:
   - 1 lead tile (big, gradient banner, hero copy)
   - 2 stacked tiles (medium, half-width)
   - 4 compact tiles (small, grid)
2. 6 department tabs across the top (matches operator memory's 7th-tab "Project & Ops").
3. Brand-loud-editorial register: Audiowide on the lead tile, JetBrains Mono department labels, gradient accents.
4. Honour `prefers-reduced-motion` on any tile-animation.

## Steps

1. Read the locked D3 HTML reference under `docs/skycot/design-explorations/.../cohorts/option-D3.html`.
2. Inventory existing /cohorts page structure.
3. Build `<CohortLeadTile>`, `<CohortStackedTile>`, `<CohortCompactTile>` components.
4. Compose into the page with the asymmetric grid.
5. Wire department tabs to filter the tile data.
6. Verify visual matches the locked reference.

## may-touch

- `web/app/(app)/cohorts/page.tsx`
- New: `web/components/skycot/cohorts/{CohortLeadTile,CohortStackedTile,CohortCompactTile}.tsx`
- `web/components/skycot/cohorts/*` existing (may need refactor)

## must-NOT-touch

- /workers/[id]/* (out of scope)
- Cohorts seed data (handled by sibling briefs)

## Acceptance criteria

- [ ] /cohorts renders the asymmetric magazine layout (1 lead + 2 stacked + 4 compact per dept).
- [ ] All 6+ department tabs work + filter tiles.
- [ ] Brand-loud-editorial register: Audiowide + gradient on lead tile.
- [ ] `prefers-reduced-motion` honoured.
- [ ] `pnpm typecheck && pnpm lint && pnpm test` clean.
- [ ] Viral-moments re-audit: /cohorts wow-factor 8+.

## References

- Audit: `docs/skycot/audit/viral-moments/2026-05-20-wave-5-viral-moments.md`
- Memory: `project_cohorts_page_design_locked.md`
