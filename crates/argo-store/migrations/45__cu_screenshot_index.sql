-- Stream `claude-cu-screenshot-storage` (mission claim: persist CU
-- screenshots through `argo-store`'s blob layer with per-tenant
-- prefixing + a TTL-driven retention sweep).
--
-- Index of every CU screenshot put through `CuScreenshotStore`.
-- Rows reference the opaque `BlobId` returned by the underlying
-- `BlobStore::put` call; the actual PNG bytes live in whichever
-- backing store implements `BlobStore`. The `blob_mime`, `blob_size_bytes`,
-- and `blob_sha256` columns mirror `BlobRef` so `list_session` can
-- return without a per-row round-trip into the blob store.
--
-- Sweep is index-row only: the trait has no delete method, so
-- production blob backends own their own bytes-level
-- retention/lifecycle.
--
-- `tenant_id` is plumbed for HARD RULE invariant 2 (multi-tenant by
-- construction). `(tenant_id, session_id)` indexes the read path
-- for replay enumeration; `(retention_until)` indexes the sweep.
CREATE TABLE IF NOT EXISTS cu_screenshot (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tenant_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    action_id TEXT NOT NULL,
    step_n INTEGER NOT NULL,
    blob_id TEXT NOT NULL,
    blob_mime TEXT NOT NULL,
    blob_size_bytes INTEGER NOT NULL,
    blob_sha256 BLOB NOT NULL,
    captured_at INTEGER NOT NULL,
    retention_until INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_cu_screenshot_tenant_session
    ON cu_screenshot(tenant_id, session_id, action_id, step_n);

CREATE INDEX IF NOT EXISTS idx_cu_screenshot_retention
    ON cu_screenshot(retention_until);
