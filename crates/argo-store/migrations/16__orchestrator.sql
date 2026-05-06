-- Stream H follow-up: sqlite persistence for orchestrator sessions + handoffs.
--
-- `session` already exists in argo-store v1. This migration extends it with
-- the orchestrator fields, then creates the handoff table. Handoff stores the
-- `EscalationStep` in the `"to"` JSON column using its serde tagged shape.

ALTER TABLE session ADD COLUMN status TEXT NOT NULL DEFAULT 'active';
ALTER TABLE session ADD COLUMN participants TEXT NOT NULL DEFAULT '[]';
ALTER TABLE session ADD COLUMN created_at INTEGER NOT NULL DEFAULT 0;
ALTER TABLE session ADD COLUMN deadline INTEGER;

CREATE INDEX IF NOT EXISTS idx_orchestrator_session_active
ON session(tenant_id, status, created_at);

CREATE TABLE IF NOT EXISTS handoff (
    id TEXT PRIMARY KEY,
    session_id BLOB NOT NULL REFERENCES session(id) ON DELETE CASCADE,
    from_worker_id TEXT NOT NULL,
    "to" TEXT NOT NULL,
    expected_outcome TEXT NOT NULL,
    payload TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    deadline INTEGER
);

CREATE INDEX IF NOT EXISTS idx_handoff_session
ON handoff(session_id, created_at);

CREATE INDEX IF NOT EXISTS idx_handoff_status
ON handoff(status, created_at);
