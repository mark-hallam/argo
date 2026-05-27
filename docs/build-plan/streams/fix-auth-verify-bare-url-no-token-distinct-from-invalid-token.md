# fix-auth-verify-bare-url-no-token-distinct-from-invalid-token

**Owner:** claude
**Authored against:** origin/main `2a9911c6828c4b37e9313b32dd024457e31d15b8` on 2026-05-28
**Validate before claim:** `bash scripts/check-brief-freshness.sh fix-auth-verify-bare-url-no-token-distinct-from-invalid-token`
**Source:** UX e2e audit 2026-05-28-0252 (onboarding F13)

🟢 **P3 — `/auth/verify` with no `?token=` query param lands on the same harsh "missing or malformed magic-link token" error state as a verifiably bad token. A user typing the URL directly, or arriving after a stale cached redirect, gets a user-hostile dead-end. The error UI itself is well-designed; only the copy doesn't distinguish the two cases.**

## Root cause

`web/app/auth/verify/page.tsx:44-46, 105-128` reads the token from `useSearchParams()`. If the token is absent, the code path hits the same error branch as "token present but cryptographically invalid". Both render the "That link / didn't work" editorial hero with "Get a new link" CTA.

The error state is nicely designed (memory `project_auth_pages_design_locked` — A-editorial-center, brand-medium register), so this is a soft P3, but it's a dead-end for anyone exploring URLs or hitting the page via a stale browser-cached redirect.

## Findings this brief closes

- onboarding F13 [P3] — `/auth/verify` with no `?token=` shows "missing or malformed" instead of distinct "looking for your sign-in link?" copy

## Fix recipe

1. **Branch on token presence vs token-present-but-invalid** at `web/app/auth/verify/page.tsx:44-46`:
   ```tsx
   const token = searchParams.get("token");
   if (!token) {
     // No token at all — soft, helpful copy
     return <VerifyNoTokenLanding />;
   }
   // Token present — actual verify attempt
   ```
2. **Add `<VerifyNoTokenLanding>` component**: same editorial-center layout as the error state but with softer copy:
   - Eyebrow: "Looking for your sign-in link?"
   - Hero: "Check your inbox for the magic link we sent."
   - Body: "If you didn't get one, request a new link — we'll email it in seconds."
   - Primary CTA: "Request a new link" → `/auth/signin?from=verify`
   - Secondary CTA (link): "Back to sign-in" → `/auth/signin`
3. **Keep the "That link / didn't work" branch** for the case where the token IS present but fails verification (cryptographic, expired, used). The copy there ("didn't work" + retry) is correct for that case.
4. **vitest** at `web/app/auth/verify/page.test.tsx`:
   - With no `?token=` query: assert renders `<VerifyNoTokenLanding>` content.
   - With `?token=invalid`: assert renders the existing "didn't work" error state.
   - With `?token=valid-mock`: assert verify-success flow proceeds.

## Acceptance criteria

- [ ] `/auth/verify` with no query param renders the soft "Looking for your sign-in link?" landing.
- [ ] `/auth/verify?token=<invalid>` continues to render the existing "didn't work" error.
- [ ] Both branches respect the A-editorial-center brand register (Audiowide eyebrow + Inter body + warm radial bg).
- [ ] vitest covers both branches.
- [ ] PR body cites onboarding F13.

## May-touch

- `web/app/auth/verify/page.tsx` (~30 LOC branch)
- `web/components/skycot/auth/VerifyNoTokenLanding.tsx` (NEW, ~80 LOC)
- `web/app/auth/verify/page.test.tsx` (NEW or extend, ~60 LOC)

## Out of scope

- Magic-link signup/verify backend flow (LIVE per CLAUDE.md status block)
- Other auth-page polish (signup hero off-center covered by `fix-auth-signup-desktop-hero-off-center`)
- Auth pages light/dark theme alignment (covered by `fix-auth-pages-light-theme-to-match-wizard`)

## Effort

XS/S — ~80 LOC new component + ~30 LOC branch + tests. ~2 hours.
