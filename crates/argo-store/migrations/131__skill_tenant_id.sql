-- Stream `mission-skill-table-add-tenant-id-column`. Source: evaluator
-- round-9 PR #2081 finding §2 P0 #2.3 — the `skill` table at
-- `migrations/v1.rs:107-119` has NO tenant_id column at all, so every
-- tenant's workers can read every other tenant's skills. Mission
-- claim 6 (multi-tenant by construction) is architecturally violated
-- at the skill primitive.
--
-- Additive migration: ALTER TABLE ADD COLUMN with a sentinel default
-- so existing rows back-fill without a data-rewrite. The placeholder
-- '__migration_placeholder__' is intentional — it preserves backwards
-- compatibility for legacy SkillStore::install() callers that have no
-- tenant context, and an operator CLI re-tenants those rows in a
-- separate brief. NEW writes through the tenant-aware install path
-- (SkillStore::install_for_tenant) carry the real tenant id.
--
-- Reads filter on `tenant_id = ?` from the tenant-aware methods
-- (e.g. SkillStore::available_for_worker, where the tenant id is
-- already in scope from the worker context). The legacy non-tenant
-- read paths continue to return workspace-global results during the
-- v1 permissive seam window; the canonical strict path is the
-- tenant-aware variants.
--
-- Slot rationale: filename slot 130 (`workflow_test_run`) and version
-- counter v78 are the highest on origin/main as of authoring. This
-- brief takes filename 131 + version v79.

ALTER TABLE skill ADD COLUMN tenant_id TEXT NOT NULL DEFAULT '__migration_placeholder__';
CREATE INDEX IF NOT EXISTS idx_skill_tenant_id ON skill(tenant_id);
