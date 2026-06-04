# sprint-ws5-2 — gate the internal /marketing GTM console behind auth

**Authored against:** origin/main `56a98b258` on 2026-06-05
**Validate before claim:** `bash scripts/check-brief-freshness.sh sprint-ws5-2-gate-internal-marketing-console-behind-auth`
**Owner:** claude · **Workstream:** WS5 (public perimeter) · **Lane:** frontend TS (no cargo)

## Why
For a fully-public launch, an internal AI-agent console must not be reachable without a session.
Surfaced during the #2703 consolidation adversarial review:
- `web/app/(app)/marketing/` is the internal **GTM agent console** (it lives in the authenticated `(app)/` group).
- BUT `web/middleware.ts` has `/marketing` in its **auth-exempt allowlist** (added 2026-05-20, commit `75b73e369`), so the page renders WITHOUT a session. (Its API calls would fail unauthenticated, but the console itself should not be exposed at all.)

## Task
1. **Confirm first (grep-before-fix).** Verify `(app)/marketing` is the internal GTM console and NOT a public marketing landing. The intended-public set is: `/` (landing), `/pricing`, `/about`, `/features`, `/blog`, `/contact`, `/customers`, `/changelog`, legal pages, `/dpa`. `/marketing` is NOT in that set. `rg "/marketing" web/` for any link/redirect that relies on it being public.
2. If confirmed internal: **remove `/marketing` from the `web/middleware.ts` auth-exempt allowlist** so it is cookie-gated like the rest of `(app)/`.
3. Add a regression **vitest**: an unauthenticated request to `/marketing` redirects to sign-in, while a genuinely public route (e.g. `/pricing`) still passes.

## Guardrails
- **If step 1 reveals `/marketing` IS intentionally public → STOP and report. Do not gate it.**
- Keep it small: the middleware change + the test, **≤5 files**.
- **WS5 is a public-perimeter "risky landing" per the operator-locked run-mode → this MUST go through adversarial review before merge. Do NOT self-merge.**

## Verify
- `npx tsc --noEmit` clean + `vitest run` green on touched files.

## PR
Title: `sprint(ws5) — perimeter — gate internal /marketing GTM console behind auth`
