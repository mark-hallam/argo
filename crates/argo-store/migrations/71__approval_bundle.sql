-- Stream `mission-approval-bundle-ui` — server-defined approval
-- bundles. Workers that emit a coherent batch of low-risk actions
-- can group them under one bundle id so the operator decides the
-- whole group atomically instead of N individual cards.
--
-- Slot picked: 63 (`elevated_view_grant`) is the highest raw slot on
-- origin/main; slots 64-70 are reserved by the BUILD_STATUS.md
-- migration-slot pin for in-flight peer briefs. Slot 71 is the
-- pinned slot for this brief. Versioned reconciliation lives in
-- `migrations/v60.rs`.
--
-- Two tables:
--   - `approval_bundle`        — bundle row + lifecycle state.
--   - `approval_bundle_member` — join row pointing at the member
--     approval-id (the existing `worker_actions` ActionRecord id;
--     opaque to the store).
--
-- Idempotent: CREATE TABLE/INDEX IF NOT EXISTS. Down migration in
-- `migrations/v60.rs` drops both tables + indexes in reverse order.
CREATE TABLE IF NOT EXISTS approval_bundle (
    id          BLOB PRIMARY KEY,
    tenant_id   BLOB NOT NULL,
    label       TEXT NOT NULL,
    created_at  INTEGER NOT NULL,
    status      TEXT NOT NULL DEFAULT 'pending',
    decided_at  INTEGER,
    decided_by  TEXT,
    decision    TEXT
);

CREATE INDEX IF NOT EXISTS idx_approval_bundle_tenant_status
    ON approval_bundle(tenant_id, status);

CREATE TABLE IF NOT EXISTS approval_bundle_member (
    bundle_id    BLOB NOT NULL,
    approval_id  TEXT NOT NULL,
    member_idx   INTEGER NOT NULL,
    PRIMARY KEY (bundle_id, approval_id),
    FOREIGN KEY (bundle_id) REFERENCES approval_bundle(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_approval_bundle_member_bundle
    ON approval_bundle_member(bundle_id, member_idx);
