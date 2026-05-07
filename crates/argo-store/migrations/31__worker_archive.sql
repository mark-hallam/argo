-- Stream `claude-worker-archive-rehydrate` (mission claim 1: persistent identity).
-- Sleeping a worker freezes its identity (Worker JSON + persona + role +
-- duties) into a single row here, then deletes it from the live `worker`,
-- `persona`, `role`, and `duty` tables. Audit log + handoff history are
-- left intact (they reference WorkerId, not these rows). Rehydrate is the
-- inverse: read the snapshot, re-create through the regular IdentityStore
-- insert path, drop the archive row.
CREATE TABLE IF NOT EXISTS worker_archive (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    archived_at INTEGER NOT NULL,
    archived_by_principal TEXT,
    reason TEXT,
    frozen_json TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_worker_archive_tenant
    ON worker_archive(tenant_id, archived_at DESC);
