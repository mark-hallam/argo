# DEC-035 — Pro price = $49/mo + single front-end app at both domains

- **Date:** 2026-06-04
- **Status:** active
- **Relates to:** DEC-034 (the paying-ready sprint). Triggered by the Windows lane B infra
  lane surfacing a public-vs-product pricing drift + a duplicated-frontend architecture.

## Context

Investigating "what is Vercel for?" the Windows infra lane found two issues
(`docs/skycot/audit/2026-06-04-marketing-vs-web-app-duplication.md`):
1. Two separate Next.js apps — `marketing/` (skycot.com apex, public site) + `web/`
   (app.skycot.com, the product) — with 5 overlapping route families.
2. They had **drifted on the Pro price**: the public marketing site said **$49/mo**, while the
   product + `v1-tiers.toml` + the Stripe display said **$29/mo**.

The operator decided (AskUserQuestion, 2026-06-04):

## Decision

### 1. Pro tier price = **$49/mo** (raise the product to match the marketing site)
Not $29. Canonical `docs/skycot/pricing/v1-tiers.toml [pro] price_monthly_usd = 49`. The
DISPLAY (web/ pricing page + FAQ + SEO metadata + `tier.rs` doc + `stripe-seed.md` →
`--unit-amount 4900`) was updated to $49 in #2702 (merged), with the `tier_tests.rs` assertion
flipped 29→49.

**OPERATOR ACTION REMAINING (the display is not the charge):** the actual Stripe charge is
driven by a Stripe `price_id` (`crates/argo-billing/src/checkout.rs:23` → `line_items[0][price]`),
NOT by `v1-tiers.toml`. To actually bill $49, the operator must **create a $49 Pro price in the
Stripe dashboard and wire its `price_id`** (per the updated `stripe-seed.md`). Until then the
product displays $49 but would charge whatever the configured price_id is.

### 2. Front-end architecture = **ONE app serving both domains** (delete `marketing/`)
Not two apps, not single-sourced-but-separate. `web/` becomes the single front-end: it serves
skycot.com (apex + www, public pages) AND app.skycot.com (the authenticated product, under
`(app)/`). The `marketing/` app is deleted. This kills the duplicate-route drift at its root
(the $49/$29 discrepancy was the symptom). Implemented by brief
`sprint-consolidate-marketing-into-web-single-app` (#2703, in review — 107 files; under
conductor review before merge).

**OPERATOR ACTION REMAINING:** attaching `skycot.com` (+ www) to `web/`'s Vercel project + the
DNS records is the operator's step (the brief produces a runbook; no automated DNS/domain change).

## Consequences
- A fresh session: Pro is **$49** (the $29 in older docs/memory is superseded). The product is a
  **single `web/` app**; do not re-create or maintain a separate `marketing/` app.
- The two operator-only steps above (Stripe $49 price_id; skycot.com domain attach) gate the
  price + the consolidation from being fully live.

## Evidence
Operator AskUserQuestion 2026-06-04 ("$49 — raise product to match"; "one app at both domains").
Audit: `docs/skycot/audit/2026-06-04-marketing-vs-web-app-duplication.md`. PRs: #2702 (price), #2703 (consolidation).
