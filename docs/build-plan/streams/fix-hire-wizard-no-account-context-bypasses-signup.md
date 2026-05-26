# fix-hire-wizard-no-account-context-bypasses-signup

**Owner**: claude.
**Authored against**: origin/main `2cc2ec42b` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-hire-wizard-no-account-context-bypasses-signup`.
**Source**: ux-e2e-audit-T2309MEL `onboarding-hire-an-ai-teammate.md` F6 [P1].

🟠 **P1 — `/landing` "Hire your first teammate" CTA routes a brand-new visitor directly to `/hire/step/1`. The visitor authors an entire AI teammate (role description, clarifying questions, persona draft, channel hookup) without ever telling Skycot who they are, what workspace they belong to, or that they should have an account. In the demo recording configuration the auth-bypass header masks this, but in any real-buyer trial deployment the wizard either fails at `hireWorker()` (gateway rejects unauthenticated POST) OR silently creates a worker against an anonymous tenant the buyer can never return to. Either failure mode breaks the funnel.**

## Root cause

`web/app/(hire)/hire/page.tsx` (and its `/hire/step/[step]/page.tsx` children) does not check NextAuth session state before rendering the wizard. There is no auth guard. The four wizard steps (Land → Listen → Drafting → Introduce) happily collect role description, run the Genesis draft call, and let the user pick a name — all without identity context.

Step 4's "Hire {Name}" button calls `useHireWorker()` → `POST /api/v1/workers`. In auth-bypass mode (`.claude/launch.json:9`) the gateway accepts the unauthenticated POST and creates a worker against the anonymous demo principal. In a production-style deployment the gateway would 401, and the user would see a generic error toast at the END of a 4-step flow — the worst possible place to be told "you should have signed up first."

Compare `/onboarding/welcome` → `/onboarding/workspace`: workspace name + company name are collected FIRST, before any worker authoring. That flow is identity-first then hire. The hire wizard is hire-first then identity-never.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F6 [P1] — `/landing` CTA bypasses signup; user lands inside wizard with no account context

## Fix recipe

Pick ONE of three approaches (operator-decisional):

### Option A (preferred — interstitial signup at step 1)

1. At `/hire/step/1`, when the user clicks Continue:
   - If `session?.user` exists → proceed to step 2
   - If not → route to `/auth/signup?returnTo=/hire/step/2&problem=<urlencoded role description>` and resume after magic-link verify
2. `/auth/signup` already exists (`web/app/auth/signup/page.tsx`). Extend it to:
   - Accept `returnTo` + `problem` query params
   - Stash `problem` in sessionStorage so it survives the redirect through the magic-link
   - After verify, restore the `problem` into the wizard's draft state and resume at the named step

### Option B (merge into /onboarding flow)

Redirect `/hire` permanently to `/onboarding/welcome`. The 5-step `/onboarding` chain is identity-first by construction. This removes the parallel-wizards problem entirely.

This is the same direction as `fix-onboarding-hire-parallel-wizards-unify-or-document` recommends. If that brief lands first with the redirect option, THIS brief becomes a no-op.

### Option C (auth gate at step 4 commit)

Less preferred (defers the signup ask to the worst moment), but easiest to ship:
- Step 4 "Hire {Name}" button checks session, routes to signup with the entire wizard draft state persisted in sessionStorage
- After verify, restore + auto-commit

## Acceptance criteria

- [ ] Clean session → click `/landing` "Hire your first teammate" → land on `/hire/step/1` OR `/auth/signup` (Option A) OR `/onboarding/welcome` (Option B)
- [ ] In Option A: filling step 1 + Continue with no session routes to `/auth/signup?returnTo=/hire/step/2&problem=...`
- [ ] In Option A: after magic-link verify, the wizard resumes at step 2 with the problem-description textarea pre-filled
- [ ] In Option B: `/hire` returns a 308 redirect to `/onboarding/welcome` server-side
- [ ] No 401 toast appears at step 4 commit in a production-style auth-on deployment
- [ ] `pnpm --dir web exec vitest run app/\(hire\)` green
- [ ] e2e walk: unauthenticated /landing → CTA → signup → verify → wizard resumes correctly

## May-touch

- `web/app/(hire)/hire/page.tsx` (~10 LOC auth guard — Option A)
- `web/app/(hire)/hire/step/[step]/page.tsx` (~15 LOC session check + redirect)
- `web/app/auth/signup/page.tsx` (~20 LOC returnTo + problem-restore)
- `web/lib/hooks/use-hire-draft.ts` (~10 LOC sessionStorage persistence — verify file)
- `web/__tests__/hire-wizard-auth.test.tsx` (~50 LOC test)

## Out of scope

- Magic-link SMTP plumbing (already shipped; out of recording scope)
- Auth-bypass behavior in dev (`isAuthBypassed()` still short-circuits in `.claude/launch.json`; this brief targets production-style deployments)
- The parallel-wizards consolidation question (covered by `fix-onboarding-hire-parallel-wizards-unify-or-document`)

## Effort

S–M. ~60 LOC + ~50 LOC test. ~1.5 hours. P1 because the funnel is structurally broken for any post-demo buyer trial — the demo recording itself works due to auth-bypass, but the audit caught this as a latent shipping bug.
