-- Stream `mvp-identity-archive-worker` — soft-delete column on
-- `worker` (MISSION_BACKLOG #40, archive half — hard-delete deferred).
--
-- `archived_at` is a nullable ISO-8601 timestamp string. NULL means
-- "active" (the default for every existing row); a non-NULL value
-- means the worker was retired via `IdentityStore::archive_worker`.
-- Default `list_workers` excludes archived rows; callers that need
-- full history pass `include_archived=true`.
--
-- Audit and handoff rows reference the textual `worker_id` and are
-- deliberately left untouched, mirroring the heavier freeze/wake
-- mechanism in `argo_store::worker_archive` (slot 31). The two
-- features are deliberately distinct: this file marks a live row
-- as retired; `worker_archive` removes the live row entirely after
-- snapshotting it to a sibling table.

ALTER TABLE worker ADD COLUMN archived_at TEXT;

CREATE INDEX IF NOT EXISTS idx_worker_archived_at
    ON worker(tenant_id, archived_at);
