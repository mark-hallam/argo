-- Stream `mission-skill-ratings-publisher` (MISSION_BACKLOG Section 3
-- Claim 4 #80) — operator thumbs on marketplace-installed skills
-- aggregate up to the publisher's `skill_rating` table.
--
-- One row per (skill_id, skill_version, rater_tenant) — re-voting
-- replaces the existing row through the composite primary key.
-- `publisher_tenant` is the publisher namespace string parsed from
-- the installed skill's `provenance.source` (`published:<grade>:<pub>`);
-- v1 has no first-class publisher tenant id so this is the closest
-- stable handle. `rater_tenant_id` carries the rater's real
-- `TenantId` uuid bytes from the workspace tenant context.
--
-- Slot 58 — next free raw-SQL filename slot after
-- 57/skill_version_migration (PR #1317, merged). Versioned
-- counterpart lives at `crates/argo-store/src/migrations/v54.rs`
-- (filename slot and `schema_version` are independent counters per
-- the v42 wrapper's docstring).

CREATE TABLE IF NOT EXISTS skill_rating (
    skill_id            BLOB NOT NULL,
    skill_version       TEXT NOT NULL,
    publisher_tenant    TEXT NOT NULL,
    rater_tenant_id     BLOB NOT NULL,
    vote                TEXT NOT NULL CHECK (vote IN ('up','down')),
    rated_at            INTEGER NOT NULL,
    comment             TEXT,
    PRIMARY KEY (skill_id, skill_version, rater_tenant_id)
);

CREATE INDEX IF NOT EXISTS idx_skill_rating_by_skill
    ON skill_rating(skill_id, skill_version);

CREATE INDEX IF NOT EXISTS idx_skill_rating_by_publisher
    ON skill_rating(publisher_tenant);
