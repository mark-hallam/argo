# web-hero-banner-shared-component-refactor — DRY the 3 brand-loud hero banners

**Owner**: claude. **Sizing**: ~200-300 LOC of dedup + 1 new shared component.
**Authored against**: origin/main `3121e32e` on `2026-05-20`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-hero-banner-shared-component-refactor`.
**Priority**: P2 — code hygiene; no user-facing change. Demo audit shipped 3 brand-loud heroes inline (Team, Approvals, Cohorts) using identical 200-line styled-jsx blocks. This refactor extracts a `<BrandLoudHero>` component.

## Scope

Create `web/components/skycot/BrandLoudHero.tsx` exporting a single component:

```tsx
<BrandLoudHero
  eyebrowDotColor="#f0fdf4"
  eyebrowLabel="YOUR AI WORKFORCE · LIVE"
  pulseDot
  title="6 teammates, one team."
  titleAccent="one team"  // optional gradient highlight
  sub="1 working right now · 1,122 actions this week · saving you ~72 hours."
  kpis={[
    { label: "Working", value: "1" },
    { label: "Actions/wk", value: "1,122" },
    { label: "Hours saved", value: "72" },
  ]}
/>
```

Refactor existing call sites:
- `web/app/(app)/team/page.tsx` — `TeamHero` → `<BrandLoudHero>`
- `web/app/(app)/page.tsx` — `ApprovalsHero` → `<BrandLoudHero>`
- `web/app/(app)/cohorts/page.tsx` — `CohortsHero` → `<BrandLoudHero>`

Delete the 3 inline implementations. Net change ≈ -400 LOC.

## Acceptance criteria

- [ ] Visual diff before/after: zero pixel difference on the 3 surfaces.
- [ ] Three styled-jsx blocks deleted; one component added.
- [ ] Component exports a useful TypeScript interface.
- [ ] Existing screenshots (Team / Approvals / Cohorts) still pass any visual regression smoke (if present).

## Out of scope

- Adding hero banner to additional surfaces — separate brief if desired.
