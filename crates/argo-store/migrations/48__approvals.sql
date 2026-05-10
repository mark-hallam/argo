-- Stream `claude-approval-request-store` — first-class approvals
-- table for the auto-approval gate. Records pending requests, who can
-- grant/deny them, and a deadline + default-decision so a future
-- timeout sweep (downstream brief
-- `claude-approval-timeout-default-decision`) can promote stale rows
-- to a deterministic resolution.
--
-- Slot picked: 47__skill_canary.sql is the highest raw slot on
-- origin/main; 48 is the next free slot. Versioned reconciliation
-- lives in v44.rs.
--
-- Idempotent: CREATE TABLE/INDEX IF NOT EXISTS. Down migration drops
-- the table and both indexes (see `migrations/v44.rs`).
CREATE TABLE IF NOT EXISTS approval_request (
    id                 BLOB PRIMARY KEY,
    action_id          BLOB NOT NULL,
    tenant_id          BLOB NOT NULL,
    requested_at       INTEGER NOT NULL,
    requested_by       TEXT NOT NULL,
    deadline_at        INTEGER NOT NULL,
    default_decision   TEXT NOT NULL,
    status             TEXT NOT NULL DEFAULT 'pending',
    decided_at         INTEGER,
    decided_by         TEXT,
    decision           TEXT
);

CREATE INDEX IF NOT EXISTS idx_approval_request_tenant_status
    ON approval_request(tenant_id, status);

CREATE INDEX IF NOT EXISTS idx_approval_request_deadline
    ON approval_request(deadline_at);
