-- migration 136: platform_staff (DEC-057 Phase 1 — audited platform-staff console)
--
-- Skycot's OWN operators. A platform-staff principal lives OUTSIDE any tenant
-- and has NO standing access to tenant data -- it exists to run account-level
-- operations (read account status, suspend/reactivate a tenant) with every
-- action audited. Distinct from user_tenant.role: a tenant role never confers
-- platform-staff power, and staff is default-deny -- the ABSENCE of a row
-- means "not staff".
--
-- Columns:
--   user_id    BLOB    UserId UUID as 16 bytes (PRIMARY KEY)
--   role       TEXT    support | billing_ops | super_admin
--   status     TEXT    active | disabled
--   created_at INTEGER unix seconds
--   created_by TEXT    granting principal for audit attribution (nullable)
--
-- Applied lazily by argo_store::tenant_query::ensure_tenant_substrate_schema,
-- exactly like the sibling identity substrate tables (106 tenant / 107
-- user_account / 108 user_tenant), so no schema-version bump is required.
CREATE TABLE IF NOT EXISTS platform_staff (
    user_id    BLOB    PRIMARY KEY,
    role       TEXT    NOT NULL DEFAULT 'support',
    status     TEXT    NOT NULL DEFAULT 'active',
    created_at INTEGER NOT NULL,
    created_by TEXT
);

CREATE INDEX IF NOT EXISTS idx_platform_staff_status ON platform_staff(status);
