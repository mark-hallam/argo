# fix-signup-no-email-required-button-honest-copy

**Owner**: claude.
**Authored against**: origin/main `78f162a5` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-signup-no-email-required-button-honest-copy`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2152/onboarding-hire-an-ai-teammate.md` F5 [P0].

🔴 **P0 DEMO-BLOCKER — `/auth/signup` renders a secondary CTA labelled "Or try a demo workspace · no email required" but the button is `disabled` until the email field is non-empty, AND the click handler also requires the email — rejecting with "Enter an email to continue." The copy lies about the precondition. This is the first surface a fresh operator hits when entering the demo journey.**

## Root cause

`web/app/auth/signup/page.tsx:217-241` renders the secondary CTA:

```tsx
<button disabled={pending || email.trim().length === 0} ...>
  Or try a demo workspace · no email required
</button>
```

Line 221 has the `disabled` gate; lines 68-72 inside `submitDemoMint` reject when `email.trim().length === 0`:

```tsx
if (email.trim().length === 0) {
  setError({ code: "email_required", message: "Enter an email to continue." });
  return;
}
```

Three possible resolutions; pick option (b) per the "no email required" copy already shipped:

- (a) Change the visible copy to "Enter your work email to try a demo workspace" — kills the copy promise.
- (b) Mint a synthetic email when the field is blank — `demo+${Date.now()}@skycot.test` — honors the copy, requires no UI change.
- (c) Remove the email field entirely from the demo path.

The gateway's demo-mint route doesn't care about the email's deliverability — it just needs a unique principal key. So (b) is the cleanest fix and matches the visible promise.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F5 [P0] — Signup demo-button copy promises "no email required"; the button requires an email

## Fix recipe

1. **Drop the `disabled` gate on the demo button** at `web/app/auth/signup/page.tsx:221`:
   ```tsx
   disabled={pending}
   ```
2. **Mint a synthetic email in `submitDemoMint`** at `web/app/auth/signup/page.tsx:68-72`:
   ```ts
   const effectiveEmail = email.trim().length === 0
     ? `demo+${Date.now()}@skycot.test`
     : email.trim();
   // continue submitting with effectiveEmail
   ```
3. **Verify upstream** — confirm the demo-mint route handler at `web/app/api/auth/demo-mint/route.ts` accepts the synthetic format and that the gateway's demo-mint route doesn't reject `demo+ts@skycot.test` as malformed (it should — every email in the codebase's demo seed has the `demo+<token>@skycot.test` shape).
4. **Add a Playwright spec** `web/playwright/auth/signup-demo-button-no-email.spec.ts`:
   - Navigate to `/auth/signup`.
   - Click the demo button WITHOUT typing into the email field.
   - Assert the URL transitions to `/onboarding/welcome` (or wherever post-demo-mint lands).
   - Assert no error toast / banner appears.

## Acceptance criteria

- [ ] Demo button on `/auth/signup` is enabled regardless of email-field content.
- [ ] Clicking it with an empty email field proceeds to onboarding (no error).
- [ ] If a real email IS typed, that email is used (existing behavior preserved).
- [ ] New Playwright spec passes.

## May-touch

- `web/app/auth/signup/page.tsx:68-72` (handler)
- `web/app/auth/signup/page.tsx:217-241` (disabled prop on button)
- `web/playwright/auth/signup-demo-button-no-email.spec.ts` (NEW)

## Out of scope

- The underlying `gateway_unreachable` error on the demo-mint proxy (covered by `fix-auth-demo-mint-route-returns-gateway-unreachable`).
- Magic-link signup path (untouched).
- The signup page hero-centering bug (covered by `fix-auth-signup-desktop-hero-off-center`).

## Effort

~30 LOC + 1 spec. ~1-2 hours. P0 because it's the first visible promise on the demo journey's entry surface.
