-- codex-billing-webhook-resilience — webhook dead-letter queue.
-- Rows are inserted by argo-billing once Stripe's own retry schedule
-- is exhausted (the gateway can't apply the event); the operator
-- `argo billing replay` CLI re-runs from `raw_payload` against the
-- same tier-mutation path and sets `resolved_at` on success.

CREATE TABLE IF NOT EXISTS billing_event_dead_letter (
    event_id        TEXT    PRIMARY KEY,
    event_type      TEXT    NOT NULL,
    tenant_id       BLOB,
    raw_payload     TEXT    NOT NULL,
    failed_at       INTEGER NOT NULL,
    failure_reason  TEXT    NOT NULL,
    retry_count     INTEGER NOT NULL DEFAULT 0,
    resolved_at     INTEGER
);
CREATE INDEX IF NOT EXISTS idx_dlq_failed_at
    ON billing_event_dead_letter(failed_at);
CREATE INDEX IF NOT EXISTS idx_dlq_resolved
    ON billing_event_dead_letter(resolved_at);
