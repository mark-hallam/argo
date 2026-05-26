# fix-hire-wizard-step1-label-contrast-wcag

**Owner**: claude or codex.
**Authored against**: origin/main `2cc2ec42b` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-hire-wizard-step1-label-contrast-wcag`.
**Source**: ux-e2e-audit-T2309MEL `onboarding-hire-an-ai-teammate.md` F9 [P2].

🟡 **P2 — `/hire` step 1's "What do you need help with?" label renders as ghosted white-on-white text above the textarea — barely visible against the warm-radial-blob F1 background. WCAG 2.1 AA failure if the measured contrast is below 4.5:1. Camera-visible if Scene 3 of the v1.0 runbook walks the hire wizard.**

## Root cause

The F1 design lock is light page + Audiowide hero in `#1e1b4b` ink. The `WizardEditorialShell` background uses a warm radial gradient blob. Step 1's `<LabelText>` for the role-description input renders in a low-contrast tone (per the audit screenshot, "barely visible above the textarea"). The placeholder text inside the textarea is fine; only the question label above it fails.

Likely cause: the label is using a brand-token color (e.g. `var(--ink-muted)` or `text-slate-400`) that resolves to a light-blue/lavender tone, which against the warm radial blob bg yields a sub-4.5:1 contrast ratio.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F9 [P2] — `/hire` step 1 label "What do you need help with?" low-contrast

## Fix recipe

1. **Audit the contrast.** Open `/hire/step/1` in Chrome dev tools, inspect the `<label>` element above the textarea, copy the computed `color` + the background color at that pixel. Run through an a11y contrast checker (Chrome dev tools' built-in Accessibility tab gives a contrast score).

2. **Bump the label ink.** Per the audit's suggested fix:
   - Change label color from current (likely `text-slate-400` / `var(--ink-muted)`) to `#475569` (slate-600) or the same `#1e1b4b` ink token the heading uses
   - Verify the result clears 4.5:1 against the radial-blob bg at every visible position (the blob has a gradient, so the worst case may be at one corner)

3. **Check sibling wizard steps.** The same `<LabelText>` is likely reused on `/hire/step/{2,3,4,5}` — apply the fix once at the component level (`web/components/skycot/hire/Step1Land.tsx` or shared `web/components/skycot/wizard/LabelText.tsx`).

4. **Add an axe test.** If a vitest+axe setup exists (per `fix-dark-mode-tokens-pricing-compare-docs-marketplace`), add a spec that loads each hire-wizard step and asserts zero contrast violations.

## Acceptance criteria

- [ ] Chrome dev tools Accessibility panel reports the "What do you need help with?" label contrast ratio ≥ 4.5:1
- [ ] Same fix applied across `/hire/step/{1,2,3,4,5}` if shared component
- [ ] axe test (if added) reports zero contrast violations on the hire wizard
- [ ] Visual check: label is comfortably readable at 100% zoom without leaning in
- [ ] `pnpm --dir web exec vitest run components/skycot/hire` green

## May-touch

- `web/components/skycot/hire/Step1Land.tsx` (~3 LOC color edit) OR
- `web/components/skycot/wizard/WizardEditorialShell.tsx` (~3 LOC if shared) OR
- `web/components/skycot/wizard/LabelText.tsx` (~3 LOC if there's a shared label component)
- `web/__tests__/hire-a11y.test.tsx` (~30 LOC new spec)

## Out of scope

- Broader dark-mode contrast issues (`fix-dark-mode-tokens-pricing-compare-docs-marketplace`, `fix-workflows-dark-mode-card-white-on-white`)
- Re-theming the warm-radial-blob bg (locked F1 design)
- Brand-token system rewrite

## Effort

S. ~3 LOC + ~30 LOC test. ~15 min. P2 because the wizard is camera-visible in Scene 3 but the issue is cosmetic — the operator can power through.
