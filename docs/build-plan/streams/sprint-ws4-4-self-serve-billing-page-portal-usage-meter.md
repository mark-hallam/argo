# sprint-ws4-4-self-serve-billing-page-portal-usage-meter

**Owner**: claude or codex. **Frontend, small — completes the self-serve "get paid" surface.**
**Depends on**: none (the endpoints are live: `GET /api/v1/tenant/usage` + `use-portal.ts` + `use-checkout.ts`).
**Authored against**: origin/main `b32525458` on `2026-06-04` (verified: `web/lib/billing/use-portal.ts` + `use-checkout.ts` exist; `GET /api/v1/tenant/usage` live per the gap-audit WS4).
**Validate before claim**: `bash scripts/check-brief-freshness.sh sprint-ws4-4-self-serve-billing-page-portal-usage-meter`.

## Why

WS4 shipped the pricing→checkout CTA (#2677) + tier-capability enforcement (#2698), but a
signed-in user has **no in-app billing surface** — they can't see how much of their plan
they've used or manage their subscription. The backend already exposes everything needed
(`GET /api/v1/tenant/usage` returns tier + used vs `max_actions_per_month`; `use-portal.ts`
opens the Stripe customer portal). This brief surfaces them as a real `/billing` page —
the last piece of the self-serve "get paid" loop a paying user expects.

## What

A signed-in `/billing` page (under the app shell, locked brand) showing:
- **Usage meter** — read `GET /api/v1/tenant/usage` (live): current tier, actions used vs the
  tier's `max_actions_per_month`, a progress bar, and the worker count vs `max_workers`. Real
  data only — NO fabricated numbers (NO THEATER); honest empty/zero state if usage is 0.
- **Manage subscription** — a button that opens the Stripe customer portal via the existing
  `usePortal()` hook (`web/lib/billing/use-portal.ts` → `POST /api/v1/billing/portal`). Honest
  error if Stripe isn't provisioned yet (don't fake).
- **Upgrade CTA** (for Free tier) → reuse the pricing `useCheckout()` path (don't fork it).

Re-verify the exact usage-endpoint shape + the hook signatures at claim time
(`rg 'tenant/usage|usePortal|useCheckout' web/lib`). Consume the LOCKED brand tokens
(purple→PINK, Inter; `globals.css`) — no new design exploration (DEC-027).

## May-touch
- `web/app/(app)/billing/page.tsx` (NEW) + a usage-meter component
- `web/lib/api/` usage fetcher (reuse the existing one if present; don't fork)
- nav entry for /billing (settings or top-level — match the established nav pattern)
- vitest for the page (usage renders from live data; portal button calls the hook; honest error)
- `docs/build-plan/BUILD_STATUS.md` (ledger flip)

Do NOT touch the billing backend or the checkout/portal hooks (reuse them).

## Acceptance criteria
- [ ] `/billing` renders the real usage meter from `GET /api/v1/tenant/usage` (tier, actions used/cap, worker count/cap); zero-usage shows an honest empty state, never fabricated counts.
- [ ] "Manage subscription" invokes `usePortal()`; non-configured Stripe surfaces a real error (no fake success).
- [ ] Free-tier upgrade CTA routes through the existing `useCheckout()` (no parallel checkout path).
- [ ] `pnpm typecheck` + `pnpm lint` clean; `pnpm vitest run` GREEN on the new specs (report counts).
- [ ] Locked brand tokens only; no new design.

## Out of scope
- Provisioning Stripe keys in prod (WS4-2 / infra); the tier-gate enforcement (WS4-3, shipped).

## Parallel-surface check
- `rg 'app/\(app\)/billing|usePortal|useCheckout' web/` — confirm no existing /billing page; reuse the hooks.

## Source
`docs/skycot/audit/2026-06-04-day0-sprint-gap-audit.md` (WS4-4).
