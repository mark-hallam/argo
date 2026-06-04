# sprint-ws4-pricing-pro-cta-to-stripe-checkout

**Owner**: claude or codex. **Frontend lane — small, independent.**
**Depends on**: none to BUILD + test (the checkout endpoint + the hook already exist). LIVE charge needs WS4-2 (Stripe prod env) — out of scope here.
**Authored against**: origin/main `35d9225d2` on `2026-06-04`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh sprint-ws4-pricing-pro-cta-to-stripe-checkout`.

## Why

Day-0 gap-audit (WS4): the billing backend is far more complete than assumed —
`POST /api/v1/billing/checkout` is LIVE, `web/lib/billing/use-checkout.ts`
exists and redirects to the Stripe hosted URL. But the pricing page's Pro CTA is
**not connected to it**: `web/app/pricing/page.tsx:255` calls `persistPlan(tier)`
only (it just records the choice locally). So a user who wants to pay has no path
to checkout. This is the literal "get paid" gap, and it's a small wiring fix.

## What

Wire the **Pro** plan CTA on the pricing page to `useCheckout()` so clicking it
starts a real Stripe Checkout session (→ `POST /api/v1/billing/checkout` →
redirect to the hosted URL). Keep `persistPlan` if it serves a real purpose
(re-verify — it may set the post-checkout return context); the point is the CTA
must additionally invoke checkout.

- Read `web/lib/billing/use-checkout.ts` + `use-portal.ts` to use the existing
  hook exactly as designed (do not write a new checkout client).
- Free tier CTA → routes to signup/onboarding (no charge). Enterprise CTA stays
  as-is for now (it's the EOI path — WS6-2 replaces the `mailto:` with a real
  capture form; do NOT change it in this brief beyond leaving the mailto intact).
- Handle the not-configured case gracefully: if Stripe isn't provisioned yet
  (WS4-2 pending), the checkout call will fail — surface a clean, honest error
  (not a silent no-op, not a fake success). NO THEATER.

## May-touch
- `web/app/pricing/page.tsx` (the Pro CTA handler)
- possibly a small `web/components/...` button/handler if the CTA is extracted
- vitest spec pinning: Pro CTA invokes the checkout hook; failure path renders an
  error (not a fake success)
- `docs/build-plan/BUILD_STATUS.md` (ledger flip)

Do NOT touch the billing backend, `use-checkout.ts`, or the Enterprise mailto.

## Acceptance criteria
- [ ] Clicking the Pro CTA calls `useCheckout()` → `POST /api/v1/billing/checkout`
      (assert via mock in vitest; the hook is the integration point).
- [ ] On a non-configured Stripe (checkout call errors), the UI shows a clean,
      truthful error state — never a fabricated "success".
- [ ] Free CTA still routes to signup/onboarding; Enterprise CTA unchanged.
- [ ] `pnpm typecheck` clean; `pnpm lint` clean; `pnpm vitest run` GREEN on the
      pricing spec (report counts).
- [ ] No regression to the rest of the pricing page.

## Out of scope
- Provisioning Stripe keys/price IDs/webhook secret in prod → **WS4-2** (infra).
- Tier→capability runtime enforcement → **WS4-3** (backend).
- The self-serve /billing portal + usage meter → **WS4-4**.
- Enterprise EOI capture form → **WS6-2**.

## Parallel-surface check
- `rg 'useCheckout|billing/checkout' web/` — confirm the existing hook is the one
  to use; do not add a second checkout path.

## Source
`docs/skycot/audit/2026-06-04-day0-sprint-gap-audit.md` (WS4). Hook:
`web/lib/billing/use-checkout.ts`. Endpoint: `POST /api/v1/billing/checkout` (live).
