-- Stream `claude-worker-roster` (V1.3) — roster primitive for
-- group-level policy + channels + budget cap.
--
-- A Roster is a named bundle of workers that share an
-- AutoApprovalPolicy, NotifyChannel preferences, and an optional
-- spend cap. Per-worker policy resolution (Worker::resolved_policy)
-- consults rosters when the worker's own auto_approval is
-- disabled.
--
-- Two tables:
--   * `roster` — one row per group. `policy_json`, `channels_json`,
--     `budget_cap_usd_micros` carry the shared configuration.
--   * `roster_member` — many-to-many between worker_id and
--     roster_id. A worker may belong to multiple rosters. Conflict
--     resolution is most-restrictive-wins inside the identity
--     crate (smallest spend cap / smallest budget / strictest
--     channels).
--
-- Multi-tenant: every row carries `tenant_id`. Two tenants can have
-- totally separate rosters in the same DB without leakage.

CREATE TABLE IF NOT EXISTS roster (
    id                      TEXT PRIMARY KEY,
    tenant_id               TEXT NOT NULL,
    name                    TEXT NOT NULL,
    policy_json             TEXT NOT NULL,
    channels_json           TEXT NOT NULL,
    budget_cap_usd_micros   INTEGER,
    created_at              INTEGER NOT NULL,
    UNIQUE(tenant_id, name)
);

CREATE INDEX IF NOT EXISTS idx_roster_tenant ON roster(tenant_id);

CREATE TABLE IF NOT EXISTS roster_member (
    roster_id   TEXT NOT NULL,
    worker_id   TEXT NOT NULL,
    added_at    INTEGER NOT NULL,
    PRIMARY KEY (roster_id, worker_id)
);

CREATE INDEX IF NOT EXISTS idx_roster_member_worker ON roster_member(worker_id);
