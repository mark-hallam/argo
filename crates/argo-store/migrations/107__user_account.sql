-- mission-tenant-substrate-tables — magic-link user account table.
-- Email is the verified identity column; display_name and last_login_at
-- are nullable until first verify. Status enum: 'active' | 'disabled'.
--
-- SAFETY: global table — user identity registry. A user may belong to
-- multiple tenants via `user_tenant`, so the account itself is not
-- tenant-scoped. Per-tenant authorization lives in `user_tenant`.

CREATE TABLE IF NOT EXISTS user_account (
    id              BLOB    PRIMARY KEY,
    email           TEXT    NOT NULL UNIQUE,
    display_name    TEXT,
    created_at      INTEGER NOT NULL,
    last_login_at   INTEGER,
    status          TEXT    NOT NULL DEFAULT 'active'
);
CREATE INDEX IF NOT EXISTS idx_user_account_status ON user_account(status);
