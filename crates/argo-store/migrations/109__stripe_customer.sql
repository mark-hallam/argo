-- claude-stripe-subscription-integration — `tenant_id` ↔ Stripe
-- `cus_…` / `sub_…` mapping. Additive; the versioned migration runner
-- stops at slot 69, so `argo_billing::tier_store::ensure_billing_schema`
-- bootstraps this table lazily on first webhook hit. Indexed by
-- `stripe_customer_id` so the webhook can also look up by customer.

CREATE TABLE IF NOT EXISTS stripe_customer (
    tenant_id           BLOB    PRIMARY KEY,
    stripe_customer_id  TEXT    NOT NULL,
    subscription_id     TEXT,
    created_at          INTEGER NOT NULL,
    updated_at          INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_stripe_customer_cust
    ON stripe_customer(stripe_customer_id);
