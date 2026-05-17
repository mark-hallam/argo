-- codex-mcp-rate-limit-rewrite — extend tenant_rate_limit from a
-- tenant-wide override to per-(tenant, connector) overrides.
--
-- Slot 112 is occupied by the in-review billing DLQ migration (#1803),
-- so this brief takes the next reserved raw-SQL slot.

DROP TABLE IF EXISTS tenant_rate_limit_v2;

CREATE TABLE tenant_rate_limit_v2 (
    tenant_id            BLOB    NOT NULL,
    connector            TEXT    NOT NULL DEFAULT '*',
    requests_per_minute  INTEGER NOT NULL,
    updated_at           INTEGER NOT NULL,
    PRIMARY KEY (tenant_id, connector)
);

INSERT OR IGNORE INTO tenant_rate_limit_v2
    (tenant_id, connector, requests_per_minute, updated_at)
SELECT tenant_id, '*', requests_per_minute, updated_at
FROM tenant_rate_limit;

DROP TABLE tenant_rate_limit;
ALTER TABLE tenant_rate_limit_v2 RENAME TO tenant_rate_limit;

CREATE INDEX IF NOT EXISTS idx_tenant_rate_limit_connector
    ON tenant_rate_limit(tenant_id, connector);
