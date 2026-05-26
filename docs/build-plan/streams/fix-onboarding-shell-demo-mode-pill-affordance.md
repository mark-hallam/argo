# fix-onboarding-shell-demo-mode-pill-affordance

**Owner**: claude.
**Authored against**: origin/main `56f4ea0b4d84ebf52c14e387d95c4e99e1cca0b7` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-shell-demo-mode-pill-affordance`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-27-0415/onboarding-hire-an-ai-teammate.md` F18 [P1].

🟠 **P1 — The 5-step onboarding wizard pages do NOT show the "Demo mode — sample data" banner that mounts inside `(app)/layout.tsx`. The wizard lives under `(setup)/layout.tsx` which does not include the banner. So a wizard-time demo recording has no on-screen indicator of which mode it's in, and the operator's only pre-record check is "did I export the env var".**

## Root cause

`web/components/skycot/DemoBanner.tsx` is rendered from `web/app/(app)/layout.tsx` (the signed-in app shell), not from `web/app/(setup)/_OnboardingShell.tsx` (the wizard shell). The `(setup)` route group has its own layout that does not include the banner.

For the v1.0 demo MP4, Scene 3 records the wizard pages — and the recording is intended to be in demo mode (per `.claude/launch.json`). The operator currently cannot visually confirm demo-mode is active during the wizard; the banner only appears once they cross into `(app)/...` after Hire-them.

This is operator-side tooling (the camera doesn't need to see "DEMO" in the wizard — and arguably shouldn't, for the investor pitch). But the operator needs a confidence cue during the pre-record dress rehearsal.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F18 [P1] — No "Demo mode" / "Live mode" affordance, so the operator can't tell which path they're on during the wizard

## Fix recipe

1. **Add a small mode pill** to `web/app/(setup)/onboarding/_OnboardingShell.tsx` (verify path) that renders only when `isDemoMode()` returns true.
2. **Visual treatment**: a low-key footer pill in the wizard shell's bottom-right corner, ~32px tall, brand-restrained (muted amber background, single-line text "DEMO MODE · sample data only"). Do NOT use the gradient banner that mounts on `(app)/*` pages — Scene 3 is on camera and the gradient banner would compete with the F1 editorial-center hero.
3. **Reuse the helper** `isDemoMode()` from `web/lib/onboarding/demo-mode.ts` (or the canonical location).
4. **Add a corresponding `data-testid="onboarding-demo-pill"`** so the demo runbook pre-record checklist can grep for it programmatically.
5. **Document in the demo runbook** `docs/skycot/demos/v1-mission-mvp-runbook.md`: pre-record checklist step "DEMO pill visible at bottom-right of the wizard shell — if absent, demo mode is off, check `.claude/launch.json` env vars".

## Acceptance criteria

- [ ] Opening `/onboarding/welcome` in demo mode shows a bottom-right pill reading "DEMO MODE · sample data only".
- [ ] Opening any onboarding step without demo mode set shows NO pill.
- [ ] The pill does not interfere with the F1 editorial-center layout (no overlap with hero or CTAs at 1280-wide).
- [ ] Demo runbook pre-record checklist updated with the visual cue.
- [ ] Web smoke test asserts the pill's presence under demo mode and absence otherwise.

## May-touch

- `web/app/(setup)/onboarding/_OnboardingShell.tsx` (add the pill render)
- `web/components/skycot/onboarding/OnboardingDemoPill.tsx` (NEW, ~20 LOC)
- `web/lib/onboarding/demo-mode.ts` (verify the helper exports `isDemoMode()`)
- `docs/skycot/demos/v1-mission-mvp-runbook.md` (pre-record checklist addendum)
- `web/playwright/onboarding/demo-pill.spec.ts` (NEW)

## Out of scope

- Sidebar live-vs-demo stamp (covered by `fix-sidebar-live-vs-demo-mode-stamp` — that brief addresses `(app)/*` shells)
- Demo banner copy or suppression on signed-in pages (covered by `fix-demo-banner-copy-branch-on-data-source-or-suppress`)
- Auth-bypass vs demo-mode disambiguation (separate concern)

## Effort

~50-80 LOC + 1 Playwright spec + 1 runbook line. ~2 hours. P1 because operator-facing tooling matters for recording reliability.
