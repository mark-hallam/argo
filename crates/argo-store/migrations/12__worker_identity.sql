CREATE TABLE IF NOT EXISTS persona (
    id TEXT PRIMARY KEY,
    worker_id TEXT NOT NULL UNIQUE,
    tenant_id TEXT NOT NULL,
    name TEXT NOT NULL,
    voice_preamble TEXT NOT NULL,
    avatar TEXT,
    bio TEXT,
    builder_attribution TEXT NOT NULL,
    provenance_chain_json TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS role (
    tenant_id BLOB NOT NULL,
    name TEXT NOT NULL,
    inherits_from TEXT,
    capabilities TEXT NOT NULL,
    worker_id TEXT UNIQUE,
    charter TEXT,
    kpis_json TEXT,
    escalation TEXT,
    PRIMARY KEY (tenant_id, name)
);

CREATE TABLE IF NOT EXISTS worker (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    vertical_id TEXT NOT NULL,
    persona_id TEXT NOT NULL,
    role_id TEXT NOT NULL,
    memory_scope TEXT NOT NULL,
    auto_approval_json TEXT NOT NULL,
    channels_json TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS duty (
    id TEXT PRIMARY KEY,
    worker_id TEXT NOT NULL,
    role_id TEXT NOT NULL,
    tenant_id TEXT NOT NULL,
    cron TEXT NOT NULL,
    entry_action TEXT NOT NULL,
    description TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_worker_tenant ON worker(tenant_id, created_at);
CREATE INDEX IF NOT EXISTS idx_duty_worker ON duty(worker_id);
