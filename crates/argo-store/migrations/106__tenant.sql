-- mission-tenant-substrate-tables — operator-facing tenant row table.
-- Additive: existing TenantId-UUID-only rows in workspace, tenant_quota,
-- tenant_rate_limit, tenant_tier, tenant_usage continue to work without a
-- corresponding row here. No FK enforcement from those tables.

CREATE TABLE IF NOT EXISTS tenant (
    id              BLOB    PRIMARY KEY,
    name            TEXT    NOT NULL,
    billing_email   TEXT    NOT NULL,
    status          TEXT    NOT NULL DEFAULT 'active',
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_tenant_billing_email ON tenant(billing_email);
CREATE INDEX IF NOT EXISTS idx_tenant_status        ON tenant(status);
