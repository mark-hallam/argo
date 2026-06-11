# web-quality-pin-residuals-copy-brand-sso

**Owner**: claude or codex — web-only, DEC-006 fast lane; LOCKED BRAND — consume tokens, no redesign.
**Depends on**: none.
**Authored against**: origin/main `1735b11c3` on `2026-06-12`.
**Validate before claim**: bash scripts/check-brief-freshness.sh web-quality-pin-residuals-copy-brand-sso

## Problem (verified by conductor's own full vitest run, 2026-06-12)

Seven small, pre-existing quality-pin failures remain after the #2841
harness root-fix — each is line-level product drift with an existing
failing test as the verifier (full characterization in the PR #2841 body
table):

1. `tests/marketing-cta-vocab.test.ts` — `/landing` primary CTA lacks
   "Hire your first teammate" (DEC-004-adjacent vocab lock).
2. `tests/onboarding-welcome.test.ts` — onboarding copy drifted from the
   pinned "Hire your first teammate" vocabulary.
3. `tests/public-pages-polish.test.tsx` — `app/landing/page.css`
   hard-codes white instead of a theme token.
4. `tests/locked-design-parity-owned.test.ts` — one owned surface off the
   runtime brand tokens (`web/app/globals.css` tokens are the source of
   truth — DESIGN_SYSTEM.md).
5. `__tests__/brand-execution-polish.test.tsx` — hardcoded `ease-out` in
   a component file (must use the token).
6. `__tests__/pricing/pro-cta-checkout.test.tsx` — `#enterprise-inquiry`
   lacks the `mailto:hello@skycot.com` link.
7. `components/auth/SsoButtons.test.tsx` (3 tests) — the test asserts SSO
   buttons render, but `SsoButtons` is INTENTIONALLY gated off
   (`NEXT_PUBLIC_SSO_ENABLED !== "1"` returns null, per the component's
   own docstring — avoids a gateway-404 dead-end). The product gate is
   correct; REWRITE THE TEST to stub the import-time env and assert both
   states (gated → null, enabled → buttons). Do NOT enable SSO (that is
   an operator/product decision).

## Fix

Items 1–6: fix the PRODUCT source line-level to satisfy the existing pin
(copy + CSS-token substitutions; consume `globals.css` tokens, locked
brand per DESIGN_SYSTEM.md — no visual redesign). Item 7: test-only
rewrite honoring the intentional gate.

## May touch

- the specific files each pin names (enumerate in the PR body)
- `SsoButtons.test.tsx` (item 7 only)
- NO weakening of any pin test for items 1–6

## Acceptance / verifier

1. The 7 named test files: all green
   (`npx vitest run <the seven files>`).
2. Full `vitest run`: failing test FILES strictly decreased vs the #2841
   baseline (11 → ≤4, the lint+CSP files covered by sibling briefs).
3. `tsc --noEmit` clean; `/landing`, `/onboarding`, `/pricing` return 200
   on dev.

~40–100 LOC across small files. **🟡 P3 — clears the long-tail so "vitest
green" becomes a whole-suite signal once the two sibling briefs land.**
