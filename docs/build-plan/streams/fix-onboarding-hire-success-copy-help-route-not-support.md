# fix-onboarding-hire-success-copy-help-route-not-support

**Owner**:
**Authored against**: origin/main `354de50a0c5fdf561125c7e931699fe94daa326d` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-hire-success-copy-help-route-not-support`.
**Source**: e2e-ux-audit T0739 — `docs/skycot/ux-audits/2026-05-27-0739/onboarding-hire-an-ai-teammate.md` F13.

🟢 **P3 — Hire-verify-failed copy reads "Hiring succeeded but your teammate isn't visible yet. Try refreshing or **contact support**." But the SPA's help/support route is `/help` (a static doc) — there is no live support channel for a v1 buyer to *contact*. If this banner fires the user reads "contact support" and finds documentation. Honesty gap; fires only on the real path on a verify-race-loss.**

## Root cause

`web/lib/onboarding/use-first-hire.ts:10-11` defines:
```
const FIRST_HIRE_VERIFY_FAILED_MESSAGE = "Hiring succeeded but your teammate isn't visible yet. Try refreshing or contact support.";
```

The string references "contact support" — but there is no contact-support affordance in v1. The `/help` route is a static documentation page (no live chat, no email link, no ticket form). The copy promises something the product doesn't deliver.

Fires only when the post-hire verify probe (`fix-onboarding-first-hire-no-post-create-verify-probe.md` covers the probe wiring) reports a verify-race-loss — i.e. the gateway says hire succeeded but the SPA's read-after-write probe can't see the worker yet. Low frequency in v1 demo, but the copy is wrong even at low frequency.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F13 [P3] — Hire-success copy references "contact support" which has no resolution path

## Fix recipe

1. **Rewrite the copy** at `web/lib/onboarding/use-first-hire.ts:10-11`. New text:
   ```
   const FIRST_HIRE_VERIFY_FAILED_MESSAGE = "Hiring succeeded — your teammate is provisioning. Refresh in a few seconds; if they don't appear, the Help link in the top right has next steps.";
   ```
2. **Make `Help` an explicit affordance in the banner.** If the banner renders the message with no embedded link, add a `<Link href="/help">Help</Link>` element so the banner's "Help" is a real click target (not just words).
3. **(Optional, follow-on)** Audit `/help` content to ensure the runbook for "my teammate didn't appear" is present. If not, file a follow-on `fix-help-page-post-hire-troubleshooting`.

## Acceptance criteria

- [ ] `FIRST_HIRE_VERIFY_FAILED_MESSAGE` no longer contains the literal "contact support".
- [ ] The rendered banner includes a clickable `<Link href="/help">Help</Link>` (or equivalent) — verified via vitest pin.
- [ ] No copy regression elsewhere (rg the codebase for the old string to confirm it's only defined in one place).

## May-touch

- `web/lib/onboarding/use-first-hire.ts` (string)
- `web/app/(setup)/onboarding/first-hire/page.tsx` OR wherever the banner renders (add Link)
- `web/app/(setup)/onboarding/first-hire/__tests__/page.test.tsx` (pin)

## Out of scope

- The post-create verify probe itself (covered by `fix-onboarding-first-hire-no-post-create-verify-probe.md`)
- The `/help` page content (separate brief if operator wants the runbook)
- A real contact-support channel (separate v1.x post-MVP brief)

## Effort

XS — ~10 LOC. ~15 minutes. P3 because low fire-rate but high-honesty-cost.
