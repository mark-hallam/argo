-- Stream `claude-skill-versioning-ab-test` (V2.0) — A/B test
-- framework for skill variants.
--
-- Two tables: `skill_ab_test_run` carries the run definition
-- (variant_a + variant_b skill ids + target invocations + status),
-- `skill_ab_test_outcome` carries one row per recorded invocation
-- (which variant fired + did it succeed + latency_ms). Significance
-- check is computed lazily from the outcome rows so we don't need
-- to denormalise running totals into the run row.
--
-- Versioning: we re-use the existing `skill(name, version)` UNIQUE
-- pair from migration v1 — no new `skill_version` table is needed
-- since multiple versions of the same skill already coexist as
-- separate `skill` rows.

CREATE TABLE IF NOT EXISTS skill_ab_test_run (
    id              TEXT PRIMARY KEY,
    skill_name      TEXT NOT NULL,
    variant_a       TEXT NOT NULL,
    variant_b       TEXT NOT NULL,
    target_runs     INTEGER NOT NULL,
    started_at      INTEGER NOT NULL,
    completed_at    INTEGER,
    status          TEXT NOT NULL,
    winner          TEXT,
    UNIQUE(skill_name, variant_a, variant_b, started_at)
);

CREATE INDEX IF NOT EXISTS idx_skill_ab_test_run_status
    ON skill_ab_test_run(status, started_at DESC);

CREATE TABLE IF NOT EXISTS skill_ab_test_outcome (
    run_id          TEXT NOT NULL REFERENCES skill_ab_test_run(id) ON DELETE CASCADE,
    invocation_id   TEXT NOT NULL,
    variant_chosen  TEXT NOT NULL,
    succeeded       INTEGER NOT NULL,
    latency_ms      INTEGER,
    recorded_at     INTEGER NOT NULL,
    PRIMARY KEY (run_id, invocation_id)
);

CREATE INDEX IF NOT EXISTS idx_skill_ab_test_outcome_run
    ON skill_ab_test_outcome(run_id, recorded_at);
