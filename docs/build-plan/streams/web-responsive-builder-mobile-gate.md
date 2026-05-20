# web-responsive-builder-mobile-gate — desktop-only gate for /workflows/.../build below 1024px

**Owner**: claude.
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-responsive-builder-mobile-gate`.
**Priority**: P2 — wave-2 responsive: workflow builder is unusable below 1024px (React Flow + 360px-fixed panel). Operator-locked design says "edit on desktop"; surface that explicitly.
**Sizing**: ~150-300 LOC (gate component + read-only fallback view + tests).

## Why this brief exists

From the wave-2 responsive audit:

> /workflows/[v]/[n]/build @ 375px — React Flow + 360px-fixed
> panel, zero media queries.
>
> /workflows/[v]/[n]/build @ 768px — same problem, panel covers
> half the iPad.
>
> Page that should be desktop-only gated: ONLY
> /workflows/[v]/[n]/build. Add a `<MobileGate>` rendering a
> read-only step list + "Open on desktop to edit" CTA below 1024px.

The visual canvas needs space; phones and small tablets can't host
it. But users on mobile should see SOMETHING useful — at minimum
the step list (which already renders on the viewer route).

## Goals

1. `<MobileBuilderGate>` component: at <1024px, renders the workflow's step list (read-only) + a polished "Open this on desktop to edit" CTA + a button to share-back-to-desktop (copies the URL).
2. The component swaps in via CSS media query (no JS-only viewport detection — survives SSR).
3. Pre-fetch the step data so the read-only view is instant.

## Steps

1. Build `<MobileBuilderGate>` under `web/components/skycot/workflows/`.
2. Add CSS `@media (max-width: 1023px) { .builder-canvas { display: none; } .mobile-gate { display: block; } }`.
3. Inside the gate: list workflow steps (reuse the viewer's step list component if possible).
4. CTA: "Open on desktop to edit" + "Copy link" button (clipboard.writeText).
5. Test: at 375px / 768px, gate renders + step list visible + CTA works.

## may-touch

- New: `web/components/skycot/workflows/MobileBuilderGate.tsx`
- `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx` (insert the gate)
- Optional: shared step-list component if not yet abstracted
- Tests

## must-NOT-touch

- React Flow canvas itself
- Other workflow pages
- The wave-2 mobile table-scroll wraps on /compare + /pricing (already shipped)

## Acceptance criteria

- [ ] Below 1024px: canvas hidden, step list + CTA visible.
- [ ] At ≥1024px: canvas renders as before.
- [ ] "Copy link" button copies the current URL.
- [ ] `pnpm typecheck && pnpm lint && pnpm test` clean.
- [ ] Visual sanity on 375px / 768px / 1024px / 1440px.

## References

- Audit: `docs/skycot/audit/responsive/2026-05-20-wave-2-responsive.md`
