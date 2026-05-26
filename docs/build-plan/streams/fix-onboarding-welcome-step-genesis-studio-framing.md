# fix-onboarding-welcome-step-genesis-studio-framing

**Owner**: claude.
**Authored against**: origin/main `80e149df55f6` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-welcome-step-genesis-studio-framing`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-1028/onboarding-hire-an-ai-teammate.md` F12 [P2].

🟠 **P1 — Welcome step's three pillars are "Audit-first / Single-binary / Enterprise-grade". Correct-but-bland infra claims that read "we're a backend platform" rather than "we onboard AI teammates like humans." The Genesis-Studio thesis (per memory `feedback_genesis_is_ai_teammate_onboarding_studio`) is absent from the investor's first deep impression after signup.**

## Root cause

`web/app/(setup)/onboarding/welcome/page.tsx:36-55` renders three pillar cards: Audit-first, Single-binary, Enterprise-grade. None mentions the AI Teammate Onboarding Studio framing — voice/text role, SOP-PDF/wiki/Slack uploads, comprehension verification. Per operator-locked memory, this is "the investor moment that distinguishes Skycot from 'ChatGPT with prompts'", and the welcome step is precisely where the investor forms the first impression of the product positioning.

## Code evidence

`web/app/(setup)/onboarding/welcome/page.tsx:36-55`:
```tsx
<PillarCard title="Audit-first" body="Every action signed, hashed..." />
<PillarCard title="Single-binary" body="One Rust binary, zero ops..." />
<PillarCard title="Enterprise-grade" body="SOC2 day-one, RBAC..." />
```

## Findings this brief closes

- onboarding-hire-an-ai-teammate F12 [P2] (escalated to P1 — welcome is the investor's first impression) — Genesis Studio framing missing from welcome step copy

## Fix recipe

1. **Replace at least one pillar** (probably "Single-binary", which is infra-internal language) with the Genesis-Studio claim. Suggested copy:
   - Title: "Onboard them like a human"
   - Body: "Describe the role in voice or text. Upload your SOPs, wiki pages, and Slack threads. We verify they understand before they start."
2. **Optionally replace a second pillar** ("Enterprise-grade") with a paired claim about voice/comprehension if the operator wants two Genesis-anchored pillars instead of one:
   - Title: "They learn your way of working"
   - Body: "Skills propagate across the workforce. New hires inherit your team's playbook on day one."
3. **Keep "Audit-first"** as the third pillar — it's both true and on-brand.
4. **Update the welcome hero subhead** to match: instead of "Hire your first AI teammate" (which is correct but generic), try "Onboard your first AI teammate" — "onboard" is the verb the Genesis-Studio thesis hinges on.
5. **A11y check**: the new pillar copy should still pass the same axe/lighthouse audits.

## Acceptance criteria

- [ ] At least one pillar references voice/text role, SOP uploads, or comprehension verification.
- [ ] Visual register matches the existing F1 editorial-center design (no Audiowide on body text, gradient sparing).
- [ ] Welcome step hero copy aligns with "onboard your AI teammate" framing.
- [ ] No regression in lighthouse / axe scores.
- [ ] Web smoke test asserts the new pillar text renders.

## May-touch

- `web/app/(setup)/onboarding/welcome/page.tsx` (pillar swap)
- `web/components/skycot/onboarding/PillarCard.tsx` (verify path; no schema change expected)
- `web/tests/onboarding-welcome.test.ts` (NEW or extend)

## Out of scope

- The full Genesis flow at /hire (separate brief tree: `mission-genesis-*`)
- Replacing the simple text-area path at `/onboarding/first-hire` with the Genesis flow (separate strategic call — captured in `mission-genesis-onboarding-pick-vertical-add-describe-own`)
- Voice input via argo-voice-io (separate brief: `mission-genesis-voice-input-via-argo-voice-io`)

## Effort

~30-50 LOC. ~30-45min copy lift. Lowest-effort step on the demo recovery path.
