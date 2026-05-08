-- Stream `claude-session-worker-id-schema` (Phase B claim-1 closure).
-- Adds `worker_id` to the session table so per-worker memory hydration
-- (#945 follow-up `mission-memory-per-worker-scope`), per-Principal
-- audit stamping (#946 follow-up), and duty→session linkage (#939) can
-- thread the WorkerId end-to-end. Nullable + partial index - every
-- pre-existing session keeps NULL for back-compat.
ALTER TABLE session ADD COLUMN worker_id BLOB;

CREATE INDEX IF NOT EXISTS idx_session_worker
    ON session(worker_id) WHERE worker_id IS NOT NULL;
