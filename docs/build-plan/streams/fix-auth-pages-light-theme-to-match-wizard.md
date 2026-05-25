# fix-auth-pages-light-theme-to-match-wizard

**Owner**: claude or codex.
**Authored against**: origin/main `d15461a3a` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-auth-pages-light-theme-to-match-wizard`.
**Source**: e2e-ux-audit Agent (onboarding-hire-an-ai-teammate) finding F9 (`docs/skycot/ux-audits/2026-05-25-2309/onboarding-hire-an-ai-teammate.md`).

🟡 **P1 — `/auth/signup` renders dark; `/onboarding/*` renders light. Brand register breaks under the user mid-flow. Violates the operator-locked auth-pages design (project_auth_pages_design_locked memory).**

## Root cause

`/auth/signup` page renders with `data-theme="skycot-dark"`, dark navy background (`lab(2.73 -0.64 -3.71)`) + purple/cyan/pink Audiowide hero on dark. The wizard immediately after (`/onboarding/welcome`) is light editorial-center per the F1 lock (warm-paper bg, `#1e1b4b` ink).

Operator-locked memories:
- `project_auth_pages_design_locked` — auth pages are editorial-A "brand-medium register, seamless marketing → auth → onboarding feel" (locked 2026-05-19)
- `project_onboarding_design_locked` — F1 editorial-center, light page primary

Today's state — dark auth → light onboarding — is a visible break. Either auth needs to be light editorial-A like the wizard, or the wizard needs a dark variant. The memory says they should match.

Function-vs-dressing: dressing-side. P1 not P0 because no scene narrates the auth page in isolation, but a recording that cuts between them is jarring AND the locked memory is unambiguous.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F9 [P1] — `/auth/signup` is dark; `/onboarding/*` is light. Brand register switches under the user mid-flow

## Fix recipe

1. **Identify the auth-group theme setter**. Likely `web/app/auth/layout.tsx` or `web/components/auth/AuthEditorialShell.tsx`. The ThemeProvider may default to dark for the auth group via `forcedTheme="dark"` or by hardcoding `data-theme="skycot-dark"` on the layout root.

2. **Switch to light** by setting `data-theme="skycot-light"` (or removing the forced override so the global theme provider applies). Match the F1 wizard recipe:
   - Warm radial-blob background
   - `#1e1b4b` ink
   - Violet→pink gradient on the Audiowide hero word
   - Inter weight 200 sub-copy

3. **Apply to all three auth pages**: `/auth/signup`, `/auth/verify`, `/auth/signin` (the locked memory covers all three).

4. **Manual smoke**: navigate marketing `/landing` (light) → click "Hire your first teammate" CTA → `/auth/signup` (now light) → submit → `/onboarding/welcome` (light). No theme flash anywhere.

5. **Pin with a Playwright spec** at `web/tests/auth-pages-light-theme.spec.ts`: for each of the 3 auth routes, assert `<html>` carries the light-theme class (or `data-theme="skycot-light"`).

## Acceptance criteria

- [ ] `/auth/signup`, `/auth/verify`, `/auth/signin` all render light (matching F1 wizard).
- [ ] No visible theme flash when navigating between marketing → auth → onboarding.
- [ ] Audiowide hero gradient + Inter weight 200 + warm bg match the wizard recipe.
- [ ] Playwright spec passes.
- [ ] Operator-side check: visual diff against the locked mockups doesn't reveal new drift.

## May-touch

- `web/app/auth/layout.tsx`
- `web/components/auth/AuthEditorialShell.tsx` (if exists)
- `web/app/auth/signup/page.tsx`, `verify/page.tsx`, `signin/page.tsx` (only if they set their own theme)
- `web/tests/auth-pages-light-theme.spec.ts` (NEW)

## Out of scope

- The signup-router race (separate brief `fix-signup-demo-mint-router-race-bypasses-wizard`)
- The signup hero off-center bug (separate brief `fix-auth-signup-desktop-hero-off-center`)
- Per-tenant theming (roadmap item per `project_multi_tenant_theming`)

## Effort

~20-60 LOC + 1 Playwright spec. ~45 min.
