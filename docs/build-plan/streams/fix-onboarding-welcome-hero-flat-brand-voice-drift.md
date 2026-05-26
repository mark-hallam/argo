# fix-onboarding-welcome-hero-flat-brand-voice-drift

**Owner**: claude.
**Authored against**: origin/main `56f4ea0b4d84ebf52c14e387d95c4e99e1cca0b7` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-welcome-hero-flat-brand-voice-drift`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-27-0415/onboarding-hire-an-ai-teammate.md` F6 [P1].

🟠 **P1 — Welcome step hero ("Build your AI workforce" / "In four minutes you'll onboard your first AI teammate.") drops the brand voice the visitor just heard on `/landing` ("Don't buy another tool. Hire a teammate. (Seriously.)"). The visitor took two clicks (Hire-CTA → Sign up → Welcome) and the energy fell off a cliff. Brand-voice continuity from marketing into onboarding is load-bearing for the investor narrative.**

## Root cause

`web/app/(setup)/onboarding/welcome/page.tsx:21-30` renders a generic enterprise hero:

```
H1: "Build your AI workforce"
sub: "In four minutes you'll onboard your first AI teammate."
```

The `/landing` brand register is plain-spoken, conversational, and proudly anti-tool ("Don't buy another tool. Hire a teammate. (Seriously.)" per the locked voice profile at `docs/skycot/marketing/VOICE_PROFILE.md` + memory `user_writing_voice_and_thought_leadership_themes`). The welcome page hero, by contrast, reads like a Salesforce product tile.

For Scene 3 of the v1.0 demo MP4, the investor watches the camera move from `/landing` → `/auth/signup` → `/onboarding/welcome` in roughly 25 seconds. The hero change registers as the product losing confidence between marketing and product surface.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F6 [P1] — "Build your AI workforce" hero is generic enterprise-speak; loses the wizard's plain-English voice

## Fix recipe

1. Rewrite the H1 + sub at `web/app/(setup)/onboarding/welcome/page.tsx:21-30` so the brand register is continuous with `/landing`. Candidates (operator-decisional):
   - H1: `"Let's hire your first teammate."` / sub: `"Four minutes. We'll do most of the talking."`
   - H1: `"Now the fun part."` / sub: `"Four minutes to your first AI colleague."`
   - H1: `"Welcome aboard."` / sub: `"Four minutes. Let's hire your first teammate."`
2. Sync the sub copy with what step 5 actually delivers (per the sibling brief `fix-onboarding-welcome-pillar-vs-step5-promise-mismatch`). If step 5 is still single-textarea today, do NOT promise "Voice or text — your call" in the sub.
3. Keep the eyebrow (currently being rewritten by `fix-onboarding-welcome-eyebrow-step-one-pattern` from `"WELCOME TO SKYCOT"` to `"STEP ONE · WELCOME"`).
4. Verify the H1 still fits the F1 editorial-center layout at 1280-wide (the locked design from memory `project_onboarding_design_locked`).

## Acceptance criteria

- [ ] Welcome H1 reads as continuous with the `/landing` brand voice (plain-spoken, conversational, no "workforce" / "platform" / "solution" enterprise-tile language).
- [ ] H1 + sub fit the F1 editorial-center layout at 1280-wide without forcing a second line on the sub.
- [ ] Eyebrow change from `fix-onboarding-welcome-eyebrow-step-one-pattern` is preserved.
- [ ] Web smoke test (`web/playwright/onboarding/welcome-hero.spec.ts` or extend existing) asserts the H1 text matches the new copy.

## May-touch

- `web/app/(setup)/onboarding/welcome/page.tsx:20-30` (eyebrow + hero copy)
- `web/playwright/onboarding/welcome-hero.spec.ts` (NEW or extend)

## Out of scope

- Welcome pillar #2 copy (covered by `fix-onboarding-welcome-pillar-vs-step5-promise-mismatch`)
- Welcome eyebrow STEP ONE pattern (covered by `fix-onboarding-welcome-eyebrow-step-one-pattern`)
- Genesis Studio framing on the welcome page (covered by `fix-onboarding-welcome-step-genesis-studio-framing`)

## Effort

~15-25 LOC, ~30 min. P1 because Scene 3 lands here on camera; the voice drop is visible.
