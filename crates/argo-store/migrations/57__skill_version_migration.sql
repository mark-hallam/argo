-- 54__skill_version_migration.sql — publisher-side migration record
-- linking an old skill version to a successor, optionally
-- auto-upgradable. Read by `argo skill upgrade`, `argo skill
-- list-migrations`, and the agent session-boot hint.
--
-- Slot rationale: filename slots 49..53 are taken by
-- worker_archived_at, worker_trust_grade, audit_worker_deleted_at +
-- worker_skill, persona_revision, and forge_run_cost respectively;
-- 54 is the next free filename slot. Reconciles versioned migration
-- v50 (`mission-skill-version-migration-path`).

CREATE TABLE IF NOT EXISTS skill_version_migration (
    skill_id            BLOB NOT NULL,
    from_version        TEXT NOT NULL,
    to_version          TEXT NOT NULL,
    publisher_tenant_id BLOB NOT NULL,
    reason              TEXT,
    auto_upgrade        INTEGER NOT NULL DEFAULT 0,  -- bool
    created_at          INTEGER NOT NULL,
    PRIMARY KEY (skill_id, from_version, to_version)
);

CREATE INDEX IF NOT EXISTS idx_skill_version_migration_to
    ON skill_version_migration(skill_id, to_version);
