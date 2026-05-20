-- Stream `web-gateway-endpoints-notifications-batch` — persistence
-- substrate for the bell-popover notifications API (4 new gateway
-- endpoints landed by the same brief).
--
-- One row per system-level workspace event addressed at a specific
-- principal (or broadcast NULL principal). The bell popover and unread
-- counter both read this table directly; gateway dismiss/dismiss-all
-- handlers stamp the `dismissed_at` column rather than deleting rows
-- so the audit chain stays intact.
--
-- Slot rationale: filename slot 123 (`session_grant_target`) is the
-- highest committed slot on origin/main, so 124 is the next free
-- filename slot. The versioned migration counter is v75; this brief
-- registers v76. Filename and version counters advance independently
-- per the v55 convention.

CREATE TABLE IF NOT EXISTS notifications (
    id                TEXT      NOT NULL PRIMARY KEY,
    tenant_id         TEXT      NOT NULL,
    principal_id      TEXT      NULL,
    kind              TEXT      NOT NULL,
    body              TEXT      NOT NULL,
    created_at        INTEGER   NOT NULL,
    read_at           INTEGER   NULL,
    dismissed_at      INTEGER   NULL,
    related_audit_id  TEXT      NULL,
    severity          TEXT      NOT NULL DEFAULT 'info'
);

CREATE INDEX IF NOT EXISTS idx_notifications_inbox
    ON notifications(tenant_id, principal_id, dismissed_at);

CREATE INDEX IF NOT EXISTS idx_notifications_created_desc
    ON notifications(tenant_id, created_at DESC);
