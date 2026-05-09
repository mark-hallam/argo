-- Stream `claude-worker-okr-cli` (mission claim 2: workers have goals).
-- Quarterly objectives declared per-worker. KPIs in `worker_kpi_value`
-- record observed values, OKRs declare *intent* — what the worker is
-- trying to achieve in a given period (`YYYYQ[1-4]`, clap-validated).
-- `status` enum mirrors `active|achieved|missed|cancelled` and is
-- stored as TEXT so future variants land additively.
CREATE TABLE IF NOT EXISTS worker_objective (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    worker_id TEXT NOT NULL,
    tenant_id TEXT NOT NULL,
    period TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL DEFAULT 'active',
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_worker_objective_worker
    ON worker_objective(worker_id, tenant_id, period, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_worker_objective_tenant
    ON worker_objective(tenant_id, period);
