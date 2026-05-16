# claude-spa-signup-flow — SPA signup with magic-link auth + tenant kickoff (Wave 2 P0)

**Owner**: claude. **Depends on**: `claude-brand-identity-system` MERGED + `claude-design-system-component-extension` MERGED.
**Authored against**: origin/main `489e0580` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-spa-signup-flow`.
**Source**: §2 of `docs/build-plan/V1_WEEKEND_PLAN.md`.

## Mission alignment

- **Claim 1 (teammate not tool):** Onboarding starts here. The first 60 seconds shape every customer's mental model of SKYCOT.
- **Viral-readiness pre-condition:** without self-serve signup, any viral traffic to `skycot.com` bounces.

## Scope

P0. Build the signup landing at `app.skycot.com/signup`:
- Magic-link email entry (no password — modern + secure + frictionless).
- Submit → backend issues + emails magic link.
- Link → `app.skycot.com/auth/verify?token=...` → JWT session issued → redirect to `/onboarding/welcome`.
- Rate-limit failed attempts (5 per IP per hour).
- Optional Google/GitHub OAuth (defer to v1.x — magic link only for v1).
- Confirmation page after submit: "check your email" + resend option.

LOC: ~800.

## Files you may touch

- `web/app/auth/signup/page.tsx` (NEW) — signup landing. Uses `SignupForm` from design system. ≤120 LOC.
- `web/app/auth/check-email/page.tsx` (NEW) — "check your email" confirmation page. ≤80 LOC.
- `web/app/auth/verify/page.tsx` (NEW) — magic-link landing; calls backend exchange + redirects. ≤100 LOC.
- `web/lib/auth/use-signup.ts` (NEW) — React hook wrapping signup mutation. ≤80 LOC.
- `web/lib/auth/use-verify.ts` (NEW) — React hook for token exchange. ≤60 LOC.
- `web/lib/auth/signup-schema.ts` (NEW) — zod schema for email validation. ≤30 LOC.
- `web/app/api/auth/signup/route.ts` (NEW) — Next.js API route → forwards to argo gateway. ≤100 LOC.
- `web/app/api/auth/verify/route.ts` (NEW) — Next.js API route → forwards to argo gateway. ≤80 LOC.
- `web/__tests__/auth/*.test.tsx` (NEW) — signup + verify + rate-limit tests. ≤200 LOC.

## Files you must NOT touch

- `web/app/(app)/**` — application surfaces (post-signup); separate brief.
- Backend Rust (gateway endpoints) — authored by `claude-magic-link-auth-backend`.
- Brand tokens — locked by `claude-brand-identity-system`.

## Acceptance criteria

- [ ] `/signup` renders signup form with email input + submit button + brand-consistent layout.
- [ ] Form validates email format client-side via zod.
- [ ] Submit → POST to `/api/auth/signup` → "check your email" page.
- [ ] Rate limit: 5 attempts/hour/IP — 6th shows clean error.
- [ ] Magic link verify → `/auth/verify?token=` → exchange → JWT in httpOnly cookie → redirect `/onboarding/welcome`.
- [ ] Accessibility: keyboard navigable, aria-labels, focus order correct.
- [ ] Dark mode works.
- [ ] Lighthouse ≥ 95 on `/signup`.
- [ ] Test coverage for: submit success, submit invalid email, rate-limit hit, verify success, verify expired token.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `claude-spa-signup-flow`
- PR title: `web: signup + magic-link verify SPA flow (claude-spa-signup-flow)`

## Notes for the implementer

- Magic link tokens: 32-byte random, base64url-encoded, 15-min TTL. Backend handles generation; SPA just exchanges.
- Cookie: httpOnly, secure, SameSite=Lax. Name: `skycot_session`.
- After verify success, BACKEND has tenant_id in JWT claims; SPA reads it from `/api/auth/me` on subsequent loads.
- If tenant not yet provisioned (first-time user), backend returns `needs_onboarding: true` and SPA redirects to `/onboarding/welcome` (handled by `claude-tenant-provisioning-endpoint`).
- Don't show passwords. Don't ask for name. Email is enough — collect name/company during onboarding.
