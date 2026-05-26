# fix-signup-demo-mode-env-flag-and-runbook-prerequisites

**Owner**: claude.
**Authored against**: origin/main `80e149df55f6` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-signup-demo-mode-env-flag-and-runbook-prerequisites`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-1028/onboarding-hire-an-ai-teammate.md` F4 [P0].

🔴 **P0 DEMO-BLOCKER — `/auth/signup` is a real magic-link form in current `.env.local`. No SMTP attached → operator submits, lands on `/auth/check-email`, never receives an email, the recording aborts in Scene 3.**

## Root cause

`web/app/auth/signup/page.tsx:27` gates demo-mode on `NEXT_PUBLIC_ARGO_DEMO_MODE === "1"`. `web/.env.local` has neither `NEXT_PUBLIC_ARGO_DEMO_MODE` nor `NEXT_PUBLIC_SKYCOT_DEMO_MODE` set. With current env, the submit button reads "Send magic link" and calls `useSignup().submit({email})` (the real path) → `/api/auth/signup` → gateway → SMTP send. Dev has no SMTP attached, so the operator never receives the email. The runbook `docs/skycot/demos/v1-mission-mvp-runbook.md` does not mention this prerequisite.

The demo-mint code path (`/api/auth/demo-mint`) exists but is gated behind a flag the operator must set manually before recording. There is no in-app "Try the demo" button on `/auth/signup` that bypasses the flag.

## DOM evidence

- `web/.env.local` lacks both demo flags
- `/auth/signup` form submit button reads "Send magic link"
- After click, redirect to `/auth/check-email?email=<input>` with no follow-up

## Findings this brief closes

- onboarding-hire-an-ai-teammate F4 [P0] — `/auth/signup` is a real magic-link form in current `.env.local`; no demo-bypass for recording

## Fix recipe

Pick ONE of (a) or (b). (a) is shorter; (b) is more demo-friendly.

**(a) Set the demo-mode env flags + document in runbook:**
1. Add `NEXT_PUBLIC_ARGO_DEMO_MODE=1` + `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` to `web/.env.local` (commit to git).
2. Add `ARGO_DEMO_MODE=1` to whatever environment file the gateway boots from (or document the export).
3. Add a pre-record checklist line in `docs/skycot/demos/v1-mission-mvp-runbook.md`: "Verify `.env.local` has demo-mode flags; gateway booted with `ARGO_DEMO_MODE=1`."

**(b) Add a "Try the demo" button on `/auth/signup` that calls `/api/auth/demo-mint` directly regardless of env flag:**
1. In `web/app/auth/signup/page.tsx`, add a secondary button below the main form: "Or try a demo workspace · no email required".
2. The button calls `apiPost("/api/auth/demo-mint")` and routes the result to `/onboarding/welcome` with the demo tenant minted.
3. Hide the button in production builds via `process.env.NEXT_PUBLIC_ALLOW_DEMO_MINT === "1"` so it doesn't leak to a public-release site, but enable it locally + in dev preview.

Strongly prefer (b) — it makes the demo path discoverable without needing the operator to remember env-var dance, AND it doesn't require committing demo flags to git that might confuse fresh contributors.

## Acceptance criteria

- [ ] Operator following `v1-mission-mvp-runbook.md` step-for-step can complete the signup flow without setting environment variables outside the runbook's listed steps.
- [ ] Either: demo-mode flags are committed to `web/.env.local` AND mentioned in runbook pre-record checklist, OR: a "Try the demo" button exists on `/auth/signup` that calls `/api/auth/demo-mint` and routes to `/onboarding/welcome`.
- [ ] Production builds do NOT show the demo bypass button (gated on env flag).
- [ ] Web smoke test asserts the demo path completes signup without an SMTP round-trip.

## May-touch

- `web/.env.local` (option a)
- `web/app/auth/signup/page.tsx` (option b — add demo-mint button)
- `web/components/skycot/AuthProvider.tsx` (verify demo-mint hook surface)
- `docs/skycot/demos/v1-mission-mvp-runbook.md` (pre-record checklist)
- `web/tests/auth-signup.test.ts` (NEW or extend existing)

## Out of scope

- Real SMTP wiring for dev (the operator wants demo-mint, not real email)
- Magic-link expiry / token-rotation logic
- Public-release auth flow (this brief is POC-recording-only)

## Effort

~40-80 LOC if (b), ~10 LOC + runbook edit if (a). ~30min-1h.
