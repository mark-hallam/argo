# fix-onboarding-wizard-polish-cluster-wrap-scroll-eyebrow

**Owner**: claude.
**Authored against**: origin/main `4b3aa443f` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-wizard-polish-cluster-wrap-scroll-eyebrow`.
**Source**: UX-AUDIT-T0519 2026-05-27 — `docs/skycot/ux-audits/2026-05-27-0519/onboarding-hire-an-ai-teammate.md` F12 + F13 + F14 + F15.

🟦 **P2 — Four small polish defects on the wizard that collectively read as "not quite finished" to the camera. None individually catastrophic; bundled because they sit on the same surface and the fixes are mechanical.**

## Root cause

Four independent micro-defects on the F1 editorial-center onboarding wizard:

1. **F12 — "Finance — Accounts Receivable" wraps onto 3 lines** in the pick-vertical 3-column grid. The em-dash treats as a soft-break boundary, so "Finance —" / "Accounts" / "Receivable" stack across three rows while sibling cards ("GTM Engineer", "Customer Support") are single-line. Visual jitter on the leftmost card.
   - File: `web/app/(setup)/onboarding/pick-vertical/page.tsx:298` (the `.ob-role-name` span); CSS in `OnboardingFormStyles.tsx`.

2. **F13 — Mid-wizard scroll position leaks back to welcome step.** Navigating `workspace` → `welcome` retains the scroll position from workspace's `autoFocus`-triggered `scrollIntoView`. User lands mid-page on welcome; header partially out of view; the Audiowide hero is half-hidden behind the sticky bar.
   - File: each step page in `web/app/(setup)/onboarding/*/page.tsx` lacks a `useEffect(() => window.scrollTo(0, 0), [])` on mount.

3. **F14 — Demo banner stacked above the wizard top bar takes vertical slice on mobile.** Two sticky headers (~34px + ~48px) eat ~70-80px of vertical real estate at 375×812 (mobile demo viewport). Welcome hero pushed below two stacked bars.
   - Files: `web/components/skycot/DemoBanner.tsx` (sticky); `web/components/auth/WizardEditorialShell.tsx` (sticky top bar).

4. **F15 — Eyebrow step-number convention inconsistent.** Steps 1, 2, 3, 5 spell out the number ("STEP TWO · WORKSPACE"). Step 4 uses "STEP FOUR · PICK A ROLE". But the page-internal step strip on every step uses numerals ("STEP 04 OF 05"). Two conventions side-by-side.
   - Files: each step page's `eyebrow` prop in `web/app/(setup)/onboarding/*/page.tsx`.

Camera-visibility per scene: F12 is on Scene 3 (visible 5-10s while operator hovers); F13 visible if operator clicks "Back" during recording; F14 only at mobile viewport (likely not the recording viewport); F15 baseline visible every step.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F12 [P2] — Vertical card "Finance — Accounts Receivable" wraps onto 3 lines
- onboarding-hire-an-ai-teammate F13 [P2] — Mid-wizard navigation can leave the page scrolled to mid-form
- onboarding-hire-an-ai-teammate F14 [P2] — Demo banner takes vertical slice from the wizard top bar
- onboarding-hire-an-ai-teammate F15 [P3] — Eyebrow "STEP TWO · WORKSPACE" vs page-internal "STEP 04 OF 05" inconsistency

## Fix recipe

1. **F12 — Rename seed.** Shorten "Finance — Accounts Receivable" to "Finance · AR" or "Accounts Receivable" in the SEED_VERTICALS array (`web/lib/api/verticals.ts` or wherever the SPA-side fallback lives). Brand voice: terse role labels match siblings ("GTM Engineer", "Sales SDR"). Also fix in the gateway seed catalog so the fix survives the boot-seed sweep (see `fix-gateway-skills-and-verticals-boot-seed-tenant-resolution.md`).

2. **F13 — Add scroll reset.** In `web/components/auth/WizardEditorialShell.tsx` (the shell wrapping every onboarding step), add:
   ```ts
   useEffect(() => {
     window.scrollTo(0, 0);
   }, [pathname]); // re-fire on route change
   ```
   This is one-liner that handles every step. Alternative: per-page `useEffect` but the shell-side fix is cleaner.

3. **F14 — Hide demo banner on `/onboarding/*` routes.** In `web/components/skycot/DemoBanner.tsx` (or the layout that mounts it), gate visibility on `pathname.startsWith('/onboarding')` returning null. The welcome page's own "WELCOME TO SKYCOT" eyebrow already signals demo intent for the recording. Sibling brief `fix-onboarding-shell-demo-mode-pill-affordance.md` covers the in-wizard demo affordance; the banner suppression is its complement.

4. **F15 — Pick numerals OR spellings; default to numerals.** Recommend numerals for scan-ability: "STEP 02 · WORKSPACE", "STEP 03 · CONNECT AI PROVIDER", "STEP 04 · PICK A ROLE", "STEP 05 · YOUR TEAMMATE". Welcome stays "WELCOME TO SKYCOT". Update each step page's `eyebrow` prop.

## Acceptance criteria

- [ ] "Finance — AR" vertical card renders on a single line at 1600px viewport (and the gateway catalog reflects the rename).
- [ ] Clicking "Back" from any onboarding step → previous step lands scrolled to top.
- [ ] Demo banner is NOT rendered on `/onboarding/*` routes (any viewport).
- [ ] All onboarding step eyebrows use numerals consistently ("STEP NN · TITLE").
- [ ] Vitest pin: `WizardEditorialShell` scroll-reset effect fires on pathname change.
- [ ] Visual diff snapshot for the pick-vertical card row (catches the wrap regression).

## May-touch

- `web/app/(setup)/onboarding/*/page.tsx` (4 files — eyebrow strings)
- `web/components/auth/WizardEditorialShell.tsx` (scroll reset)
- `web/components/skycot/DemoBanner.tsx` (pathname gate)
- `web/lib/api/verticals.ts` (seed rename)
- `crates/argo-marketplace/manifests/finance-ar.yaml` (gateway-side seed)
- Snapshot or vitest test

## Out of scope

- Onboarding-step page metadata titles (covered by `fix-onboarding-step-pages-tab-title-via-layout-metadata.md`)
- Full demo-banner copy/positioning rewrite (covered by `fix-demo-mode-banner-investor-safe-copy.md` + `fix-demo-banner-sidebar-overlap-desktop.md`)
- Audiowide hero typography (locked design — leave alone)

## Effort

S — ~30-60 LOC across 4 files. ~45-90 min.
