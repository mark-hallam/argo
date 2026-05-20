# web-error-boundaries-route-segment-coverage — add error.tsx to high-risk route segments

**Owner**: claude.
**Authored against**: origin/main `546e3072` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-error-boundaries-route-segment-coverage`.
**Priority**: P1 — wave-4 error-boundaries audit found only 1 of 58 route segments has its own `error.tsx`. Single throw anywhere in those 57 segments shows the bare Next.js default error screen — instant credibility loss on camera. The wave-5 fix-cycle added `global-error.tsx` + `(app)/workflows/.../build/error.tsx`; this brief covers the remaining ~10 high-risk segments.
**Sizing**: ~200-300 LOC (~10-20 LOC per `error.tsx`, one per route segment; mostly reuse the existing `<ErrorShell>` component).

## Why this brief exists

From the wave-4 error-boundaries audit (`docs/skycot/audit/error-boundaries/2026-05-20-wave-4-error-boundaries.md`):

> **Route segments WITHOUT `error.tsx`**: **57** (out of 58 with
> `page.tsx`). … The single highest-leverage fix is **adding
> web/app/global-error.tsx + web/app/(app)/workflows/[v]/[n]/build/error.tsx**
> before tomorrow's recording — both reuse `<ErrorShell />`, ~30
> minutes total work, eliminate the two most-likely on-camera
> white-screen paths.

The wave-5 fix-cycle landed those 2. This brief tackles the next 10
highest-risk segments. After this brief lands, the demo path is
end-to-end error-bounded; the remaining low-priority segments can
be a clean-up sweep in v1.x.

## Goals

Add `error.tsx` (route-segment error boundary) to ~10 high-risk segments. Each one:
- Is `"use client"` per Next.js requirement.
- Reuses `<ErrorShell variant="server-error" />` from `web/components/error/ErrorShell.tsx` for the visual surface.
- Logs the caught error to `console.error` with a route tag (so error-reporting can attribute).
- Offers a `reset` action (retry) + a `<Link>` back to safety (e.g. `/team` or `/`).

## Route segments to cover (priority order)

1. `web/app/(app)/workers/[workerId]/error.tsx` — fetch-heavy, 9 sub-tabs each load worker data.
2. `web/app/(app)/audit/error.tsx` — fetch-heavy + hero surface.
3. `web/app/(app)/audit/action/[id]/error.tsx` — detail drill-in.
4. `web/app/(app)/audit/replay/[id]/error.tsx` — replay stream can throw on bad data.
5. `web/app/(setup)/onboarding/error.tsx` (root segment for /onboarding/*).
6. `web/app/(hire)/hire/error.tsx`.
7. `web/app/auth/error.tsx` (auth flow root).
8. `web/app/(app)/cohorts/error.tsx`.
9. `web/app/(app)/skills/error.tsx`.
10. `web/app/(app)/settings/error.tsx`.

## Steps

1. Read `web/app/error.tsx` and `web/app/(app)/workflows/[verticalId]/[name]/build/error.tsx` as reference patterns.
2. Create the 10 listed error.tsx files (small file per segment).
3. Each one uses `<ErrorShell />` with route-appropriate copy and reset/recovery actions.
4. Add a single test that simulates an error thrown in the route's `page.tsx` and asserts the ErrorShell renders.

## may-touch

- 10 new `error.tsx` files at the paths above.
- Optional: `web/components/error/ErrorShell.tsx` if a prop tweak helps reuse.

## must-NOT-touch

- Existing `error.tsx` files (root + builder) — they already work.
- `web/components/error/ErrorShell.tsx` major surgery — this brief is additive, not a refactor.

## Acceptance criteria

- [ ] 10 new `error.tsx` files at the 10 listed segments.
- [ ] Each reuses `<ErrorShell />` and follows the existing root `error.tsx` pattern.
- [ ] Each catches the error, logs to console with a route tag, exposes a reset action.
- [ ] tsc clean, lint clean, at least 1 test proving the boundary catches a thrown error.
- [ ] Wave-4 error-boundaries audit re-runs at: route segments WITH error.tsx ≥ 13 (was 1; +2 in wave-5 = 3; +10 here = 13).

## References

- `docs/skycot/audit/error-boundaries/2026-05-20-wave-4-error-boundaries.md` (the motivating audit)
- `web/app/error.tsx` (root, the reference pattern)
- `web/app/(app)/workflows/[verticalId]/[name]/build/error.tsx` (wave-5 addition, another reference pattern)
