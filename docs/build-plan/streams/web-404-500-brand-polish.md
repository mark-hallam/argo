# web-404-500-brand-polish — apply D3 brand recipe to existing 404 + 500 pages

**Owner**: claude.
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-404-500-brand-polish`.
**Priority**: P3 — wave-4 404/500 audit: existing not-found.tsx + error.tsx score 3/10 brand-on (no Audiowide, no gradient — skipped from D3 pass).
**Sizing**: ~100-200 LOC (refactor existing pages + ErrorShell variant + tests).

## Why this brief exists

From the wave-4 404/500 audit (`docs/skycot/audit/404-500/2026-05-20-wave-4-404-500.md`):

> Custom 404: YES (`web/app/not-found.tsx`). Brand-on score **3/10**
> — uses Inter not Audiowide, no purple→pink gradient, no warm
> radial-blob bg, generic SVG. Skipped from the operator's locked
> D3 brand-consistency pass.
>
> Custom 500: YES (`web/app/error.tsx`). Brand-on score **3/10**
> (same reasons).

These pages render when something breaks — the worst possible time
for a generic-SaaS look on camera. Apply Audiowide + gradient + warm
radial-blob to match the rest of the locked design.

## Goals

1. /not-found.tsx and /error.tsx render Audiowide hero ("Lost in the canvas." or similar) + purple→pink gradient on accent + warm `#fafaff` bg + `#1e1b4b` ink.
2. `<ErrorShell>` component (already exists) gets a brand-loud variant.
3. Keep recovery actions: 404 → "Back home", 500 → "Try again" + "View status".
4. Honour `prefers-reduced-motion`.

## Steps

1. Read existing /not-found.tsx, /error.tsx, /global-error.tsx (wave-5 added the latter — it's already brand-on).
2. Refactor not-found.tsx + error.tsx to match the global-error.tsx brand recipe.
3. Optionally promote the inline-styles approach used in global-error.tsx into ErrorShell so all 3 share the same brand surface.
4. Visual sanity: hit /404 + simulate a throw + verify both render brand-on.

## may-touch

- `web/app/not-found.tsx`
- `web/app/error.tsx`
- `web/components/error/ErrorShell.tsx`
- Tests

## must-NOT-touch

- `web/app/global-error.tsx` (already brand-on from wave-5)
- Other per-route error.tsx (handled by `web-error-boundaries-route-segment-coverage`)

## Acceptance criteria

- [ ] /not-found.tsx renders Audiowide hero + gradient + warm bg.
- [ ] /error.tsx (root) renders same.
- [ ] Reduced-motion guard on any animation.
- [ ] Recovery actions (Back home / Try again / Contact support) work.
- [ ] `pnpm typecheck && pnpm lint && pnpm test` clean.

## References

- Audit: `docs/skycot/audit/404-500/2026-05-20-wave-4-404-500.md`
- Pattern: `web/app/global-error.tsx` (wave-5 — brand-on reference)
