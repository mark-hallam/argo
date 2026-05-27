# fix-onboarding-hero-ai-teammate-vs-teammate-headline-drift

**Owner:** either
**Authored against:** origin/main `410e6ed1240d26f9ab6b48eb94a2fdf177c38270` on 2026-05-28
**Validate before claim:** `bash scripts/check-brief-freshness.sh fix-onboarding-hero-ai-teammate-vs-teammate-headline-drift`
**Source:** ux-audit-2026-05-27-2351

🟢 **P2 — Marketing → signup → onboarding hero copy disagrees: `/landing` and `/auth/signup` use "AI teammate" but `/onboarding/welcome` drops the "AI". Inconsistent across the funnel.**

## Root cause

Confirmed by curl + DOM inspection:
- `/landing` h1 includes "AI teammate"
- `/auth/signup` h1 reads `Hire your first AI teammate.` (line 158, `web/app/auth/signup/page.tsx`)
- `/onboarding/welcome` h1 reads `Hire your first teammate.` (per the `heroSegments` prop in `web/app/(setup)/onboarding/welcome/page.tsx:22-25`)

> Both are correct in isolation; the inconsistency is the friction ("did SKYCOT just stop using 'AI' on purpose?"). The cleanest unification is to drop "AI" from the signup hero — the wizard's job is to onboard a teammate, the camera already saw "AI" 18 times on `/landing`.
> — `onboarding-hire-an-ai-teammate.md` F12

The investor demo MP4 traverses /landing → /auth/signup → /onboarding/welcome and the camera reads these three hero lines back-to-back; the inconsistency is camera-visible. Per the function-vs-dressing rule this is dressing-side, but the visible-on-camera nature pushes it to P2 not P3.

## Findings this brief closes

- `onboarding-hire-an-ai-teammate.md` F12 [P2] — Welcome step's "Hire your first teammate" hero ≠ Signup page's "Hire your first AI teammate" hero

## Fix recipe

1. **Pick the canonical hero phrase** — the audit recommends "Hire your first teammate" (the welcome page version), since `/landing` already establishes the "AI" framing repeatedly. Operator should confirm.
2. **Edit `web/app/auth/signup/page.tsx:158-162`** — change `<h1>Hire your first AI teammate.</h1>` to `<h1>Hire your first teammate.</h1>`.
3. **Cross-check the marketing landing copy** — operator may prefer the other direction ("AI teammate" on all three). If so, edit `web/app/(setup)/onboarding/welcome/page.tsx:22-25` instead.
4. **Verify there are no other downstream pages** with a third variant (e.g. `/landing/(verticals)/<id>/page.tsx`): `rg "Hire your first" web/app/` will enumerate.
5. **Snapshot tests** at `web/app/auth/signup/page.test.tsx` (or equivalent) should be updated to match.

## Acceptance criteria

- [ ] All three hero lines (/landing, /auth/signup, /onboarding/welcome) consistently use one phrase.
- [ ] Snapshot tests pass.
- [ ] PR body cites finding F12 + the chosen direction.
- [ ] No regression on the locked F1 editorial design (which doesn't mandate the specific words).

## May-touch

- `web/app/auth/signup/page.tsx`
- `web/app/(setup)/onboarding/welcome/page.tsx`
- `web/app/(marketing)/landing/page.tsx` (verify only)
- Any vitest snapshot for the above

## Out of scope

- Other onboarding copy polish (covered by individual `fix-onboarding-welcome-*` briefs)
- Marketing nav teammate-vs-worker drift (covered by `fix-marketing-cta-teammate-vs-worker-vocab-drift`)

## Effort

XS — 1-2 line change + snapshot update. ~15-30 minutes.
