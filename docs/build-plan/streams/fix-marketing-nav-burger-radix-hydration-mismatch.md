# fix-marketing-nav-burger-radix-hydration-mismatch

**Priority**: P2 — every marketing page shows React's red "1 Issue" dev-overlay badge because the mobile burger menu (a Radix Dialog) generates a different `aria-controls` id on the server vs the client. Cosmetic (the menu works), but the badge is visible on camera during any marketing-page recording, and the warning is real noise that masks future hydration regressions.
**Source**: 2026-05-29 full-site walkthrough (item-4), hydration-mismatch warning captured on `/compare` (reproduces on all `MarketingNav` pages).
**Authored against**: origin/main `ac974cb06` on 2026-05-29
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-marketing-nav-burger-radix-hydration-mismatch`
**Authored by**: claude conductor (interactive session)

## The bug (verified live)

On `/compare` (and every page rendering `components/marketing/MarketingNav.tsx`), the console shows a hydration mismatch:

```
A tree hydrated but some attributes of the server rendered HTML didn't match
the client properties. ...
  <button className="mk-burger" aria-label="Open menu" data-testid="marketing-nav-open"
+   aria-controls="radix-_R_4clrkndrlb_"
-   aria-controls="radix-_R_hineitmrlb_"
```

Root cause: `MarketingNav.tsx:106-116` renders a Radix `Dialog.Root`/`Dialog.Trigger`/`Dialog.Content` for the mobile menu. Radix auto-generates the `aria-controls`/content id via an internal id hook; in this SSR setup the server-rendered id and the client-rendered id differ, so React flags the attribute mismatch and surfaces the dev "1 Issue" badge. (`mk-burger` is `display:none` at desktop width but Radix still mounts the Trigger, so it fires on every page.)

## Scope / fix

Make the Dialog's id deterministic so server and client render the same `aria-controls`. Preferred approach (pick the one that holds on the installed Radix version):

1. **Explicit stable id** — pass an explicit `id` to `RDialog.Content` and the matching `aria-controls` is then stable across SSR/client. Radix `Dialog.Content` accepts an `id`; deriving it from a constant (e.g. `"mk-mobile-menu"`) removes the random suffix entirely. This is the smallest, most robust fix.
2. If Radix's internal wiring ignores an explicit content id for `aria-controls`, gate the random-id surface behind React's `useId()` seeded consistently, or wrap the Trigger so the controlled id is provided explicitly.

Do NOT "fix" this by deferring the menu to a mount-only client render (`useEffect` mounted flag) — that throws away SSR for the nav and is a worse trade than a stable id.

## Acceptance

- [ ] Loading any marketing page (`/landing`, `/compare`, `/pricing`, `/docs`, `/changelog`) produces **zero** hydration-mismatch warnings in the console and **no** red "1 Issue" dev badge.
- [ ] The burger menu still opens/closes and the Dialog still carries the links + sign-in + CTA (no functional regression).
- [ ] `aria-controls` on `.mk-burger` equals the Dialog content's `id` and is identical between SSR HTML and hydrated DOM (assert via the existing `MarketingNav.test.tsx`, or a Playwright check that asserts no hydration warning on `/compare`).
- [ ] `pnpm exec tsc --noEmit` + `pnpm exec next lint` clean for touched files.

## Out of scope

- Any visual restyle of the nav or the mobile sheet.
- Other Radix components elsewhere in the app (only the marketing burger menu is in scope; if the same pattern recurs in-app, a follow-up brief covers it).

## May-touch

- `components/marketing/MarketingNav.tsx` (the `RDialog` block, lines ~106-182)
- `components/marketing/__tests__/MarketingNav.test.tsx` (assertion for stable id / no-mismatch)

## Operator authority

Web-only, ≤5 files — eligible for the web-only admin-merge lane with vitest green in the PR body per the 2026-05-26 operator authorization.
