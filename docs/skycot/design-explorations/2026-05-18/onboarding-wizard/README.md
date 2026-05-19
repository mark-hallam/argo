# Onboarding wizard — design exploration (2026-05-18)

> **⚠ 2026-05-19 update**: The locked pick (F1 editorial-center, all 5 steps) received a brand-consistency pass alongside all other in-product pages. The "brand-restrained / brand-medium / brand-loud" labels in the comparison table below describe the **original mockups**, not the final state. See `DESIGN_SESSION_RESUME.md` and `project_brand_consistency_pass.md` for the unified brand recipe now applied across every page.

The onboarding wizard is a 5-step flow:

1. **Welcome** (you're here)
2. Workspace — name + invite teammates
3. Connect provider — Anthropic / OpenAI / SKYCOT-managed
4. Pick vertical — support / invoice / recruiting / quote / sales
5. First hire — name, skills, schedule, start

Onboarding is the **first impression**. Five wildly varied design languages for the welcome screen — pick the language, then I'll extend it across the other 4 steps.

## The five lenses

| File | Metaphor | Voice | Brand register | Best for |
|---|---|---|---|---|
| [option-A-cinematic.html](./option-A-cinematic.html) | Cinematic / Apple-keynote | "Build your AI workforce." | **Brand-loud** (dark page, gradient hero, Audiowide) | Aspirational tone from the first second |
| [option-B-conversational.html](./option-B-conversational.html) | Chat with "Maya, your onboarding host" | "Hi Mark — welcome to SKYCOT!" | Brand-medium (Inter, gradient avatars, chat bubbles) | Reinforcing the "AI teammate" brand promise via a host AI |
| [option-C-stripe-split.html](./option-C-stripe-split.html) | Stripe / Atlas split-screen onboarding | Sober + professional | **Brand-restrained** (Inter, white + light gradient wash) | Enterprise buyers who want to feel they're using grown-up software |
| [option-D-notion-friendly.html](./option-D-notion-friendly.html) | Notion empty-state / friendly emoji | "Hey Mark! Let's set this up together." | Brand-medium (Inter, warm cream paper, emoji decor) | Approachable + casual + warm |
| [option-E-game-launcher.html](./option-E-game-launcher.html) | Game character-creation / mission launcher | "MISSION 01 OF 05 · WELCOME" | **Brand-loud max** (dark page, Audiowide, neon glows, slot grid preview) | Operators who want activation to feel like a game · ties to Workers page (C2 holographic deck) |

## Shared content across all 5

- Same "you're at step 1 of 5" progress
- Same operator name (Mark) personalization
- Same time estimate (~4 min, no credit card needed)
- Same social proof (2,847 companies onboarded this month)
- Same "Get started" CTA + secondary "watch tour"

## How to pick

```bash
open ~/code/argo/docs/skycot/design-explorations/2026-05-18/onboarding-wizard/*.html
```

Five-second gut on each, then tell me one of:

1. "A — cinematic" → I extend to the other 4 wizard steps in this language
2. "B — conversational" → same
3. "C — stripe split" → same
4. "D — notion friendly" → same
5. "E — game launcher" → same
6. Combos → e.g. "C's split-screen with A's hero copy"
7. "None — try X"

After the language is locked, I produce 4 more screens (workspace / connect provider / pick vertical / first hire) and we lock the onboarding flow.
