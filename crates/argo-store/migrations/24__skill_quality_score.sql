-- Stream `claude-skill-quality-judge` (V2.0) — per-skill 0..100
-- quality score, recomputed periodically.
--
-- One row per (skill_id, computed_at). Operators query the most
-- recent row for the live score. The history rows back trends and
-- regressions.
--
-- `inputs_json` is a JSON snapshot of the QualityInputs the score
-- was computed from (success_rate, avg_latency_ms, retry_count,
-- p99_latency_ms). Storing the snapshot lets future operators
-- audit "why did the score drop" without recomputing from raw
-- ledger queries.

CREATE TABLE IF NOT EXISTS skill_quality_score (
    skill_id        TEXT NOT NULL,
    score           INTEGER NOT NULL,
    inputs_json     TEXT NOT NULL,
    computed_at     INTEGER NOT NULL,
    PRIMARY KEY (skill_id, computed_at)
);

CREATE INDEX IF NOT EXISTS idx_skill_quality_skill
    ON skill_quality_score(skill_id);
CREATE INDEX IF NOT EXISTS idx_skill_quality_computed
    ON skill_quality_score(computed_at DESC);
