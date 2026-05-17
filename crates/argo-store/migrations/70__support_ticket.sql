-- Stream `claude-support-inbox-spa-page` (Wave 4 P2 follow-up to #1762).
--
-- Inbound support tickets persisted server-side so the operator inbox
-- can triage independently of the outbound webhook (Plain / Linear).
-- The Next.js POST /api/support/ticket route forwards the webhook out
-- as before; this storage path is the in-addition-to side that survives
-- webhook delivery failure (graceful degradation).
--
-- `tenant_id` is nullable: tickets created from unauthenticated
-- marketing-site contact forms have no tenant context until an
-- operator manually assigns one. `assigned_to` exists for the future
-- triage workflow brief; v1 UI doesn't surface it.
--
-- Status / priority / author_kind are string discriminators per the
-- brief's hidden constraints — operability wins over the byte budget.

CREATE TABLE IF NOT EXISTS support_ticket (
    id              BLOB PRIMARY KEY,
    tenant_id       BLOB,
    user_email      TEXT NOT NULL,
    subject         TEXT NOT NULL,
    body            TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'open',
    priority        TEXT NOT NULL DEFAULT 'normal',
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL,
    assigned_to     BLOB
);

CREATE INDEX IF NOT EXISTS idx_support_ticket_status
    ON support_ticket(status);

CREATE INDEX IF NOT EXISTS idx_support_ticket_tenant
    ON support_ticket(tenant_id);

CREATE TABLE IF NOT EXISTS support_ticket_reply (
    id              BLOB PRIMARY KEY,
    ticket_id       BLOB NOT NULL,
    author_kind     TEXT NOT NULL,
    author_id       BLOB,
    body            TEXT NOT NULL,
    created_at      INTEGER NOT NULL,
    FOREIGN KEY (ticket_id) REFERENCES support_ticket(id)
);

CREATE INDEX IF NOT EXISTS idx_support_ticket_reply_ticket
    ON support_ticket_reply(ticket_id);
