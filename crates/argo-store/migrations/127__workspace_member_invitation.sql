-- web-settings-members-invite-real-wire — one row per outstanding
-- workspace-member invitation. Accepted rows stay forever as audit
-- trail; expired rows are eligible for periodic sweep but not deleted
-- by the runtime path. The invitation carries its own SHA-256 token
-- hash so the accept flow does not have to share `magic_link_tokens`
-- (login tokens) with invite tokens — distinct lifecycles, distinct
-- expiry windows (15 min vs 7 days).

CREATE TABLE IF NOT EXISTS workspace_member_invitation (
    id                   TEXT      NOT NULL PRIMARY KEY,
    tenant_id            BLOB      NOT NULL,
    email                TEXT      NOT NULL,
    role                 TEXT      NOT NULL
                              CHECK(role IN ('owner','admin','user','guest')),
    cohort_scope_json    TEXT      NOT NULL DEFAULT '[]',
    token_hash           TEXT      NOT NULL UNIQUE,
    invited_by           TEXT      NOT NULL,
    invited_at           INTEGER   NOT NULL,
    expires_at           INTEGER   NOT NULL,
    accepted_at          INTEGER,
    accepted_by_user_id  BLOB
);

CREATE INDEX IF NOT EXISTS idx_workspace_member_invitation_tenant_email
    ON workspace_member_invitation(tenant_id, email);
CREATE INDEX IF NOT EXISTS idx_workspace_member_invitation_expires
    ON workspace_member_invitation(expires_at);
CREATE INDEX IF NOT EXISTS idx_workspace_member_invitation_pending
    ON workspace_member_invitation(tenant_id, email)
    WHERE accepted_at IS NULL;
