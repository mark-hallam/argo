-- mission-pm-scheduled-skill-reeval — trend history for scheduled
-- skill eval-set re-runs.
--
-- SAFETY: global table because skills are a cross-tenant marketplace
-- catalog (see `skill` in v1.rs). Re-eval runs target the global
-- catalog by `skill_id`; results apply uniformly across tenants.
--
-- Slot 121 intentionally skips currently reserved 119/120 work in
-- open PRs. The table is lazily applied by
-- `argo_skill::scheduled_reeval_cron::ensure_schema`, matching the
-- post-v70 raw-SQL bootstrap pattern used by magic-link and data
-- export jobs.

CREATE TABLE IF NOT EXISTS skill_reeval_history (
    eval_run_id        TEXT    PRIMARY KEY,
    skill_id           TEXT    NOT NULL,
    skill_name         TEXT    NOT NULL,
    skill_version      TEXT    NOT NULL,
    run_at             INTEGER NOT NULL,
    pass_rate          REAL,
    previous_pass_rate REAL,
    regressed          INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_skill_reeval_history_skill
    ON skill_reeval_history(skill_id, run_at DESC);

CREATE INDEX IF NOT EXISTS idx_skill_reeval_history_regressed
    ON skill_reeval_history(regressed, run_at DESC);
