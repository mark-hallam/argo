# fix-marketing-footer-privacy-terms-security-routes-307-to-signin

**Owner:** claude
**Authored against:** origin/main `2a9911c6828c4b37e9313b32dd024457e31d15b8` on 2026-05-28
**Validate before claim:** `bash scripts/check-brief-freshness.sh fix-marketing-footer-privacy-terms-security-routes-307-to-signin`
**Source:** UX e2e audit 2026-05-28-0252 (whole-site F01)

🟠 **P1 — Marketing footer links to `/privacy`, `/terms`, `/security` get 307-redirected to `/auth/signin?from=%2Fprivacy` for logged-out visitors. These are public-intended pages that the middleware allowlist omits. Any prospective customer landing on the marketing site and clicking the legal links hits a sign-in wall instead of the legal page.**

## Root cause

`web/middleware.ts:25-41` lists the routes exempt from the auth gate:

```ts
const PUBLIC_PATHS = [
  "/landing",
  "/marketing",
  "/pricing",
  "/customers",
  "/compare",
  "/changelog",
  "/docs",
  "/help",
  "/sla",
  "/for/",
];
```

But `web/components/marketing/MarketingFooter.tsx` links to `/privacy`, `/terms`, `/security`, and those routes have real page files at `web/app/privacy/`, `web/app/terms/`, `web/app/security/` — they are public-intended content, not authed pages. The middleware sees them as unallowlisted and 307s the visitor to `/auth/signin?from=%2Fprivacy`.

Reproduction (logged-out incognito browser):

```
GET http://localhost:3000/privacy   → 307 Location: /auth/signin?from=%2Fprivacy
GET http://localhost:3000/terms     → 307 Location: /auth/signin?from=%2Fterms
GET http://localhost:3000/security  → 307 Location: /auth/signin?from=%2Fsecurity
```

Sibling `fix-spa-root-redirect-unauth-to-landing` handled the `/` root redirect but did not touch the footer paths. This is the trust beat — a buyer clicking "Privacy" in the footer expects to read the privacy policy, not bounce to a sign-in form.

## Findings this brief closes

- whole-site F01 [P1] — Marketing footer links to /privacy, /terms, /security redirect logged-out visitors to signin

## Fix recipe

1. **Extend the `PUBLIC_PATHS` allowlist** in `web/middleware.ts:25-41` with the three legal/public pages:
   ```ts
   const PUBLIC_PATHS = [
     "/landing",
     "/marketing",
     "/pricing",
     "/customers",
     "/compare",
     "/changelog",
     "/docs",
     "/help",
     "/sla",
     "/privacy",
     "/terms",
     "/security",
     "/for/",
   ];
   ```
2. **Confirm the page files exist and render real content** by reading `web/app/privacy/page.tsx`, `web/app/terms/page.tsx`, `web/app/security/page.tsx`. If any is a stub or `redirect()` to an authed page, replace with real public content (or coordinate with the operator if the content isn't ready).
3. **Marketing footer also references `mailto:hello@skycot.com`** (per whole-site F15) — out of scope here; leave for operator confirmation.
4. **Add a smoke test** at `web/tests/middleware-allowlist.test.ts` (NEW or extend) that asserts each of the three new paths returns a real `Response` (not a 307) when the request has no `Cookie` header. Use the Next.js middleware test harness pattern from any existing middleware test.
5. **Playwright spec** (optional but cheap) at `web/playwright/marketing/footer-legal-links.spec.ts` that walks `/landing` → click footer "Privacy" → assert URL ends with `/privacy` and the H1 text starts with "Privacy" (not "Sign in").

## Acceptance criteria

- [ ] Logged-out `GET /privacy`, `/terms`, `/security` return 200 with the real legal page content (no 307 redirect).
- [ ] Middleware allowlist test passes with all three paths.
- [ ] Playwright footer-link spec green (if added).
- [ ] No regression on the existing public-paths suite.
- [ ] PR body cites whole-site F01.

## May-touch

- `web/middleware.ts` (additive)
- `web/app/privacy/page.tsx`, `web/app/terms/page.tsx`, `web/app/security/page.tsx` — verify content exists (touch only if stub)
- `web/tests/middleware-allowlist.test.ts` (NEW or extend)
- `web/playwright/marketing/footer-legal-links.spec.ts` (NEW, optional)

## Out of scope

- Writing the actual privacy / terms / security policy content (operator-decisional)
- The `hello@skycot.com` mailto live-status (covered by whole-site F15, operator-only)
- Marketing theme toggle (covered by separate brief)
- `/team` and `/brand` URL-namespace re-shuffle (covered by separate brief — whole-site F13)

## Effort

XS — ~5 LOC middleware change + ~30 LOC test. ~30 min including a playwright smoke. If a page stub needs real content, ~1-2 hours.
