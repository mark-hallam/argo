-- Stream `claude-handoff-routing-rules` (V1.3) — declarative
-- handoff routing rules.
--
-- A HandoffRoutingRule lets operators express "when handoff X
-- happens, route to Y". Rules are tenant-scoped, persisted as
-- JSON blobs (condition + target_selector), and consulted by
-- HandoffWalker before falling through to the static escalation
-- chain.
--
-- Single table:
--   * `handoff_routing_rule` — one row per rule. `name` is unique
--     per tenant. `priority` decides match order (lower fires
--     first). `active` is a soft-disable flag.

CREATE TABLE IF NOT EXISTS handoff_routing_rule (
    id              TEXT PRIMARY KEY,
    tenant_id       TEXT NOT NULL,
    name            TEXT NOT NULL,
    condition_json  TEXT NOT NULL,
    target_json     TEXT NOT NULL,
    priority        INTEGER NOT NULL,
    active          INTEGER NOT NULL,
    created_at      INTEGER NOT NULL,
    UNIQUE(tenant_id, name)
);

CREATE INDEX IF NOT EXISTS idx_handoff_routing_tenant
    ON handoff_routing_rule(tenant_id);
