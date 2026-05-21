-- claude-stripe-subscription-integration — Stripe webhook idempotency
-- dedup. The Stripe `evt_…` id is the primary key; a second delivery
-- of the same event becomes an `INSERT OR IGNORE` no-op and the
-- webhook handler short-circuits via
-- `argo_billing::tier_store::has_processed_event`.
--
-- SAFETY: global table — Stripe events arrive on the deployment-level
-- webhook URL before the payload is parsed for tenant routing. Dedup
-- by raw `evt_…` id; the routed tenant is recorded on
-- `stripe_customer` and `billing_event_dead_letter`.

CREATE TABLE IF NOT EXISTS stripe_event (
    event_id     TEXT    PRIMARY KEY,
    received_at  INTEGER NOT NULL
);
