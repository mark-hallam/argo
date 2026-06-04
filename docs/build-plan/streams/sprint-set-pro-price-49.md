# sprint-set-pro-price-49

**Owner**: claude or codex. **Operator decision 2026-06-04: Pro = $49/mo (raise product to match the marketing site).**
**Depends on**: none (independent; ship now).
**Authored against**: origin/main `1260da9fb` on `2026-06-04`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh sprint-set-pro-price-49`.

## Why

The public marketing site advertised Pro at **$49/mo** while the product + canonical
`v1-tiers.toml` + the Stripe display all said **$29/mo** — a public-vs-product discrepancy.
The operator's call (2026-06-04): **$49 is the real price — raise the product to match.** This
brief aligns the product display + canonical config (+ tests) to $49.

**Critical distinction (verified):** the actual Stripe CHARGE is driven by a Stripe
`price_id` (`crates/argo-billing/src/checkout.rs:23,53` → `line_items[0][price]`), NOT by
`v1-tiers.toml`'s `price_monthly_usd` (which is DISPLAY only). So this brief changes what the
product DISPLAYS; the actual charge becoming $49 requires a **$49 Stripe price created in the
Stripe dashboard + its price_id wired** — that is an OPERATOR/infra action (document it, don't
fake it). Get the display right + the seed doc right; flag the Stripe-price creation.

## What
1. **Display → $49:** `web/app/pricing/page.tsx` (the Pro `price` + the FAQ "$29/month" line),
   `web/app/pricing/layout.tsx` (if it carries the price in metadata/copy),
   `docs/skycot/pricing/v1-tiers.toml` `[pro] price_monthly_usd = 29 → 49`.
2. **Stripe seed doc:** `docs/skycot/pricing/stripe-seed.md` → the Pro price = $49 (the operator
   creates the $49 Stripe price; the brief updates the doc + names the env var holding the price_id).
3. **Update every `$29`/`"29"` pin** so CI/tests stay green — `rg '\$29|"29"|price_monthly_usd = 29|29/month' web/ docs/skycot/pricing crates/` at claim time; update the web pricing-page tests + any toml/price assertion.

NO THEATER: do NOT claim the charge is $49 in copy if only the display changed — the page shows
$49, and the brief's PR body states the Stripe-price creation is the operator's remaining step.

## May-touch
- `web/app/pricing/page.tsx` + `layout.tsx` + their tests
- `docs/skycot/pricing/v1-tiers.toml` + `stripe-seed.md` + any toml/price test
- `docs/build-plan/BUILD_STATUS.md` (ledger flip)

Do NOT touch `marketing/app/pricing` — `marketing/` is being deleted (see sprint-consolidate-marketing-into-web-single-app).

## Acceptance criteria
- [ ] web/ pricing page + FAQ + `v1-tiers.toml [pro]` all show **$49**; no residual `$29` for Pro anywhere (`rg` clean).
- [ ] All previously-`$29`-pinned tests updated to `$49`; `pnpm typecheck` + `pnpm vitest run` GREEN; `cargo test -p argo-identity` (tier config) GREEN.
- [ ] `stripe-seed.md` documents the $49 Pro price; PR body explicitly states "actual charge requires the operator to create a $49 Stripe price + wire its price_id" (the display alone doesn't change the charge).
- [ ] No file >500 LOC.

## Source
Operator decision 2026-06-04 (AskUserQuestion). Evidence: `docs/skycot/audit/2026-06-04-marketing-vs-web-app-duplication.md`.
