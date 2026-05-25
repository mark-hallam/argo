# fix-auth-signup-desktop-hero-off-center

**Owner**: claude or codex.
**Authored against**: origin/main `0653c478d` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-auth-signup-desktop-hero-off-center`.
**Source**: e2e-ux-audit Agent 1 finding F6 (`docs/skycot/ux-audits/2026-05-25-1100-onboarding-hire-an-ai-teammate.md`).

🟡 **P2 polish — `/auth/signup` desktop hero is right-of-center at 1600×900. Visually unbalanced for any demo screencap.**

## Root cause

At native preview width (~1600×900) the "Hire your first AI teammate" hero on `/auth/signup` sits in the right half of the page; the left half is blacked-out (dark mode background). The editorial-center column appears to be positioned with coordinates that assume a different viewport. The mobile (375) layout reflows correctly.

NOT broken — content is reachable, just visually unbalanced. The file at issue is `web/components/auth/WizardEditorialShell.tsx` — likely `wiz-stage` is not `max-width: 720px; margin: 0 auto;` at the widest breakpoint.

Memory `project_auth_pages_design_locked` and `project_onboarding_design_locked` BOTH lock the F1 editorial-center pattern — this implementation drifted away from that.

## Findings this brief closes

- Agent 1 F6 [P2] — `/auth/signup` desktop hero is right-of-center (off-screen left at 1600px)

## Fix recipe

1. Open `web/components/auth/WizardEditorialShell.tsx` and confirm the stage container's responsive styles.
2. Apply at the desktop breakpoint:
   ```css
   .wiz-stage {
     max-width: 720px;
     margin: 0 auto;
     padding: 0 24px;
   }
   ```
   (Use the brand's canonical container width — confirm against `web/components/skycot/onboarding/WizardEditorialShell.tsx` for the analog onboarding shell which Agent-1 reports renders correctly. They're meant to match.)
3. Add a Playwright (or vrt) screenshot test at 1600×900 pinning the centered layout.

## Acceptance criteria

- [ ] `/auth/signup` at 1600×900 renders the hero column visually centered (equal margin left and right).
- [ ] Visual diff vs `/onboarding/welcome` at the same viewport: hero columns occupy the same x-coordinates.
- [ ] Mobile (375) and tablet (768) layouts unchanged.
- [ ] Smoke test (Playwright screenshot or component test) pins the centering.

## May-touch

- `web/components/auth/WizardEditorialShell.tsx`
- `web/components/skycot/onboarding/WizardEditorialShell.tsx` (cross-check; should NOT need a change)
- Test file

## Out of scope

- Other auth pages (`/auth/verify`, `/auth/signin`) — verify they're fine in the test sweep but don't refactor unless they also drift.
- Onboarding wizard chrome refactor (separate brief: `fix-onboarding-hire-parallel-wizards-unify-or-document`)

## Effort

~10-30 LOC + 1 screenshot test. ~30-45 min.
