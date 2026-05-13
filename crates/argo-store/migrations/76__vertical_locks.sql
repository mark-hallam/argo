CREATE TABLE IF NOT EXISTS vertical_locks (
    tenant_id TEXT NOT NULL,
    vertical_id TEXT NOT NULL,
    worker_id TEXT NOT NULL,
    holder_pid INTEGER NOT NULL,
    acquired_at INTEGER NOT NULL,
    lease_secs INTEGER NOT NULL,
    PRIMARY KEY (tenant_id, vertical_id, worker_id)
) STRICT, WITHOUT ROWID;
