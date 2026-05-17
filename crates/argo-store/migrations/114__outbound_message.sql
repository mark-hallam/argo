-- mission-send-message-production-dispatcher — outbound_message
-- idempotency table for the production `send_message` dispatcher.
-- Bootstrapped lazily via
-- `argo_store::outbound_message_query::ensure_outbound_message_schema`
-- because the versioned migration runner stops at slot 69 (same
-- pattern as `110__stripe_event.sql`).

CREATE TABLE IF NOT EXISTS outbound_message (
    message_id    TEXT    PRIMARY KEY,
    tenant_id     BLOB    NOT NULL,
    channel       TEXT    NOT NULL,
    recipient     TEXT    NOT NULL,
    body_hash     TEXT    NOT NULL,
    sent_at       INTEGER NOT NULL,
    outcome_json  TEXT    NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_outbound_tenant
    ON outbound_message(tenant_id, sent_at);

CREATE INDEX IF NOT EXISTS idx_outbound_body_hash
    ON outbound_message(tenant_id, body_hash, sent_at);
