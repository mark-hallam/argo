-- mission-audit-principal-tenant-from-token — gateway bearer-token
-- bindings.
--
-- Each row anchors one bearer token to a `(principal_id, tenant_id)`
-- pair so the gateway can derive principal + tenant from the
-- authenticated credential instead of trusting caller-supplied
-- `X-Argo-Principal` / `X-Argo-Tenant-Id` headers. Tokens are stored
-- as SHA-256 hex hashes; the raw token never lands in the database.
-- Opt-in: bearers without a row get a `service:<fingerprint>`
-- synthetic principal and the workspace-default tenant.
--
-- Slot 124: reference DDL — applied at runtime by
-- `argo-store::service_token_binding::ensure_schema` (same lazy-
-- bootstrap pattern as `magic_link_tokens` / `outbound_message`).
-- The versioned migration runner caps at v74; this slot is reached
-- the first time an opt-in CLI / gateway call touches the table.

CREATE TABLE IF NOT EXISTS service_token_binding (
    token_hash   TEXT NOT NULL PRIMARY KEY,
    principal_id TEXT NOT NULL,
    tenant_id    TEXT NOT NULL,
    label        TEXT,
    created_at   TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_service_token_binding_tenant
    ON service_token_binding(tenant_id);
