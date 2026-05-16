-- Stream `mvp-worker-skills-table` — worker↔skill association table.
-- Adds the persistence primitive for "which workers have skill S in
-- tenant T?", required by `mission-routing-skillmatch-rolematch` (M4)
-- to resolve `TargetSelector::SkillMatch(String)`.
--
-- `worker_id` is TEXT to match the existing `worker.id TEXT PRIMARY
-- KEY` column type in `12__worker_identity.sql` -- the brief's BLOB
-- spec was authored against an outdated schema assumption.
-- `proficiency` is a 1..5 ordinal (1=novice, 5=expert) — left as
-- INTEGER rather than CHECK-constrained so future tuning lands
-- additively.
--
-- Slot picked: 49__worker_archived_at.sql (PR #1271, merged) and
-- 50__worker_trust_grade.sql (PR #1273, merged) occupy the prior
-- two raw slots, so 51 is the next free slot. Versioned
-- reconciliation lives in v47.rs.
--
-- Idempotent: CREATE TABLE/INDEX IF NOT EXISTS. Down migration drops
-- the index and table (see `migrations/v47.rs`).
CREATE TABLE IF NOT EXISTS worker_skill (
    worker_id TEXT NOT NULL,
    skill_id TEXT NOT NULL,
    proficiency INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (worker_id, skill_id)
);

CREATE INDEX IF NOT EXISTS worker_skill_skill_idx
    ON worker_skill(skill_id);
