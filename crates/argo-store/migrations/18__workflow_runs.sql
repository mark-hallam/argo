-- Stream `claude-workflow-dag` (V1.3) — workflow run history.
--
-- Every `argo workflow run` (non-dry-run) writes one row at start
-- and updates it at end. The CLI reads this table to surface
-- `argo workflow status <run-id>` and `argo workflow list-runs`.
--
-- Verdicts per task land in `task_verdicts_json` as a serialised
-- map (task_id → TaskVerdict). The wire shape mirrors
-- argo_workflow::TaskVerdict so future operators can deserialise
-- back into a typed report.

CREATE TABLE IF NOT EXISTS workflow_run (
    id TEXT PRIMARY KEY,
    workflow_id TEXT NOT NULL,
    workflow_yaml TEXT NOT NULL,
    started_at INTEGER NOT NULL,
    finished_at INTEGER,
    ok INTEGER,
    task_verdicts_json TEXT
);

CREATE INDEX IF NOT EXISTS idx_workflow_run_started_at
    ON workflow_run(started_at DESC);
