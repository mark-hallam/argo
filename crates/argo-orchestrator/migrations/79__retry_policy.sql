-- Additive per-skill retry policy schema for M6-orch-retry-policy.
CREATE TABLE IF NOT EXISTS skill_retry_policy (
    id TEXT PRIMARY KEY,
    tenant_id BLOB NOT NULL,
    skill_id TEXT NOT NULL,
    ir_version TEXT,
    max_attempts INTEGER NOT NULL,
    backoff TEXT NOT NULL,
    initial_delay_ms INTEGER NOT NULL,
    max_delay_ms INTEGER NOT NULL,
    retry_on TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_skill_retry_policy_tenant_skill
    ON skill_retry_policy(tenant_id, skill_id, enabled);
