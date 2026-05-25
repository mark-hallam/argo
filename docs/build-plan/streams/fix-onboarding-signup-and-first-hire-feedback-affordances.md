# fix-onboarding-signup-and-first-hire-feedback-affordances

**Owner**: claude.
**Authored against**: origin/main `5c4abb23e1f6` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-signup-and-first-hire-feedback-affordances`.
**Source**: e2e-ux-audit (onboarding-hire-an-ai-teammate) findings F6 + F7 + F14 — `docs/skycot/ux-audits/2026-05-26-0820/onboarding-hire-an-ai-teammate.md`.

🟡 **P1 — Self-host evaluators without SMTP get an irrecoverable signup state; operators clicking "Hire them" get no acknowledgment and no recovery affordance on failure.**

## Root cause

Three coupled UX gaps in the signup → first-hire flow, all about missing feedback / fallback affordances:

**F6 — landing CTA wires to /auth/signup with no demo bypass**:
`web/app/landing/page.tsx:145, :414` both link to `/auth/signup`. For a recording session where the operator wants to skip auth, there's no `?demo=1` shortcut or visible "Try a demo workspace" affordance. Demo prep needs DEMO_MODE + SMTP-skip wired before landing → signup → onboarding can be walked end-to-end without an off-camera env-flip.

**F7 — signup page renders "Send magic link" even when SMTP isn't configured**:
`web/app/auth/signup/page.tsx:26` always renders the "Send magic link" CTA unless `NEXT_PUBLIC_ARGO_DEMO_MODE === "1"`. There's no client-side probe for whether magic-link delivery actually works. A self-host evaluator without SMTP gets an irrecoverable email-pending state with no fallback path. The gateway DOES expose `POST /api/v1/demo/mint-principal` (returns a JWT directly — verified working in the audit), but the UI never surfaces it without the env flag.

**F14 — "Hire them" provides no acknowledgment + no recovery**:
`first-hire/page.tsx:42-64` awaits the mutation, calls `reset()` (clears the draft!), then `router.push(/workers/<id>)`. On success: no "Hiring Maya..." toast, no "✓ Hired" confirmation — page just navigates. On failure: the draft is NOT reset (good) but the error copy says "Try again, or come back once the gateway is up" — with no Back button to /pick-vertical and no Skip-to-team link. Operator is stuck on a dead form.

All three are dressing-side UX gaps that compound to make the signup → hire walk feel fragile. None alone breaks the demo, but together they create a fragile path that breaks on any infrastructure variance.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F6 [P1] — Landing CTA wires to `/auth/signup` not `/onboarding/welcome`; no demo shortcut
- onboarding-hire-an-ai-teammate F7 [P1] — Signup page renders "Send magic link" with no fallback when SMTP isn't configured
- onboarding-hire-an-ai-teammate F14 [P1] — First-hire shows no acknowledgment on success; no recovery affordance on failure

## Fix recipe

**For F6** (landing demo shortcut):
1. Touch `web/app/landing/page.tsx` lines 145 + 414. When `NEXT_PUBLIC_ARGO_DEMO_MODE === "1"`, swap the primary CTA href from `/auth/signup` to `/onboarding/welcome` (the gateway demo-mint can hydrate the session lazily on first onboarding write).
2. Add a small "Try a demo workspace" link beside the primary CTA when `NEXT_PUBLIC_ARGO_DEMO_MODE !== "1"` that POSTs to `/api/auth/demo-mint` and then redirects to `/onboarding/welcome`. Demo path becomes one-click regardless of env-flag state.

**For F7** (signup SMTP fallback):
1. Touch `web/app/auth/signup/page.tsx`.
2. Add a probe at mount: `GET /api/auth/probe-smtp` (a new health endpoint at `web/app/api/auth/probe-smtp/route.ts` that returns `{ healthy: boolean }` based on whether NextAuth's email provider config has a working transport).
3. When `healthy === false`, render an inline note beneath the form: "Magic link requires email delivery. For local testing, [Try a demo workspace](javascript:demoMint())" — clicking demoMint calls `/api/auth/demo-mint` and redirects to `/onboarding/welcome`.
4. The "Send magic link" button stays primary; the demo link is secondary and only shown when SMTP probe fails.

**For F14** (hire feedback):
1. Touch `web/app/(setup)/onboarding/first-hire/page.tsx`.
2. Add an in-flight state: while the mutation pends, hero swap to "Onboarding ${displayName}..." with a small animated dot strip. Use the existing skycot motion primitives.
3. On success: BEFORE `router.push`, fire a sonner toast "✓ Hired ${displayName}. Taking you to their page..." with a 600ms delay before navigation. Gives the operator a visual confirmation beat for the camera.
4. On failure:
   - Add a "Back to vertical" button that calls `router.push("/onboarding/pick-vertical")` — operator can change their mind without re-entering the wizard from scratch.
   - Add a "Skip to your team" link that calls `router.push("/team")` — escape hatch.
   - Keep the existing error message + retry button.
   - Do NOT call `reset()` on failure (preserves draft for retry — already correct, just re-affirming).

## Acceptance criteria

**F6**:
- [ ] With `NEXT_PUBLIC_ARGO_DEMO_MODE=1`, `/landing` hero + footer CTAs both point at `/onboarding/welcome` (not `/auth/signup`).
- [ ] Without the demo flag, primary CTA stays `/auth/signup` AND a secondary "Try a demo workspace" link is rendered that calls `/api/auth/demo-mint` + redirects.

**F7**:
- [ ] New endpoint `web/app/api/auth/probe-smtp/route.ts` returns `{healthy:boolean}` based on NextAuth email transport config.
- [ ] When SMTP probe returns unhealthy, signup page renders an inline note + demo-mint link below the form.
- [ ] When SMTP probe returns healthy, page renders unchanged (no extra UI).

**F14**:
- [ ] Clicking "Hire ${name}" shows an in-flight loading hero ("Onboarding ${name}...") immediately.
- [ ] On success: sonner toast fires with check-mark + name; 600ms delay before navigation.
- [ ] On failure: form shows error message + "Try again" + "Back to vertical" + "Skip to your team" — three distinct recovery paths.
- [ ] Draft is preserved on failure (existing behavior; no regression).

## May-touch

- `web/app/landing/page.tsx` (CTA wiring)
- `web/app/auth/signup/page.tsx` (SMTP probe + fallback link)
- `web/app/api/auth/probe-smtp/route.ts` (NEW)
- `web/app/(setup)/onboarding/first-hire/page.tsx` (acknowledgment + recovery)
- `web/lib/auth/smtp-probe.ts` (NEW — reads NextAuth config to detect SMTP availability)

## Out of scope

- Setting `NEXT_PUBLIC_ARGO_DEMO_MODE=1` in `.env.local` (operator/runbook concern)
- Magic-link delivery improvements (separate brief if needed)
- Configuring real SMTP for self-host (`docs/install/smtp.md` work, not code)

## Effort

~120-180 LOC across three pages + one new API route + one new lib helper. ~2-3 hours.
