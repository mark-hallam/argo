-- Stream `claude-worker-presence-tracking` (mission claim 1).
-- Adds `worker.last_active_at` so operators can answer "which of
-- my workers is actually working right now?" without scanning
-- every audit row. Older rows = NULL = "never seen".
-- SQLite has no portable `ADD COLUMN IF NOT EXISTS`; the runtime
-- helper `worker_presence::ensure_schema` guards re-runs.
ALTER TABLE worker ADD COLUMN last_active_at INTEGER;

CREATE INDEX IF NOT EXISTS idx_worker_presence_tenant_last_active
    ON worker(tenant_id, last_active_at DESC);
