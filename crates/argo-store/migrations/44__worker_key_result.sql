-- Stream `claude-worker-okr-cli` (mission claim 2: workers have goals).
-- Per-objective key results. Each row binds one declared `kpi_name` to
-- a numeric `target_value` plus `comparison` operator (`gte|lte|eq`) and
-- a `weight` used to compute the weighted progress score. KPI names
-- reference the time-series store in `worker_kpi_value`. FK enforcement
-- against the kpi_name column is intentionally NOT enforced — KPI names
-- are still free-form at the recording layer (#1062 brief).
CREATE TABLE IF NOT EXISTS worker_key_result (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    objective_id INTEGER NOT NULL,
    kpi_name TEXT NOT NULL,
    target_value REAL NOT NULL,
    comparison TEXT NOT NULL,
    weight REAL NOT NULL,
    FOREIGN KEY (objective_id) REFERENCES worker_objective(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_worker_key_result_objective
    ON worker_key_result(objective_id);
