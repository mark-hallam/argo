-- claude-stripe-subscription-integration — Stripe webhook idempotency
-- dedup. The Stripe `evt_…` id is the primary key; a second delivery
-- of the same event becomes an `INSERT OR IGNORE` no-op and the
-- webhook handler short-circuits via
-- `argo_billing::tier_store::has_processed_event`.

CREATE TABLE IF NOT EXISTS stripe_event (
    event_id     TEXT    PRIMARY KEY,
    received_at  INTEGER NOT NULL
);
