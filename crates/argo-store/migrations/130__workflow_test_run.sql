-- Stream `claude-followup-pr-2021-test-run-gateway-persist`
-- (re-author of closed PR #2021 after migration-slot collision at
-- 129/v77). Server-side persistence of each completed workflow test
-- run from the builder's "Test run" overlay (#1971 streams NDJSON
-- but persists nothing; the tick-24 history panel keeps the per-
-- browser run list in localStorage). This table backs the history
-- panel so a workflow's run history survives browser clears and is
-- multi-device.
--
-- One row per completed test run. Column shape matches the builder's
-- `CompletedTestRun` payload (`useTestRun.ts`):
--   - `started_at` / `ended_at` are Unix milliseconds (numeric in
--     the JS hook, persisted verbatim).
--   - `status` is the hook's run phase: "complete" | "failed".
--   - `summary_json` holds the final `RunSummary` blob (ok / fail /
--     durationMs / tokens / costUsd); `'null'` when the run aborted
--     before a `run:summary` event arrived.
--   - `step_states_json` is the per-step `Record<stepId, StepRunState>`
--     snapshot from completion; default `'{}'` for empty runs.
--   - `transport_error` is the hook's `transportError` string —
--     non-null when the NDJSON fetch failed mid-stream.
--
-- Tenant-scoped via the canonical pattern: every read filters on
-- `tenant_id = ?` first; cross-tenant lookups return an empty list
-- (not a 404), per the brief's "GET returns [] for an unknown
-- workflow" acceptance criterion.
--
-- Slot rationale: raw filename 129 (`workplace_knowledge`) and
-- versioned counter v77 are now both occupied on origin/main. This
-- re-author takes the next free slots: filename 130, versioned v78.

CREATE TABLE IF NOT EXISTS workflow_test_run (
    id                TEXT    PRIMARY KEY,
    tenant_id         TEXT    NOT NULL,
    vertical_id       TEXT    NOT NULL,
    workflow_name     TEXT    NOT NULL,
    started_at        INTEGER NOT NULL,
    ended_at          INTEGER NOT NULL,
    status            TEXT    NOT NULL,
    summary_json      TEXT    NOT NULL DEFAULT 'null',
    step_states_json  TEXT    NOT NULL DEFAULT '{}',
    transport_error   TEXT
);

CREATE INDEX IF NOT EXISTS idx_workflow_test_run_lookup
    ON workflow_test_run(tenant_id, vertical_id, workflow_name, ended_at DESC);
