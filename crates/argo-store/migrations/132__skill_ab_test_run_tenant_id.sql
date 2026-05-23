-- Schema migration v80 — add tenant_id to skill_ab_test_run.
--
-- Source: evaluator round-9 PR #2081 finding §2 P0 #2.1 — the
-- A/B-test run table is workspace-global today, so tenant B can read
-- and promote tenant A's run rows. The companion runtime bind ships
-- in mission-skill-ab-test-run-tenant-id-bind once this lands.
--
-- Additive: existing rows backfill to `__migration_placeholder__`; an
-- operator-facing re-tenant CLI is queued separately (mirrors the
-- parent `skill` table plan in mission-skill-table-add-tenant-id-column).
-- The composite indexes replace the pre-existing
-- `idx_skill_ab_test_run_status` so the canonical `running` lookup
-- stays tenant-scoped.
ALTER TABLE skill_ab_test_run ADD COLUMN tenant_id TEXT NOT NULL DEFAULT '__migration_placeholder__';
CREATE INDEX IF NOT EXISTS idx_skill_ab_test_run_tenant_skill ON skill_ab_test_run(tenant_id, skill_name);
DROP INDEX IF EXISTS idx_skill_ab_test_run_status;
CREATE INDEX IF NOT EXISTS idx_skill_ab_test_run_status ON skill_ab_test_run(tenant_id, status, started_at DESC);
