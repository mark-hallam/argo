# fix-onboarding-add-done-page-with-meet-your-teammate-handoff

**Owner**:
**Authored against**: origin/main `354de50a0c5fdf561125c7e931699fe94daa326d` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-add-done-page-with-meet-your-teammate-handoff`.
**Source**: e2e-ux-audit T0739 — `docs/skycot/ux-audits/2026-05-27-0739/onboarding-hire-an-ai-teammate.md` F14.

🟢 **P3 — The wizard caps at step 5; on submit the user redirects directly to `/workers/<id>/overview` with no celebration moment, no "Maya is ready — meet your first teammate" handoff. For the investor demo the recording loses its emotional peak. For a real user the moment "I have hired a teammate" is undifferentiated from "I'm browsing a worker page".**

## Root cause

`web/app/(setup)/onboarding/first-hire/page.tsx:55-57` runs the redirect:
```ts
router.replace(`/workers/${workerId}/overview`);
```

No intermediate page, no celebration component, no transition affordance between "I just hired a teammate" and "I'm reading their overview". The wizard's step strip shows `5 of 5 · FIRST HIRE` then the next render the user is on a fully-loaded worker page — the punchline is missing.

The locked F1 editorial design and brand voice ("teammates, not tools") set up an emotional beat at this point in the journey. The current redirect skips the beat.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F14 [P3] — Step pip strip + step-label show "5 of 5 · FIRST HIRE" but no celebratory completion confirmation

## Fix recipe

1. **Create `/onboarding/done` route** at `web/app/(setup)/onboarding/done/page.tsx`. Same `WizardEditorialShell` chrome (eyebrow + hero typography + radial-blob bg). Hero copy:
   - Eyebrow: `MEET YOUR FIRST TEAMMATE`
   - Hero: gradient on the worker's name, e.g. **Maya is ready.** (read worker name from query param `?worker=<id>` or from the onboarding draft if still present).
   - Body: 1-2 sentences in the brand voice. Sample: "She'll Slack-message you in a few seconds with her first questions. Your draft is saved here while she settles in."
   - Primary CTA: `Meet Maya →` → routes to `/workers/<id>/overview` (the existing destination).
   - Secondary: `View team →` → routes to `/team`.
2. **Update the first-hire redirect** at `web/app/(setup)/onboarding/first-hire/page.tsx:55-57` to route through `/onboarding/done?worker=<id>` instead of directly to `/workers/<id>/overview`.
3. **Auto-advance after 6 seconds** (operator-tunable). After the celebration page first paints, a `setTimeout(() => router.replace('/workers/<id>/overview'), 6000)` advances the user. The CTA button is the fast-path; the timeout is the kindness for users who freeze. Cancel the timeout if the user navigates away or clicks the CTA early.
4. **Track that the user has seen the done page** so refreshing `/onboarding/done` (e.g. operator hits back) does not re-trigger the celebration loop indefinitely. Simplest: gate on `?worker=<id>` query param + a sessionStorage breadcrumb to prevent re-show across same-tab.

## Acceptance criteria

- [ ] `/onboarding/done?worker=<id>` renders with the locked F1 editorial chrome.
- [ ] Hero displays the worker's name in the gradient slot.
- [ ] Primary CTA routes to `/workers/<id>/overview`.
- [ ] First-hire submit → land on `/onboarding/done?worker=<id>` (NOT directly on `/workers/<id>/overview`).
- [ ] 6-second auto-advance fires if the user does nothing; clicking the CTA cancels the timeout.
- [ ] Playwright pin: walk the demo onboarding path; assert URL passes through `/onboarding/done` before reaching `/workers/`.

## May-touch

- `web/app/(setup)/onboarding/done/page.tsx` (NEW)
- `web/app/(setup)/onboarding/done/__tests__/page.test.tsx` (NEW)
- `web/app/(setup)/onboarding/first-hire/page.tsx` (redirect URL change)
- `web/components/auth/WizardEditorialShell.tsx` (no shape change expected; reuse)

## Out of scope

- The `reset()` deferral on completion (covered by `fix-onboarding-save-and-exit-button-destructive-rename.md`)
- The worker's actual Slack-message wiring (operator-only / out-of-v1)
- A separate "tour" flow on the worker page after the celebration

## Effort

S — ~80-120 LOC across new page + redirect + pins. ~2 hours. P3 because the demo can land without it (current redirect works), but it's the highest-leverage emotional polish for the recording.
