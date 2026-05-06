-- Worker circuit-breaker audit trail.
--
-- Latest row per worker is the current circuit state. Closed
-- rows keep `failure_count` so consecutive failures are durable
-- without mutating earlier audit entries.

CREATE TABLE IF NOT EXISTS worker_circuit_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    worker_id TEXT NOT NULL,
    state TEXT NOT NULL CHECK (state IN ('closed', 'open', 'half_open')),
    failure_count INTEGER NOT NULL DEFAULT 0,
    reason TEXT,
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_worker_circuit_history_worker
ON worker_circuit_history(worker_id, created_at, id);
