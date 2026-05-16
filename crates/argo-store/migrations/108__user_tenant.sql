-- mission-tenant-substrate-tables — user ↔ tenant membership join table.
-- Composite primary key on (user_id, tenant_id) so a user can hold one
-- role per tenant. Role enum: 'owner' | 'admin' | 'user' | 'guest'.

CREATE TABLE IF NOT EXISTS user_tenant (
    user_id     BLOB    NOT NULL,
    tenant_id   BLOB    NOT NULL,
    role        TEXT    NOT NULL,
    created_at  INTEGER NOT NULL,
    PRIMARY KEY (user_id, tenant_id),
    FOREIGN KEY (user_id)   REFERENCES user_account(id),
    FOREIGN KEY (tenant_id) REFERENCES tenant(id)
);
CREATE INDEX IF NOT EXISTS idx_user_tenant_tenant ON user_tenant(tenant_id);
CREATE INDEX IF NOT EXISTS idx_user_tenant_user   ON user_tenant(user_id);
