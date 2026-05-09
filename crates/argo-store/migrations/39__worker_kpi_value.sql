-- Stream `claude-worker-kpi-tracker` (mission claim 2: KPI runtime evidence).
-- Per-worker KPI time-series store. Each row is one observed numeric value
-- of a named KPI for a worker, optionally annotated with a free-text note.
-- KPI names are not validated against the worker's charter `kpi_json`
-- definition list — the first MVP need is observable values, definition
-- enforcement can land as a follow-up.
CREATE TABLE IF NOT EXISTS worker_kpi_value (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    worker_id TEXT NOT NULL,
    tenant_id TEXT NOT NULL,
    kpi_name TEXT NOT NULL,
    value REAL NOT NULL,
    observed_at INTEGER NOT NULL,
    note TEXT
);

CREATE INDEX IF NOT EXISTS idx_worker_kpi_value_worker
    ON worker_kpi_value(worker_id, tenant_id, kpi_name, observed_at DESC);

CREATE INDEX IF NOT EXISTS idx_worker_kpi_value_tenant
    ON worker_kpi_value(tenant_id, observed_at DESC);
