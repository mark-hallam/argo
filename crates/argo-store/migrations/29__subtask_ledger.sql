-- Stream `claude-subtask-ledger-store` (V1.3 — durable Planner output).
--
-- Persists every subtask the Planner emits when it decomposes a
-- coarse goal into a workflow DAG. One row per (parent_goal, plan
-- node) so that:
--
--   * decompositions are auditable (which goal -> which subtasks?)
--   * restarts can resume mid-decomposition (durability)
--   * operators can inspect a worker's plan via `argo subtask ...`
--   * the v2.0 Dreams integration can analyse planner cohorts
--     (sessions whose parent_goal matches by cosine similarity)
--
-- Row lifecycle: `pending` on insert, `running` when the runner
-- picks the node up, then one of `done` / `failed` / `skipped`.
-- `output` is a JSON object filled when the row enters `done`.

CREATE TABLE IF NOT EXISTS subtask_ledger (
    -- UUIDv7 string id (sortable insert order).
    id TEXT PRIMARY KEY,
    -- Coarse goal string the Planner decomposed.
    parent_goal TEXT NOT NULL,
    -- Optional Session this subtask belongs to. NULL for plans
    -- emitted by `argo plan generate` without `--apply`.
    parent_session_id TEXT,
    -- Owning worker (typically `ctx.worker_id`).
    worker_id TEXT NOT NULL,
    -- DAG path like `/<workflow_id>/<task_id>`. Stable per
    -- (parent_goal, task) pair so updates can find the row by
    -- path when the runner doesn't yet know the row id.
    plan_node_path TEXT NOT NULL,
    -- Human-readable description (task id + workflow description).
    description TEXT NOT NULL,
    -- Lifecycle: pending / running / done / failed / skipped.
    state TEXT NOT NULL DEFAULT 'pending',
    -- Optional JSON-encoded output payload. Filled when `state`
    -- enters `done` and is NULL otherwise.
    output TEXT,
    -- Optional micros-since-epoch for running/done/failed.
    started_at INTEGER,
    completed_at INTEGER,
    -- Created-at micros since epoch.
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_subtask_ledger_session
    ON subtask_ledger(parent_session_id, created_at);

CREATE INDEX IF NOT EXISTS idx_subtask_ledger_worker
    ON subtask_ledger(worker_id, created_at);

CREATE INDEX IF NOT EXISTS idx_subtask_ledger_state
    ON subtask_ledger(state, created_at);

CREATE INDEX IF NOT EXISTS idx_subtask_ledger_parent_goal
    ON subtask_ledger(parent_goal);

-- A worker should never have two pending rows on the same plan
-- node path for the same parent_session — the unique guard catches
-- accidental double-decompose at write time.
CREATE UNIQUE INDEX IF NOT EXISTS idx_subtask_ledger_node_unique
    ON subtask_ledger(parent_session_id, worker_id, plan_node_path);
