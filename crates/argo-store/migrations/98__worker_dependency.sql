-- Stream `claude-worker-dependency-tracking` (V1.1).
-- Declared dependencies between workers — the structural lens missing
-- when a worker breaks (offline / failing / promoted) and the operator
-- needs to know "what other workers will be impacted". Distinct from
-- `handoff_routing` weights: this table records *declared intent*,
-- routing is *runtime weights*. They co-exist.
--
-- `kind` is stored as TEXT (`data|approval|handoff|skill`) so future
-- variants land additively. Composite PK on (a, b, kind) lets the same
-- pair coexist across kinds (cross-kind cycles are allowed by the
-- brief). Cycle detection within one `kind` runs in the application
-- layer (DFS) at insert time, since SQLite has no graph constraints
-- to express the rule declaratively.
CREATE TABLE IF NOT EXISTS worker_dependency (
    a_worker_id TEXT NOT NULL,
    b_worker_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    declared_at INTEGER NOT NULL,
    note TEXT,
    PRIMARY KEY (a_worker_id, b_worker_id, kind)
);

-- Out-edge index: `list --direction out` and the workspace graph walk.
CREATE INDEX IF NOT EXISTS idx_worker_dependency_out
    ON worker_dependency(a_worker_id, kind);

-- In-edge index: `list --direction in` and `impact` upstream walk.
CREATE INDEX IF NOT EXISTS idx_worker_dependency_in
    ON worker_dependency(b_worker_id, kind);
