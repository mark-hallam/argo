-- Stream `mission-skill-explanation-provenance`.
-- Skill provenance log used by `argo skill explain`.
--
-- Slot 69 is reserved for this brief in BUILD_STATUS.md. Rust
-- migration v69 owns the reversible implementation and backfill.

CREATE TABLE IF NOT EXISTS skill_provenance (
    skill_name      TEXT    NOT NULL,
    skill_version   TEXT    NOT NULL,
    provenance_kind TEXT    NOT NULL,
    actor           TEXT,
    parent_id       TEXT,
    change_reason   TEXT,
    breaking_change INTEGER NOT NULL DEFAULT 0,
    ts              INTEGER NOT NULL,
    PRIMARY KEY (skill_name, skill_version)
);

CREATE INDEX IF NOT EXISTS idx_skill_provenance_name
    ON skill_provenance(skill_name);

INSERT OR IGNORE INTO skill_provenance
    (skill_name, skill_version, provenance_kind, actor, parent_id,
     change_reason, breaking_change, ts)
SELECT name, version, 'Bootstrap', NULL, NULL, 'migration backfill', 0, created_at
FROM skill;
