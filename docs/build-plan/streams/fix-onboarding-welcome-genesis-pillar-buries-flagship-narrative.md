# fix-onboarding-welcome-genesis-pillar-buries-flagship-narrative

**Owner:** either
**Authored against:** origin/main `410e6ed1240d26f9ab6b48eb94a2fdf177c38270` on 2026-05-28
**Validate before claim:** `bash scripts/check-brief-freshness.sh fix-onboarding-welcome-genesis-pillar-buries-flagship-narrative`
**Source:** ux-audit-2026-05-27-2351

🟢 **P3 — Welcome step's Genesis pillar copy reads "Voice and SOP uploads coming next" — buries Genesis as a stub when per operator memory it's the investor-narrative flagship "AI Teammate Onboarding Studio".**

## Root cause

Per memory `feedback_genesis_is_ai_teammate_onboarding_studio` (operator-clarified 2026-05-25T11:40Z):

> Genesis takes voice/text role + SOP-PDF/wiki/Slack uploads + access-controls + comprehension verification. The investor moment that distinguishes Skycot from "ChatGPT with prompts". Phase 1 (text-only) authored; Phase 2 (multi-modal onboarding) pending operator alignment.

The welcome step's middle pillar currently reads:

> "Describe the role in plain English; Skycot synthesises charter, action vocabulary, memory schema, and eval suites. Voice and SOP uploads coming next."

The phrasing buries the lede — "coming next" implies a stub, when in fact Phase 1 is text-only-but-real. For the investor narrative, this pillar should read like the flagship.

— `onboarding-hire-an-ai-teammate.md` F17 + F18

The "Try Genesis studio →" link below the Get Started CTA is also rendered as a small underlined-on-hover link — easy to miss. If Genesis is the investor-narrative flagship, this should be at minimum a tertiary button or a more prominent CTA chip.

## Findings this brief closes

- `onboarding-hire-an-ai-teammate.md` F17 [P3] — Genesis pillar on welcome step still reads "Voice and SOP uploads coming next" — operator memory says Genesis IS the AI teammate onboarding studio
- `onboarding-hire-an-ai-teammate.md` F18 [P3] — Welcome step's "Try Genesis studio →" affordance is a thin underline, not a card/CTA

## Fix recipe

1. **Rewrite the Genesis pillar** at `web/app/(setup)/onboarding/welcome/page.tsx:46-54` to lead with what Genesis DOES today:
   - "Genesis · Onboarding Studio. Describe the role in plain English; upload SOPs, paste wiki URLs, link Slack threads; Genesis synthesises charter, action vocabulary, memory schema, eval suites — then checks the teammate's comprehension before they start work." (or similar — operator may want to refine).
   - Defer "Voice uploads (coming v1.1)" to the link target page or omit entirely.
2. **Promote the Genesis CTA** at `web/app/(setup)/onboarding/welcome/page.tsx:76-88`:
   - Replace the underline-on-hover `<Link>` with a `<button class="ob-cta-ghost">` or a `<Link>` styled as a tertiary button with a brand-gradient border.
   - Position next to the Get Started primary CTA, not below it.
3. **Cross-link from `/genesis` back to onboarding** so the discoverability is two-way (operator may decide this is out of scope).
4. **Snapshot test** updated.

## Acceptance criteria

- [ ] The Genesis pillar copy reads as a present-tense capability, not a deferred stub.
- [ ] The "Try Genesis studio" affordance is visibly a CTA (button-shaped, clear hover state, brand-gradient or accent).
- [ ] The pillar + CTA remain on-brand per the locked F1 editorial design.
- [ ] Snapshot tests pass.
- [ ] PR body cites findings F17 + F18 + the operator memory `feedback_genesis_is_ai_teammate_onboarding_studio`.

## May-touch

- `web/app/(setup)/onboarding/welcome/page.tsx`
- Snapshot test fixtures

## Out of scope

- Genesis page content rework (covered by `fix-genesis-page-h1-onboarding-studio-not-hire` + `fix-onboarding-welcome-step-genesis-studio-framing`)
- Genesis multi-modal upload implementation (post-v1)
- Welcome step autofocus (covered by `fix-onboarding-welcome-step-autofocus-cta`)

## Effort

XS — ~30-50 LOC + snapshot update. ~30-60 minutes.
