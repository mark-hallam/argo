# web-storybook-core-ui-coverage

**Owner**: codex.
**Depends on**: none.
**Authored against**: origin/main `8da732a7d` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-storybook-core-ui-coverage`.

🟢 **P3 DESIGN-SYSTEM COVERAGE.** `docs/build-plan/PRODUCT_ROADMAP.md` still
lists "Storybook stories per primitive" as the next thin slice for locking the
design language. The current story surface is narrow and does not pin the core
UI primitives workers keep reusing across the demo dashboard pages.

## Source

- `docs/build-plan/PRODUCT_ROADMAP.md` §Track B/C Storybook notes.
- Current anchors under `web/components/ui/`, `web/components/skycot/`, and
  `web/stories/`.

## Scope

- Add Storybook stories for core primitives that already exist:
  `Button`, `Card`, `StatusPill`, and `BrandLoudHero`.
- Cover meaningful states and sizes rather than a single happy-path render.
- Keep stories as visual-contract fixtures; do not redesign the components.

## May-touch

- `web/stories/`
- `web/components/ui/Button.tsx`
- `web/components/ui/Card.tsx`
- `web/components/ui/StatusPill.tsx`
- `web/components/skycot/BrandLoudHero.tsx`
- `web/package.json` / Storybook config only if an existing script or import
  path prevents stories from building.
- `docs/build-plan/BUILD_STATUS.md` — standard row flip only.

## Must not touch

- Application routes.
- Backend crates.
- Design-exploration source HTML.
- Component visual design beyond tiny prop/export fixes required for stories.

## Acceptance criteria

- [ ] Storybook renders stories for `Button`, `Card`, `StatusPill`, and
      `BrandLoudHero`.
- [ ] Button stories cover primary/secondary/destructive/disabled/loading or
      the equivalent variants present in the component.
- [ ] StatusPill stories cover every supported status variant.
- [ ] Card stories cover dense content and action/footer use if supported.
- [ ] BrandLoudHero story uses real props from the component and avoids
      decorative placeholder copy.
- [ ] `pnpm -C web storybook:build` or the repo's nearest Storybook build
      command succeeds; if no build script exists, run `pnpm -C web typecheck`
      and document the missing script.
- [ ] Every modified file stays under 500 LOC.

