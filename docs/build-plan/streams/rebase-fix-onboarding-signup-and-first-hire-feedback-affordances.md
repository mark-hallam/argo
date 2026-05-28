# rebase-fix-onboarding-signup-and-first-hire-feedback-affordances

**Owner**: claude.
**Authored against**: origin/main `a5df90c41` on `2026-05-29`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh rebase-fix-onboarding-signup-and-first-hire-feedback-affordances`.
**Supersedes**: PR #2348 (closed for re-author 2026-05-29) + original brief `streams/fix-onboarding-signup-and-first-hire-feedback-affordances.md`.

🟡 **P1 — Self-host evaluators without SMTP get an irrecoverable signup state. Re-implement against current main; the original PR #2348 conflicted on the heavy drift in onboarding/signup/first-hire since 2026-05-26.**

## What changed on main since PR #2348 was authored

The original brief assumed `/onboarding/first-hire` was the textarea-driven flow. That has since gone through:
- `718fb9f6d` canonical-roster picker (later reverted)
- `58ea1c93f` revert back to textarea
- `7ab89623b` parameterise example prompt by vertical
- `ec4de733b` persist firstHirePrompt + provider key across step-back (#2477)
- `7e4a8ddc5` soften copy (#2493)
- `e59516d44` honest copy + Help link on verify-failed banner (#2484)
- `16aaab0ec` polish cluster — wrap + scroll + banner + eyebrow (#2504)

`/auth/signup` has also drifted:
- `de17ceaef` added "Or try a demo workspace" secondary button (#2357) — partial F7 coverage
- `e4a947e8d` unified hero with /landing (#2548)
- `789d39f9e` hire-wizard auth gate bounces to /auth/signup (#2540)

The landing page hero CTA wiring (`a824b2b12` — #2328) was overhauled too.

**Net effect**: F7 (demo-workspace fallback button) has partial coverage via #2357; F6 (landing demo shortcut) is partially shipped via the secondary demo button. The remaining load-bearing gap is the SMTP probe endpoint and the F14 first-hire feedback affordances.

## What this brief MUST ship

**For F6 (landing demo shortcut)** — possibly redundant; verify first:
1. `git grep -n "Or try a demo workspace\|demoMint\|/api/auth/demo" web/app/landing/page.tsx web/app/auth/signup/page.tsx` — if the landing page already exposes a demo shortcut, this finding may be closed already. Confirm against `docs/skycot/ux-audits/2026-05-26-0820/onboarding-hire-an-ai-teammate.md` F6.
2. If still load-bearing: when `NEXT_PUBLIC_ARGO_DEMO_MODE === "1"`, swap the primary landing CTA href to `/onboarding/welcome` rather than `/auth/signup`.

**For F7 (signup SMTP probe + fallback link)** — STILL LOAD-BEARING:
1. New endpoint at `web/app/api/auth/probe-smtp/route.ts` returning `{ healthy: boolean }` based on NextAuth's email-provider transport config.
2. New helper at `web/lib/auth/smtp-probe.ts` that reads the same env vars NextAuth's email provider uses (EMAIL_SERVER_HOST / EMAIL_SERVER_PORT / EMAIL_SERVER_USER / EMAIL_SERVER_PASSWORD / EMAIL_FROM) and returns `true` only when all are present.
3. In `web/app/auth/signup/page.tsx`, probe at mount. When unhealthy, render an inline note: "Magic link requires email delivery. For local testing, try a demo workspace." Demo-workspace link should call the same handler as the existing secondary demo button (#2357 wired it via `shouldShowDemoMintButton()` + `setDemoPending`).
4. When healthy, the page renders unchanged.

**For F14 (first-hire feedback)** — REVALIDATE BEFORE TOUCHING:
The current first-hire page already has:
- Error scrubbing via `_scrub-error.ts`
- `FirstHireErrorBanner` with `showHelpLink` (`e59516d44` shipped this)
- A `[error, setError]` state
The brief's "Try again / Back to vertical / Skip to your team" three-button recovery is only partially shipped (back button to /onboarding/pick-vertical exists at line 149). What's still missing per the original brief:
1. In-flight loading hero: while the mutation is pending, swap hero copy to "Onboarding ${displayName}..." with a small motion strip.
2. Sonner toast on success: "✓ Hired ${displayName}. Taking you to their page..." with a 600ms delay before navigation.
3. "Skip to your team" link on failure (routes to `/team`).

Decline-to-implement is acceptable if a fresh audit of `/onboarding/first-hire/page.tsx` against `docs/skycot/ux-audits/2026-05-26-0820/onboarding-hire-an-ai-teammate.md` F14 shows the finding is no longer load-bearing.

## Acceptance criteria

- [ ] `bash scripts/check-brief-freshness.sh rebase-fix-onboarding-signup-and-first-hire-feedback-affordances` passes.
- [ ] **F7 endpoint live**: `curl http://localhost:3000/api/auth/probe-smtp` returns `{"healthy":boolean}`.
- [ ] **F7 UI**: signup page shows the inline note + demo link only when probe returns unhealthy.
- [ ] **F14 in-flight**: clicking "Hire" shows the hero swap immediately (visually verifiable with the dev server).
- [ ] **F14 success**: sonner toast fires before navigation.
- [ ] **F14 failure**: three-button recovery row renders (Try again + Back to vertical + Skip to your team).
- [ ] No conflict with #2357 (the existing "Or try a demo workspace" button) — the new probe-based link is in addition to that button.
- [ ] No conflict with #2477 (`firstHirePrompt` persistence) — preserve the draft on failure (existing behavior, no regression).
- [ ] Vitest pin asserts the SMTP probe + the in-flight hero swap + the success toast.

## May-touch

- `web/app/landing/page.tsx` (only if F6 is still load-bearing)
- `web/app/auth/signup/page.tsx` (SMTP probe wiring)
- `web/app/api/auth/probe-smtp/route.ts` (NEW)
- `web/lib/auth/smtp-probe.ts` (NEW)
- `web/app/(setup)/onboarding/first-hire/page.tsx` (F14 — only the deltas, do NOT undo existing error-banner work)
- `web/tests/onboarding-first-hire-feedback.test.ts` (NEW pin)

## Out of scope

- Replacing the F1 editorial wizard chrome — already locked.
- Touching the canonical-roster picker (reverted by `58ea1c93f` — do not re-introduce).
- Touching the existing secondary "Or try a demo workspace" button (#2357 ships it; leave alone).
- Magic-link email delivery itself.

## Effort

~150-200 LOC; ~2-3 hours.

## Source

- Original brief: `streams/fix-onboarding-signup-and-first-hire-feedback-affordances.md`
- Closed PR: #2348 (closed 2026-05-29 as REAL-CONFLICT-NEEDS-REAUTHOR)
- UX audit: `docs/skycot/ux-audits/2026-05-26-0820/onboarding-hire-an-ai-teammate.md` F6 + F7 + F14
