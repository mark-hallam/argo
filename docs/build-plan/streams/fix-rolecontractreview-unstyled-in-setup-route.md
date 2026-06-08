# fix-rolecontractreview-unstyled-in-setup-route

**Owner**: claude.
**Authored against**: origin/main `d0ee42aee` on `2026-06-08`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-rolecontractreview-unstyled-in-setup-route`.

## Why (evidence-grounded — deploy-readiness run 2026-06-08, BUG-1)
The hire review screen (`RoleContractReview`, the "here is exactly what this teammate can do" panel) renders **UNSTYLED** under the `(setup)` route group (the onboarding/first-hire flow). Browser probe: `rcStyleTagPresent:false`, `className` is the bare `rc-action`, no styled-jsx hash in the DOM. The SAME component renders correctly on `/genesis` in the `(app)` route group. So `(setup)` is missing the styled-jsx injection context (`StyleRegistry` / `useServerInsertedHTML`) that `(app)` has, OR the `.rc-*` rules need to live in `globals.css` so they apply in every route group regardless of styled-jsx. This is the first screen a new customer sees when hiring — it must look right.

## Grep-before-write (confirm the root cause at claim time)
- `rg -n 'rc-action|rc-cta|RoleContractReview|style jsx' web/components web/app` — where the `.rc-*` CSS lives and how it's delivered.
- Compare the `(app)` layout vs the `(setup)` layout for a styled-jsx registry: `rg -n 'useServerInsertedHTML|StyleRegistry|styled-jsx' web/app/\(app\) web/app/\(setup\)`.
- NOTE: an earlier fix moved some `.rc-*` CSS into `web/app/globals.css` precisely so it works in all route groups (styled-jsx needs an injection context + "use client"). Confirm whether the review screen's styles are fully in `globals.css`; if a styled-jsx `<style jsx>` block remains the source of truth and isn't injecting under `(setup)`, EITHER add the registry to the `(setup)` layout OR move the remaining `.rc-*` rules to `globals.css`. Prefer the `globals.css` route (no per-route-group fragility) unless that regresses `(app)`.

## Files you may touch
- `web/app/(setup)/layout.tsx` (add the styled-jsx registry if that's the chosen fix), and/or
- `web/app/globals.css` + the `RoleContractReview` component (move `.rc-*` rules to globals.css; keep the brand-locked styling identical).

## Acceptance (truth-bounded)
1. The review screen on `/onboarding/first-hire` (or the `(setup)` hire path) renders with the SAME styling as `/genesis` — the `.rc-*` rules apply (capture the rendered HTML showing the style is present, or a before/after screenshot).
2. `/genesis` (in `(app)`) still renders correctly — no regression.
3. `pnpm -C web exec tsc --noEmit` = 0; touched vitest green.
4. Brand identity unchanged (purple→pink gradient, fonts) — presentation fix only.

## On ship (same commit)
Note in the PR it clears deploy-readiness BUG-1.

## Effort
S (CSS delivery fix). styled-jsx requires `"use client"` — do not remove it; verify the route renders (HTTP 200) after the change.
