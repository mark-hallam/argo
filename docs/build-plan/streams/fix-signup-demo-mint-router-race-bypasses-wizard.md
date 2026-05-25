# fix-signup-demo-mint-router-race-bypasses-wizard

**Owner**: claude.
**Authored against**: origin/main `d15461a3a` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-signup-demo-mint-router-race-bypasses-wizard`.
**Source**: e2e-ux-audit Agent (onboarding-hire-an-ai-teammate) finding F8 (`docs/skycot/ux-audits/2026-05-25-2309/onboarding-hire-an-ai-teammate.md`).

🟡 **P1 — Signup → demo-mint → wizard handoff races NextAuth's session refresh; user lands on `/team` instead of `/onboarding/welcome`. Runbook Scene 3 narrative cannot be naturally recorded.**

## Root cause

`web/app/auth/signup/page.tsx:106` routes the demo-mint flow to `/onboarding/welcome` AFTER calling `signIn("gateway-bearer", ...)`. In the audit session, submitting the signup form (with the prefilled `demo@skycot.com` email) instead landed on `/team` directly — i.e. the operator-set demo-mint code path is good, but the `router.push` raced with NextAuth's session refresh callback (which also pushes a destination).

Result: investor narrative "hire a teammate is a 4-minute wizard" cannot be recorded by hitting "Enter demo workspace" — the wizard is never shown. The operator has to manually type `/onboarding/welcome` to reach it, which breaks the natural funnel narrative.

The race is not deterministic — sometimes the wizard shows, sometimes it doesn't. Code at `web/app/auth/signup/page.tsx:80-95` shows the author understood the JWT + cookie + NextAuth handoff carefully; the bug is the subsequent navigation, not the auth handoff itself.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F8 [P1] — Signup → `/team` shortcut bypasses the 5-step wizard; the runbook's Scene 3 narrative cannot be recorded by hitting "Enter demo workspace"

## Fix recipe

Two options; pick (a) for the cleanest fix:

**(a) Always force `/onboarding/welcome` on demo-mint regardless of session state**:
1. Touch site: `web/app/auth/signup/page.tsx:106`.
2. Change `router.push("/onboarding/welcome")` to `router.replace("/onboarding/welcome")` so the demo-mint navigation cannot be popped back to.
3. In the NextAuth `signIn` callback at `web/lib/auth.ts` (or similar), check if the signup came from the demo-mint lane (via a query param or a cookie). If yes, do NOT redirect — leave the navigation to the signup-page's `router.replace`.

**(b) Gate the redirect on session-token confirmation**:
1. Touch site: `web/app/auth/signup/page.tsx:80-110`.
2. After `signIn`, await a single round-trip that confirms the JWT is hot (`fetch('/api/auth/session')` until `session.user` is set, ~50-300ms typical).
3. Only then `router.replace("/onboarding/welcome")`.

Option (a) is shorter; option (b) is more robust if the SPA's `useSession()` hook drives other downstream effects.

## Acceptance criteria

- [ ] Submitting `/auth/signup` with the prefilled demo email lands on `/onboarding/welcome` 10/10 times across cold-loads.
- [ ] No flash to `/team` in the URL bar mid-navigation.
- [ ] The wizard's "STEP 01 OF 05" indicator renders on first paint.
- [ ] No regression on the normal magic-link signup flow (which should still route to `/onboarding/welcome` for first-time tenants and `/` for return tenants).
- [ ] Playwright spec at `web/tests/signup-demo-mint-routes-to-welcome.spec.ts` pins the behavior.

## May-touch

- `web/app/auth/signup/page.tsx`
- `web/lib/auth.ts` (or NextAuth config file)
- `web/tests/signup-demo-mint-routes-to-welcome.spec.ts` (NEW)

## Out of scope

- The hardcoded `needs_onboarding: true` flag (separate brief `fix-demo-mint-needs-onboarding-flag-bounces-scene-1`) — that's the OPPOSITE bug for already-seeded tenants
- Magic-link signup flow (separate brief `mission-signup-magic-link-spam-protection`)
- Auth page theme drift (separate brief `fix-auth-pages-light-theme-to-match-wizard`)

## Effort

~30-80 LOC + 1 Playwright spec. ~1 hour.
