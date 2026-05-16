# Stripe product/price seed — `claude-stripe-subscription-integration`

Operator runbook for seeding the three SKYCOT tiers into Stripe so the
`/billing` page's Checkout button has a real price id to hand to
`POST /api/v1/billing/checkout`.

## Prerequisites

- A Stripe account in **test mode** for v1 weekend; production mode is
  a separate operator workstream.
- The Stripe CLI installed (`brew install stripe/stripe-cli/stripe`)
  and authenticated (`stripe login`).
- Read access to `docs/skycot/pricing/v1-tiers.toml` (the canonical
  tier definitions).

## One-time seed (test mode)

For each tier, create a Product + a recurring Price. The webhook maps
`price.metadata.tier` back to `argo_identity::tier::Tier` — so the
metadata key matters.

```bash
# Free — visible so customers can self-select; checkout returns to the
# /billing page with the tier unchanged (no subscription required).
stripe products create \
  --name "Skycot Free" \
  --metadata tier=free \
  --description "Mock provider, 1 worker, 100 actions / month"

# Pro — $29 / month recurring.
PRO_PRODUCT=$(stripe products create \
  --name "Skycot Pro" \
  --metadata tier=pro \
  --description "Real provider keys, 5 workers, 5,000 actions / month" \
  | jq -r .id)
stripe prices create \
  --product "$PRO_PRODUCT" \
  --unit-amount 2900 \
  --currency usd \
  --recurring interval=month \
  --metadata tier=pro

# Enterprise — custom-quoted. Create a placeholder $0 price so the
# Checkout link works; the operator overrides the line item per
# customer at quote time.
ENT_PRODUCT=$(stripe products create \
  --name "Skycot Enterprise" \
  --metadata tier=enterprise \
  --description "Unlimited workers, SSO, audit export, custom verticals" \
  | jq -r .id)
stripe prices create \
  --product "$ENT_PRODUCT" \
  --unit-amount 0 \
  --currency usd \
  --recurring interval=month \
  --metadata tier=enterprise
```

Record the `price_…` ids and add them to the SPA env:

```bash
NEXT_PUBLIC_STRIPE_PRICE_PRO=price_…
NEXT_PUBLIC_STRIPE_PRICE_ENTERPRISE=price_…
```

(Free needs no price — the `/billing` page disables that button.)

## Webhook configuration

Stripe must POST to `https://<gateway-host>/api/v1/billing/webhook` so
the tier writeback fires. Test-mode local development can forward to
the dev gateway via the Stripe CLI:

```bash
stripe listen --forward-to localhost:8787/api/v1/billing/webhook
```

The CLI prints a `whsec_…` secret. Export it before starting the gateway:

```bash
export STRIPE_SECRET_KEY=sk_test_…
export STRIPE_WEBHOOK_SECRET=whsec_…
```

Required event types (configure in the Stripe Dashboard → Developers →
Webhooks for the production endpoint; the CLI forwards all events
locally):

- `customer.subscription.created`
- `customer.subscription.updated`
- `customer.subscription.deleted`

## Verification (end-to-end, test mode)

1. Start the gateway with `STRIPE_SECRET_KEY` + `STRIPE_WEBHOOK_SECRET` set.
2. From the SPA, click **Upgrade to Pro** → Stripe Checkout opens.
3. Use Stripe test card `4242 4242 4242 4242` with any future expiry +
   any 3-digit CVC + any zip.
4. Checkout completes → Stripe POSTs `customer.subscription.created`.
5. Confirm `argo_store::tenant_query::tenant_tier(...)` now returns
   `"pro"` (or check `GET /api/v1/tenant/usage`).
6. Click **Manage subscription** → Stripe Customer Portal opens for
   that customer; cancellation flow round-trips via
   `customer.subscription.deleted` → tier drops back to `"free"`.

## Production approval (operator)

Stripe account verification (business profile, bank account, tax info)
is a separate workstream. Once approved, replace the test keys with
`sk_live_…` / `whsec_…` and re-run the seed against live mode.
