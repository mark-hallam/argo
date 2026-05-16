-- Stream `claude-checkpoint-resume` (V1.3) — workflow
-- checkpoint + resume.
--
-- After each task in a `workflow_run` succeeds, the executor
-- writes a `workflow_checkpoint` row capturing that task's
-- output. On resume, the executor pre-populates outputs from
-- these rows and skips already-checkpointed tasks.
--
-- `cache_key` (optional, set per-task) gives whole-workflow
-- idempotency: when a task declares one and a checkpoint with
-- the same key already exists in any prior run, the task's
-- output is reused without re-execution. Mainly for skipping
-- expensive LLM calls on retry.

CREATE TABLE IF NOT EXISTS workflow_checkpoint (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    workflow_run_id TEXT NOT NULL,
    task_id TEXT NOT NULL,
    -- Microseconds since epoch.
    completed_at INTEGER NOT NULL,
    -- JSON-encoded `BTreeMap<String, serde_json::Value>` of
    -- the task's emitted outputs.
    outputs_json TEXT NOT NULL,
    -- Optional idempotency key. Distinct per task so different
    -- workflows can share cached output for the same expensive
    -- step (e.g. "summarise-2026-Q1-deals").
    cache_key TEXT,
    -- Per-run sequence number (1, 2, 3, ...) for rollback /
    -- pretty listing. Set by the writer.
    seq INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_workflow_checkpoint_run
    ON workflow_checkpoint(workflow_run_id, seq);

CREATE INDEX IF NOT EXISTS idx_workflow_checkpoint_cache_key
    ON workflow_checkpoint(cache_key)
    WHERE cache_key IS NOT NULL;

-- A run is "pending" when its `workflow_run.finished_at` is
-- NULL (the executor crashed mid-flight). `argo workflow
-- resume` scans for these and offers to resume.
