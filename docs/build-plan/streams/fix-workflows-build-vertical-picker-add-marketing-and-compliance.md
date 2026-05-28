# fix-workflows-build-vertical-picker-add-marketing-and-compliance

**Owner**:
**Authored against**: origin/main `354de50a0c5fdf561125c7e931699fe94daa326d` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-workflows-build-vertical-picker-add-marketing-and-compliance`.
**Source**: e2e-ux-audit T0739 — `docs/skycot/ux-audits/2026-05-27-0739/scene-4-and-5-claim-4-and-5.md` F7.

🟡 **P1 — `/workflows/build` vertical picker hardcodes five verticals (sales / support / gtm-engineer / people-ops / finance). The legacy `/verticals` gateway response lists 33 verticals loaded. The cohorts page (`/cohorts`) shows seven department tabs including Marketing and Compliance. Operator hitting "Start from blank" cannot pick those two departments — visible gap for any post-demo customer click-through.**

## Root cause

`web/app/(app)/workflows/build/page.tsx:4-10` hardcodes a 5-element vertical array:

```ts
const VERTICALS = [
  { id: "sales", ... },
  { id: "support", ... },
  { id: "gtm-engineer", ... },
  { id: "people-ops", ... },
  { id: "finance", ... },
];
```

The cohorts source-of-truth (`web/lib/cohorts.ts` or similar; verify via `rg 'Marketing|Compliance' web/lib/`) carries the broader 7-department taxonomy: Engineering, Product, Sales, Customer Success, Marketing, Finance, **Project & Ops** (per memory `project_cohorts_page_design_locked` — Marketing is the 5th dept tab; Compliance may be a sub-vertical inside Finance or its own tab; verify against `web/app/(app)/cohorts/page.tsx`).

The hardcoded vertical array is a v0 scaffold; the canonical source is the gateway `/verticals` envelope, which (post the gateway boot-seed fix) lists ~30+ verticals. The picker should consume the same list — not duplicate it incorrectly.

## Findings this brief closes

- scene-4-and-5-claim-4-and-5 F7 [P1] — `/workflows/build` vertical picker omits two verticals — Marketing and Compliance not selectable

## Fix recipe

1. **Source the vertical list from the gateway.** In `web/app/(app)/workflows/build/page.tsx`, replace the hardcoded array with a fetch via `useVerticals()` (or the existing onboarding `useVerticals` hook in `web/lib/onboarding/use-verticals.ts`). The component renders a skeleton during fetch + the real grid on resolve.
2. **Fallback to a richer hardcoded list** when the gateway 404s in mock-offline mode. Add at minimum Marketing + Compliance to the 5-element fallback array — even if the gateway never responds, the picker covers the 7 dept tabs. Total: 7 fallback entries matching the cohorts taxonomy.
3. **Verify the picker icons + descriptions** are present for the 2 added entries (Marketing, Compliance). Reuse the existing icon/description style of the 5 hardcoded entries.
4. **Vitest pin**: in mock-offline mode, the picker grid renders ≥7 cards. With a real gateway, it renders the full server-side list.

## Acceptance criteria

- [ ] `/workflows/build` renders a vertical picker with at minimum 7 cards (matching the 7 cohorts dept tabs).
- [ ] Marketing and Compliance are selectable.
- [ ] Clicking a vertical advances to the next step in the build flow (no regression).
- [ ] In a workspace with real gateway-loaded verticals, the picker shows the full server list (not the 7-entry fallback).
- [ ] Vitest pin: assert the rendered card count matches the cohorts taxonomy size.

## May-touch

- `web/app/(app)/workflows/build/page.tsx` (replace hardcoded array)
- `web/lib/onboarding/use-verticals.ts` OR `web/lib/api/verticals.ts` (consume existing hook; no shape change expected)
- `web/lib/cohorts.ts` (verify taxonomy alignment)
- `web/app/(app)/workflows/build/__tests__/page.test.tsx` (pin)

## Out of scope

- The gateway `/verticals` boot-seed (covered by `fix-gateway-skills-and-verticals-boot-seed-tenant-resolution.md`)
- The `/workflows/build/marketplace` placeholder if it exists (covered by `fix-workflows-build-vertical-picker-add-marketplace.md` — overlaps; coordinate)
- The Project & Ops dept tab semantics on `/cohorts` (4th position per `web/components/skycot/cohorts/dept-registry.ts:18-50`; covered by `fix-cohorts-dept-registry-design-lock-drift-6-vs-8.md`)

## Effort

S — ~40-60 LOC + pin. ~1 hour. P1 because demo Scene 5 doesn't exercise this surface, but post-demo customer click-through reveals the gap immediately.
