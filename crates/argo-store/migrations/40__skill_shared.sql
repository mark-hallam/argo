-- Stream `claude-skill-share-cross-tenant-poc` — cross-tenant skill
-- sharing PoC. Mission claim 4 ("Cross-tenant sharing with provenance
-- + trust") wedge: a tenant can mark a skill as shared and tenants in
-- the same org_chart cluster can see it (read-only).
--
-- Two additive columns on the existing `skill` version table:
--   * `shared`              — 0/1 flag (default 0; pre-existing skills
--     remain private to whoever installed them).
--   * `shared_with_cluster` — nullable cluster label. NULL means "no
--     cluster declared yet"; a non-NULL value names the org_chart
--     cluster (root org_unit name) authorized to read the skill.
--
-- Read path: `argo_store::skill_share::list_shared_for_tenant` resolves
-- the viewer tenant's cluster (root org_unit name from the org_chart
-- schema in `21__org_chart.sql`) and joins it against the
-- `shared_with_cluster` label on shared skills.
--
-- Down migration drops the columns + the index.
ALTER TABLE skill ADD COLUMN shared INTEGER NOT NULL DEFAULT 0;
ALTER TABLE skill ADD COLUMN shared_with_cluster TEXT;

CREATE INDEX IF NOT EXISTS idx_skill_shared_cluster
    ON skill(shared_with_cluster) WHERE shared = 1;
