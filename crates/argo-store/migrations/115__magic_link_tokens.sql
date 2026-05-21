-- claude-magic-link-auth-backend — single-use magic-link tokens.
--
-- `token_hash` is the SHA-256 hex of the 32-byte random token emailed
-- to the user; the raw token is never persisted. `expires_at` is 15
-- minutes from issue; `used_at` flips on the first successful consume
-- so the second consume of the same hash returns AlreadyUsed.
-- `email` repeats the target so the verify path resolves a destination
-- before the user_account row is created.
--
-- SAFETY: global table — magic-link tokens are issued and consumed
-- pre-tenant. The token resolves to an email, which then resolves to a
-- user_account; tenant scope is established only after sign-in via
-- `user_tenant` membership. Per-tenant token partitioning would break
-- multi-tenant users.
--
-- Slot 115: reference DDL — applied at runtime by
-- `argo-store::user_account::ensure_schema` (same pattern as
-- worker_presence / siem_dispatch_cursor / tenant substrate). The
-- versioned migration runner caps at v69 so callers that haven't run
-- a slot-based applier still bootstrap on first use.

CREATE TABLE IF NOT EXISTS magic_link_tokens (
    token_hash      TEXT    PRIMARY KEY,
    email           TEXT    NOT NULL,
    expires_at      INTEGER NOT NULL,
    used_at         INTEGER
);

CREATE INDEX IF NOT EXISTS idx_magic_link_tokens_expires_at
    ON magic_link_tokens(expires_at);
