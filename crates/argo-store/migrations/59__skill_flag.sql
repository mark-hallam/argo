-- Stream `mission-forge-operator-feedback-eval-signal` (MISSION_BACKLOG
-- Section 3 Claim 4 #85) — per-skill operator flags consulted by the
-- Forge ingestion path before pulling operator feedback as eval cases.
--
-- The schema is intentionally generic so future per-skill flags
-- piggyback on the same table without further migrations: one row
-- per (skill_id, flag) pair, value as INTEGER (0/1 for booleans,
-- arbitrary scalars for thresholds).
--
-- Today only `no-forge-feedback` is consulted by
-- `argo-forge::feedback_eval::ingest_operator_feedback`; the column
-- is TEXT (skill_name string) to match the existing
-- `skill_quality_score` and `skill_canary` conventions — operator
-- CLI input `argo skill set-flag <name> --no-forge-feedback`
-- expects a name string, not a binary id blob.

CREATE TABLE IF NOT EXISTS skill_flag (
    skill_id        TEXT NOT NULL,
    flag            TEXT NOT NULL,
    value           INTEGER NOT NULL,
    PRIMARY KEY (skill_id, flag)
);

CREATE INDEX IF NOT EXISTS idx_skill_flag_flag
    ON skill_flag(flag);
