-- web-gateway-endpoints-onboarding-batch — onboarding bookkeeping.
--
-- Three additive tables landed under one slot to keep the brief's
-- migration footprint minimal. None of them carry plaintext secrets:
-- the provider-key blob is encrypted by the gateway handler via
-- `argo-credentials::age_wrap` before it reaches this table.
--
-- Slot 126: reference DDL — applied at runtime by
-- `argo-store::workspace_keys::ensure_schema` (lazy-bootstrap pattern
-- mirrors `magic_link_tokens` / `outbound_message`). The versioned
-- registry caps at v76 so callers that haven't run the slot applier
-- still bootstrap on first use.

CREATE TABLE IF NOT EXISTS workspace_provider_key (
    tenant_id        BLOB NOT NULL,
    workspace_id     BLOB NOT NULL,
    provider         TEXT NOT NULL,
    masked_key       TEXT NOT NULL,
    ciphertext_b64   TEXT NOT NULL,
    test_cost_micros INTEGER NOT NULL DEFAULT 0,
    test_tokens      INTEGER NOT NULL DEFAULT 0,
    created_at       INTEGER NOT NULL,
    updated_at       INTEGER NOT NULL,
    PRIMARY KEY (tenant_id, workspace_id, provider)
);

CREATE INDEX IF NOT EXISTS idx_workspace_provider_key_tenant
    ON workspace_provider_key(tenant_id);

CREATE TABLE IF NOT EXISTS workspace_installed_vertical (
    tenant_id       BLOB NOT NULL,
    workspace_id    BLOB NOT NULL,
    vertical_id     TEXT NOT NULL,
    workflows_added INTEGER NOT NULL DEFAULT 0,
    skills_added    INTEGER NOT NULL DEFAULT 0,
    installed_at    INTEGER NOT NULL,
    PRIMARY KEY (tenant_id, workspace_id, vertical_id)
);

CREATE INDEX IF NOT EXISTS idx_workspace_installed_vertical_tenant
    ON workspace_installed_vertical(tenant_id);
