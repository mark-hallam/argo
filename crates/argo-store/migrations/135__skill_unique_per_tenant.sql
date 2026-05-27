-- Stream `fix-skill-table-unique-name-version-globally-blocks-per-tenant-seed`.
-- Source: evaluator round 97 (2026-05-27) — the v80 migration added
-- `tenant_id` to the `skill` table but left the v1 `UNIQUE(name, version)`
-- constraint global, so per-tenant catalog seeding (Scene 4 of the v1.0
-- demo MP4) panics with `UNIQUE constraint failed: skill.name, skill.version`
-- the second a fresh tenant boots against an already-seeded gateway.
-- Fix: relax the constraint to `UNIQUE(name, version, tenant_id)` via the
-- canonical SQLite table-swap (SQLite cannot drop UNIQUE via ALTER TABLE).
--
-- HAZARD 1 — `session_skill.skill_id REFERENCES skill(id) ON DELETE CASCADE`
-- (v4.rs:33). FK enforcement is OFF in the live runtime today (no
-- `PRAGMA foreign_keys = ON` anywhere in `crates/argo-store/`), so
-- `DROP TABLE skill` does not currently cascade-delete `session_skill`
-- rows. But if FKs ever flip on (postgres target, security hardening),
-- the drop would cascade-delete every `session_skill` row before the
-- swap completes. `PRAGMA defer_foreign_keys` defers FK violation
-- *checks* to commit, NOT `ON DELETE CASCADE` actions — so to
-- forward-protect this migration we materialise `session_skill` into
-- a regular (non-TEMP) helper table, empty the live table before the
-- drop, swap, then re-insert. A regular helper table is used instead
-- of `TEMP` because the `tests/migration_chain.rs` raw-replay harness
-- executes each statement on a pool that may hand out different
-- connections; TEMP tables are connection-scoped and would vanish
-- between statements there.
--
-- HAZARD 2 — four auxiliary indexes (v9 idx_skill_vertical_id, v11
-- idx_skill_lifecycle, v41 idx_skill_shared_cluster, v80
-- idx_skill_tenant_id) MUST be recreated post-swap or queries silently
-- regress to full table scans.
--
-- Slot rationale: filename slot 134 (worker_kpis_rolled_at) and version
-- counter v83 are the highest on origin/main. This brief takes filename
-- 135 + version v84.

PRAGMA defer_foreign_keys = ON;

CREATE TABLE IF NOT EXISTS _session_skill_v84_save (
  session_id  BLOB NOT NULL,
  skill_id    BLOB NOT NULL,
  attached_at INTEGER NOT NULL
);
DELETE FROM _session_skill_v84_save;
INSERT INTO _session_skill_v84_save SELECT session_id, skill_id, attached_at FROM session_skill;
DELETE FROM session_skill;

CREATE TABLE skill_new (
  id              BLOB PRIMARY KEY,
  name            TEXT NOT NULL,
  version         TEXT NOT NULL,
  body            TEXT NOT NULL,
  frontmatter     TEXT NOT NULL,
  source          TEXT NOT NULL,
  hash            BLOB NOT NULL,
  learned_in      BLOB,
  superseded_by   BLOB REFERENCES skill(id),
  created_at      INTEGER NOT NULL,
  vertical_id     TEXT,
  lifecycle_state TEXT NOT NULL DEFAULT 'active',
  shared          INTEGER NOT NULL DEFAULT 0,
  shared_with_cluster TEXT,
  min_trust_grade INTEGER,
  tenant_id       TEXT NOT NULL DEFAULT '__migration_placeholder__',
  UNIQUE(name, version, tenant_id)
);

INSERT INTO skill_new (
  id, name, version, body, frontmatter, source, hash,
  learned_in, superseded_by, created_at, vertical_id, lifecycle_state,
  shared, shared_with_cluster, min_trust_grade, tenant_id
)
SELECT id, name, version, body, frontmatter, source, hash,
  learned_in, superseded_by, created_at, vertical_id, lifecycle_state,
  shared, shared_with_cluster, min_trust_grade, tenant_id
FROM skill;

DROP TABLE skill;

ALTER TABLE skill_new RENAME TO skill;

CREATE INDEX idx_skill_vertical_id ON skill(vertical_id);
CREATE INDEX idx_skill_lifecycle ON skill(lifecycle_state);
CREATE INDEX idx_skill_shared_cluster ON skill(shared_with_cluster) WHERE shared = 1;
CREATE INDEX IF NOT EXISTS idx_skill_tenant_id ON skill(tenant_id);

INSERT INTO session_skill (session_id, skill_id, attached_at)
  SELECT session_id, skill_id, attached_at FROM _session_skill_v84_save;
DROP TABLE _session_skill_v84_save;

PRAGMA defer_foreign_keys = OFF;
