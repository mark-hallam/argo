-- Stream `mission-forge-cost-attribution` — per-run cost attribution
-- columns on `forge_run` (MISSION_BACKLOG Section 6 #206).
--
-- Today `forge_run.spend_usd_micros` captures aggregate LLM spend
-- only; this migration breaks the spend out into structured cost
-- attribution: per-call token counts (input / output / cache_read),
-- model-priced LLM cost, E2B sandbox time + cost, and a stamped
-- total. All values are USD-micros (`1_000_000 = $1.00`) so they
-- fit alongside the existing `spend_usd_micros` column. Existing
-- rows default to 0 so the columns are additive.
--
-- Slot rationale: raw 53 picks the next free filename after
-- `52__persona_revision.sql` (versioned v48 on origin/main). The
-- two counters move independently; reconciles versioned migration
-- `v49.rs`.

ALTER TABLE forge_run ADD COLUMN llm_input_tokens INTEGER NOT NULL DEFAULT 0;
ALTER TABLE forge_run ADD COLUMN llm_output_tokens INTEGER NOT NULL DEFAULT 0;
ALTER TABLE forge_run ADD COLUMN llm_cache_read_tokens INTEGER NOT NULL DEFAULT 0;
ALTER TABLE forge_run ADD COLUMN llm_cost_usd_micros INTEGER NOT NULL DEFAULT 0;
ALTER TABLE forge_run ADD COLUMN e2b_seconds INTEGER NOT NULL DEFAULT 0;
ALTER TABLE forge_run ADD COLUMN e2b_cost_usd_micros INTEGER NOT NULL DEFAULT 0;
ALTER TABLE forge_run ADD COLUMN total_cost_usd_micros INTEGER NOT NULL DEFAULT 0;

CREATE INDEX IF NOT EXISTS idx_forge_run_total_cost
    ON forge_run(total_cost_usd_micros DESC);

CREATE INDEX IF NOT EXISTS idx_forge_run_skill_started_at
    ON forge_run(skill_name, started_at DESC);
