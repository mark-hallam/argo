# fix-onboarding-wizard-shell-demo-mode-banner-pass-through

**Owner**: claude.
**Authored against**: origin/main `b77864a000a9738c80f235fc2605d192197cd5c7` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-wizard-shell-demo-mode-banner-pass-through`.
**Source**: e2e-ux-audit T0819 — `docs/skycot/ux-audits/2026-05-27-0819/onboarding-hire-an-ai-teammate.md` F10 [P1].

🟠 **P1 — Demo mode (`NEXT_PUBLIC_SKYCOT_DEMO_MODE=1`) is active on the wizard but there is no top-level demo-mode banner on the wizard shell. The signup page carries a yellow `DemoModeBanner`; the wizard drops it. A user who entered via deep link never sees the banner. The connect-provider step has its own inline "Demo mode" copy, but that's the only signal — and the user might not connect "Demo mode" to "step 5 will route them to a sample persona" (F1 in the same audit).**

## Root cause

`web/components/auth/WizardEditorialShell.tsx` (the shell wrapping all 5 onboarding steps) does not render `DemoModeBanner`. The signup page at `web/app/auth/signup/page.tsx` does. So as the user transitions signup → wizard, the persistent demo-mode signal disappears even though the demo-mode flag is still on.

Compounding: per memory `feedback_no_theater_in_demo`, the v1.0 demo MP4 should be REAL not mocks. But the launch.json today still ships `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1`, and the demo-mode short-circuit on first-hire (covered by `fix-demo-first-hire-discards-prompt-and-redirects-to-sample-worker`) routes the user to a hardcoded sample worker. Without a banner the user (or investor demo viewer) can't tell which steps are real vs simulated.

The connect-provider step (`web/app/(setup)/onboarding/connect-provider/page.tsx:141-153`) DOES render an honest inline message: "Demo mode — workers run with a mock provider (simulated responses). Type any sk-ant- key to continue; no Anthropic call is made." That's the right vibe — but it's per-step copy, not a shell-level banner.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F10 [P1] — Connect-provider step calls itself "Demo mode" without any visible flag — user doesn't know what changed

## Fix recipe

1. **Add `DemoModeBanner` to `WizardEditorialShell.tsx`.** Render at the top of the shell, above the step-progress strip, conditioned on `isDemoMode()` from `web/lib/skycot/demo-mode.ts`. Match the banner shape used on the signup page.
2. **Banner copy**: "Demo mode — first-hire creates a staged teammate, not a real provisioning. Provider calls are simulated."
3. **Banner is dismissable per session** (matches signup behaviour). Persist dismissal in `sessionStorage` so it stays gone for the duration of the wizard.
4. **Test** in `web/components/auth/__tests__/WizardEditorialShell.test.tsx`: assert `DemoModeBanner` renders when `isDemoMode()` returns true; doesn't render when false.
5. **(Optional) Per-step inline accent**: keep the connect-provider step's inline "Demo mode" copy AND have the shell banner — the shell banner is the wayfinding, the per-step copy is the explanation of what specifically changes for that step.

## Acceptance criteria

- [ ] Visiting any `/onboarding/<step>` page with `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` shows a top-level demo banner above the step strip.
- [ ] Banner is dismissable; dismissal persists across step navigation (sessionStorage).
- [ ] Banner copy makes clear that first-hire is staged + provider is mocked.
- [ ] When `NEXT_PUBLIC_SKYCOT_DEMO_MODE=0` (or unset), the banner is absent.
- [ ] Vitest pin asserts the gate.

## May-touch

- `web/components/auth/WizardEditorialShell.tsx` (add banner)
- `web/components/skycot/DemoModeBanner.tsx` (reuse — verify it accepts the props we need)
- `web/lib/skycot/demo-mode.ts` (no change; reuse `isDemoMode()`)
- `web/components/auth/__tests__/WizardEditorialShell.test.tsx` (new test)

## Out of scope

- Eliminating demo-mode short-circuits (covered by `fix-demo-first-hire-discards-prompt-and-redirects-to-sample-worker`)
- Renaming the env var (covered by `fix-claude-launch-json-env-var-naming-mismatch`)
- Step-3 inline demo-mode copy (already shipped honestly; keep)

## Effort

S — ~30-60 LOC. ~30-60 min. P1 because the v1.0 demo MP4 records the wizard and the camera reads the banner state as a trust signal — without it the demo-mode short-circuits feel like bugs not affordances.
