-- Additive SLA tracking schema for M5-orch-sla-tracking.
CREATE TABLE IF NOT EXISTS sla_definition (
    id TEXT PRIMARY KEY,
    tenant_id BLOB NOT NULL,
    target_kind TEXT NOT NULL,
    target_id TEXT NOT NULL,
    target_latency_ms INTEGER NOT NULL,
    escalation_policy TEXT NOT NULL,
    breach_action TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_sla_definition_tenant_target
    ON sla_definition(tenant_id, target_kind, target_id, enabled);

CREATE TABLE IF NOT EXISTS sla_breach_event (
    id TEXT PRIMARY KEY,
    tenant_id BLOB NOT NULL,
    sla_id TEXT NOT NULL,
    target_kind TEXT NOT NULL,
    target_id TEXT NOT NULL,
    observed_latency_ms INTEGER NOT NULL,
    threshold_ms INTEGER NOT NULL,
    action TEXT NOT NULL,
    emitted_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_sla_breach_event_tenant_emitted
    ON sla_breach_event(tenant_id, emitted_at DESC);
