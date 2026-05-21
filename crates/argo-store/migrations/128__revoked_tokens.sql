-- mission-revocation-with-verify-middleware-v1x — session-JWT revocation list.
--
-- One row per logged-out cookie JWT (`__Host-session`). The
-- `middleware_session_verify` cookie-auth gate consults this table on
-- every authenticated request and rejects 401 when the cookie's `jti`
-- (uuid v7) is present here. The companion `POST /api/v1/auth/logout`
-- handler inserts a row, clears the cookie, and emits an
-- `AuditKind::LogoutCalled` audit row.
--
-- Slot 128: reference DDL — applied at runtime by
-- `argo-store::revoked_tokens::ensure_schema` (same lazy-bootstrap
-- pattern as slots 115/116). Versioned migration runner caps at v75
-- so callers that haven't run a slot-based applier still bootstrap
-- on first use.

CREATE TABLE IF NOT EXISTS revoked_tokens (
    jti         TEXT    PRIMARY KEY,
    tenant_id   TEXT    NOT NULL,
    revoked_at  INTEGER NOT NULL,
    principal   TEXT,
    reason      TEXT
);

CREATE INDEX IF NOT EXISTS idx_revoked_tokens_tenant
    ON revoked_tokens(tenant_id);

CREATE INDEX IF NOT EXISTS idx_revoked_tokens_revoked_at
    ON revoked_tokens(revoked_at);
