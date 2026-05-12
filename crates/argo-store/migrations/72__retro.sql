-- Stream `mission-retro-review-ui` (MISSION_BACKLOG Section 6 #209) —
-- worker retrospectives. After a worker completes a batch of actions
-- (or a long-running duty wraps), the operator runs a retro over a
-- time window: aggregated metrics from `audit_record` + a 5-section
-- operator-filled template (Surface / What went well / Surprises /
-- What to change / Action items).
--
-- `metrics_json` is frozen at creation — the operator can re-open the
-- draft to edit the narrative, but the underlying counts don't drift
-- as more audit rows land. `sections_json` is the editable surface;
-- `finalized_at` locks the row (the gateway's PATCH handler refuses
-- mutations once the column is non-null).
--
-- Schema-version slot: Rust counter advances to v60 (next free after
-- v59 = `elevated_view_grant`); raw filename slot 72 follows the
-- pinned header (slot 71 reserved for `mission-approval-bundle-ui`,
-- still CLAIMED on origin/main when this brief landed). Filename and
-- version counters advance independently per the v55 convention.

CREATE TABLE IF NOT EXISTS retro (
    id              BLOB PRIMARY KEY,
    tenant_id       BLOB NOT NULL,
    worker_id       BLOB NOT NULL,
    window_start    INTEGER NOT NULL,
    window_end      INTEGER NOT NULL,
    metrics_json    TEXT NOT NULL,
    sections_json   TEXT NOT NULL,
    created_at      INTEGER NOT NULL,
    finalized_at    INTEGER
);

CREATE INDEX IF NOT EXISTS idx_retro_worker
    ON retro(worker_id, created_at);

CREATE INDEX IF NOT EXISTS idx_retro_tenant
    ON retro(tenant_id, created_at);
