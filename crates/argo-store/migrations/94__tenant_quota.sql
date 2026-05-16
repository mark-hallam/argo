-- V1.4: per-tenant resource quotas.
--
-- Rows are keyed by tenant id as text so the quota CLI can accept
-- the same tenant strings carried by the worker identity tables.

CREATE TABLE IF NOT EXISTS tenant_quota (
    tenant_id TEXT PRIMARY KEY,
    period TEXT NOT NULL CHECK (period IN ('daily', 'monthly')),
    max_usd_micros INTEGER,
    max_actions INTEGER,
    max_workers INTEGER,
    set_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_tenant_quota_period
ON tenant_quota(period, set_at);
