# Wave-11 Audit #1 — Magic-Link Signin Flow

- Audit date: 2026-05-21
- Scope: end-to-end magic-link signup → verify → land
- Method: static analysis only (read-only)
- SHA: `main`

## End-to-end score: 5 / 10

The happy path is wired, the brand language is locked, the API surface is
typed, and the tests for individual pages pass — but there is a **silent
P0 cookie-name mismatch between the gateway and the SPA middleware**
that means a freshly-verified user is bounced back to `/auth/signin`
forever. The flow does not complete in production today without that
fix.

---

## Page-by-page audit

### 1. `/auth/signup` (`web/app/auth/signup/page.tsx`)

| Item | State |
|------|-------|
| Email field validation | OK — zod schema in `web/lib/auth/signup-schema.ts:8` (trim + lowercase + 1-254 chars + RFC email). |
| Submit handler | OK — `useSignup.submit` in `web/lib/auth/use-signup.ts:26`. POSTs `{ email }` to `/api/auth/signup`. |
| Empty email | Submit button is `disabled` while `email.trim().length === 0` — user cannot fire empty. |
| Invalid email | Zod rejects locally, sets `state.kind = error`, inline `role="alert"` shown. `fetch` is **not** called. |
| Server 500 / network error | Caught — surfaces "Couldn't send magic link" or "Couldn't reach SKYCOT." |
| Rate-limit (429) | Handled — surfaces gateway-returned message (see P0 #2 below). |
| Success → `/auth/check-email?email=<encoded>` | OK — `useEffect` triggered when state is `sent`. |
| Accessibility | `aria-invalid`, `aria-describedby`, `role="alert"`, `autoFocus` on email — all present. |

Friction: **the same `useSignup` hook is reused on `/auth/signin` with
totally different semantics** (signin runs SSO discovery, signup does
not). The hook does not distinguish the two callers, which is fine
today but a sharp edge if signup ever needs to diverge.

### 2. `/auth/check-email` (`web/app/auth/check-email/page.tsx`)

| Item | State |
|------|-------|
| Shows entered email | OK — reads `?email=` query param, renders inline. |
| Falls back gracefully if `?email=` missing | OK — copy still renders, but Resend button is `disabled` when email is empty. |
| TTL visible | OK — copy says "It expires in 15 minutes." Matches gateway `MAGIC_LINK_TTL_SECS = 15 * 60` (`crates/argo-store/src/user_account.rs:36`). |
| Resend CTA | Present — re-hits `/api/auth/signup`. Inherits the same nested-error-shape bug (P0 #2). |
| Cooldown / debounce on Resend | **MISSING** — button only disables while in-flight. User can click 5 times in a row, then hit IP-based 429. No client-side cooldown. |
| Resend confirmation | Shows "✓ Sent again just now." after success — but the prior link is still valid until it expires or is consumed, so two valid links can coexist. Tests cover this expectation indirectly. |
| Support fallback | `mailto:support@skycot.com` — OK. |
| Suspense boundary | OK — wrapped to satisfy `useSearchParams` SSR rule. |

### 3. `/auth/verify` (`web/app/auth/verify/page.tsx`)

| Item | State |
|------|-------|
| `?token=` extracted | OK via `useSearchParams`. |
| `?email=` extracted | **NOT read** — verify only uses `token`. Mostly fine, but means a magic-link sent in cross-device mode cannot pre-fill anything for the user (low friction, just observation). |
| Missing token | Surfaces "Missing or malformed magic-link token." (good copy). |
| Loading state | Spinner + "Signing you in…" copy. |
| Verified state | Brief "You're in" then redirect via `router.replace`. |
| Bad token / expired token | Error shape mismatch — see **P0 #2 below**. The copy renders, but the *classification* lies. |
| Redirect target | If `needsOnboarding` → `/onboarding/welcome`; else → `/`. Gateway determines `needs_onboarding = tenant_id is None` (`crates/argo-cli/src/handlers/gateway/auth_magic_link_verify.rs:127`). |
| Error retry CTA | "Get a new link" → `/auth/signup` (good). |

### 4. Redirect target — `/` (Approvals) vs. `/onboarding/welcome`

- New user (no tenant link) → `/onboarding/welcome`. Five-step wizard.
- Returning user (≥1 tenant) → `/`, which routes to `web/app/(app)/page.tsx` (Approvals inbox).
- Logic is correct; the friction sits in step 5 (cookie-name mismatch).

---

## P0 — Showstopper bugs

### P0 #1 — Cookie name mismatch (auth flow is broken end-to-end)

- Gateway emits `Set-Cookie: __Host-session=...` (`crates/argo-cli/src/handlers/gateway/auth_magic_link_verify.rs:19,134`).
- Next API route `/api/auth/verify/route.ts:62-65` forwards that header unchanged. **Good.**
- `web/auth.config.ts:89` server-side reads `__Host-session` to surface the JWT into the NextAuth session. **Good.**
- BUT `web/middleware.ts:41-42` checks for `authjs.session-token` / `__Secure-authjs.session-token` — neither of which the gateway ever sets.

**Consequence:** Verify lands the `__Host-session` cookie successfully,
`router.replace("/")` fires, middleware then runs on the very next
request, fails the cookie check, and redirects back to
`/auth/signin?from=%2F`. The user sees signin again — they think their
link is broken.

Fix: middleware must read `__Host-session` (and optionally fall back to
the NextAuth cookies for the dev / bearer-token path). The test
`web/middleware.test.ts:11` is also asserting against the wrong cookie
and would mask any regression.

### P0 #2 — Gateway error response shape doesn't match SPA expectations

- SPA expects `{ error: { code: "token_expired", message: "..." } }` —
  `use-signup.ts:42-50`, `use-verify.ts:50-56`.
- Gateway returns the flat string field:
  - `{"error":"expired"}` for expired tokens
    (`auth_magic_link_verify.rs:92,99,102`).
  - `{"error":"rate_limited","scope":"signup","retry_after_secs":...}`
    for rate-limit (`auth_magic_link_signup.rs:32-37`).
- Result: `body.error?.code` and `body.error?.message` are `undefined`.
  - `classify()` in `use-verify.ts:26-31` falls through to `"server"`
    even when the gateway said `"expired"`.
  - Friendly messages ("Sign-in link expired. Request a new one.") are
    never shown; user sees the generic fallback "Sign-in link is no
    longer valid." for every failure class.
  - The 429 user-facing message becomes "Too many attempts. Wait an
    hour and try again." (a hard-coded fallback), not the
    gateway-provided retry_after.
- The Next API proxy at `/api/auth/verify/route.ts:48-51` blindly
  forwards the gateway payload, so the shape mismatch reaches the
  client unmodified.

Fix: either reshape in the Next proxy, or have the gateway emit the
nested shape. The proxy is the lower-risk place because the gateway
shape may have other consumers.

### P0 #3 — The auth/verify integration test stubs the wrong cookie

`web/tests/e2e/onboarding.spec.ts:129` stubs the verify response with
`Set-Cookie: skycot_session=stub-jwt`. That is a third cookie name —
neither the production `__Host-session` nor the middleware-expected
`authjs.session-token`. The E2E only passes because Playwright's stub
cookie is read by nothing downstream in the local mode (middleware is
not exercised once route stubs are active).

---

## P1 — Friction & polish

1. **No `from=` round-trip after verify.** Middleware sends unauth users
   to `/auth/signin?from=%2Fteam`, but `/auth/signin` does not read
   `from` and feed it into the magic-link request, and `/auth/verify`
   does not honor it on success — every verified user lands on `/`
   regardless of where they were headed.
2. **Resend has no debounce.** User can mash the button until the IP
   bucket flips to 429; the only friction is the in-flight disable.
3. **No "wrong device" copy.** If the magic link is clicked on a
   phone but signup happened on laptop, the JWT cookie lands on the
   phone's browser, not the laptop's. Nothing on the verify page nor
   the check-email page warns the user. This is the single largest
   support driver in magic-link products.
4. **Signin layout `title: "Sign in"` is still the default tab title
   for any `/auth/*` page that doesn't have its own layout export.**
   `auth/layout.tsx` is empty; per-page layouts redefine it, so this
   is OK on signup/check-email/verify/signin — but any new page added
   under `/auth/*` will inherit a blank title until it adds its own
   `metadata`. Worth a comment in the layout file.
5. **`/auth/error.tsx` calls `/auth/signin`, never `/auth/signup`.**
   A signup-phase 500 will surface "Back to sign in" as the secondary
   action — wrong direction for the very first interaction.

---

## Edge-case handling matrix

| Edge case | Handled? | Notes |
|-----------|----------|-------|
| Empty email submitted | Yes | Submit button disabled. |
| Invalid email format | Yes | Inline alert, no network call. |
| Email over 254 chars | Yes | Zod schema caps at 254. |
| Email with uppercase / whitespace | Yes | Trim + lowercase before send. |
| Gateway 500 | Yes | Generic fallback message shown. |
| Gateway unreachable (502 from Next proxy) | Yes | Surfaces "Couldn't reach SKYCOT." |
| Network failure on client | Yes | `catch` block. |
| Rate-limit (429) | Partial | Caught, but message text is *always* the hard-coded fallback (P0 #2). |
| Token missing in URL | Yes | "Missing or malformed magic-link token." |
| Token expired (15 min lapsed) | **No** | Misclassified as `server` due to P0 #2; user sees generic copy. |
| Token reused (replay) | **No** | Gateway does have replay protection (`ConsumedToken::AlreadyUsed`) but SPA misclassifies the response. |
| Token tampered (8+ chars but not real) | **No** | Same misclassification. |
| Magic link opened on different device | **No** | Cookie lands on the wrong browser, original device sees nothing. No copy warns user. |
| Magic link opened while already signed in | **No** | Verify proceeds, mints new cookie, replaces existing session silently. Could be intentional but feels off. |
| Resend used while previous link still valid | Partial | Two links coexist. Gateway TTL still 15 min from issue time of each. No invalidation. |
| Browser without cookies | **No** | Verify "succeeds" but middleware never sees a cookie — infinite loop. |
| Signup→verify on the same tab vs new tab | Yes | `router.replace` works on either. |
| `from=/team` in middleware → never honored | **No** | Lost after verify. |
| OAuth callback failures | Partial | Same error-shape risk as verify, but the SSO path is the operator's secondary v1 surface. |

**Edge cases unhandled: 8** (token expired, token reused, token
tampered, cross-device, signed-in user re-verifying, cookies disabled,
`from=` honor, no Resend cooldown). Two more are partial: rate-limit
copy, resend invalidation of prior link.

---

## Top 5 friction points (in order)

1. **The cookie-name mismatch makes successful verification look like
   failure.** This is the only ship-blocker; everything else is paper
   cuts.
2. **Error classification lies.** Every failure looks the same to the
   user, robbing them of recovery hints (refresh? request new link?
   contact support?).
3. **No cross-device explanation.** A user requesting the link on
   laptop and opening it on phone hits a dead end with no guidance.
4. **`from=` query param is dropped at verify.** Users who deep-linked
   into the app and got bounced to signin always land back on `/` —
   the deep link they wanted is lost.
5. **Resend has no cooldown / visible TTL countdown.** Operator
   support inbox will fill with "didn't get the email" because users
   re-request 4-5 times before the email lands.

---

## Verdict — can a brand-new user sign up + verify + reach `/`?

**No, not today.** The cookie-name mismatch (P0 #1) means even a
perfectly-behaved happy-path user is bounced back to `/auth/signin`
the instant they land on `/`. They see signin twice and assume the
link is broken — classic "I clicked the link, it didn't work" support
ticket.

After that single line is fixed (middleware reads `__Host-session`),
the flow works for the happy path. P0 #2 only manifests on the sad
paths (expired / reused tokens) and degrades the recovery copy
without blocking anyone.

Brand quality and copy are excellent. Form ergonomics, accessibility,
and validation are all solid. The wiring between the SPA and the
gateway is where the cracks are.

---

## Recommended fix order

1. **Fix middleware cookie name** — `web/middleware.ts:41-42` →
   `req.cookies.get("__Host-session")?.value`. One-line PR. **Ship-blocker.**
2. **Update `middleware.test.ts:11`** to set `__Host-session` and add
   a regression test that explicitly asserts unsupported cookie names
   do NOT pass.
3. **Reshape gateway error responses in `/api/auth/verify` proxy** to
   `{ error: { code, message } }`. Lives in `web/app/api/auth/verify/route.ts`.
4. **Mirror the reshape in `/api/auth/signup` proxy** for 429 / 400.
5. **Add `from=` query forwarding through verify** — read `?from=` on
   `/auth/verify`, pass it through, `router.replace(from ?? dest)`.
6. **Add a `cooldown` on the Resend CTA** (e.g. 30s) with visible
   countdown.
7. **Add a cross-device hint** under the verify "didn't work" copy:
   "Did you click the link on a different device than where you
   requested it? Open the email on the same device."
