-- Stream `claude-org-awareness` (V1.3) — org chart + addressable
-- names + manager-chain primitives.
--
-- Four tables form the org graph:
--   * `org_unit` — a tree of organisational units (a company has
--     departments, departments have teams, etc.). Self-referential
--     via `parent_id`.
--   * `org_member` — one row per teammate. `kind` distinguishes
--     human teammates from worker (AI) teammates so the audit
--     ledger can treat them differently. `manager_id` is a soft
--     reference to another `org_member.id` for the reports-to
--     chain.
--   * `org_member_alias` — many-to-one mapping of addressable
--     names (e.g. "Maria", "M.H.", "@maria") onto a canonical
--     member. Used by `argo org member <alias>` to resolve.
--   * `org_reports_to` — explicit reports-to edges, denormalised
--     out of `manager_id` for graph queries that walk the chain
--     in either direction. Insert + maintain in lockstep with
--     `org_member.manager_id`.
--
-- Multi-tenant scoping: every row carries `tenant_id` so two
-- tenants can have totally separate org charts in the same DB.

CREATE TABLE IF NOT EXISTS org_unit (
    id          TEXT PRIMARY KEY,
    tenant_id   TEXT NOT NULL,
    name        TEXT NOT NULL,
    parent_id   TEXT,
    created_at  INTEGER NOT NULL,
    UNIQUE(tenant_id, name)
);

CREATE INDEX IF NOT EXISTS idx_org_unit_parent ON org_unit(parent_id);
CREATE INDEX IF NOT EXISTS idx_org_unit_tenant ON org_unit(tenant_id);

CREATE TABLE IF NOT EXISTS org_member (
    id          TEXT PRIMARY KEY,
    tenant_id   TEXT NOT NULL,
    kind        TEXT NOT NULL,
    canonical   TEXT NOT NULL,
    unit_id     TEXT,
    manager_id  TEXT,
    worker_id   TEXT,
    created_at  INTEGER NOT NULL,
    UNIQUE(tenant_id, canonical)
);

CREATE INDEX IF NOT EXISTS idx_org_member_unit ON org_member(unit_id);
CREATE INDEX IF NOT EXISTS idx_org_member_manager ON org_member(manager_id);
CREATE INDEX IF NOT EXISTS idx_org_member_worker ON org_member(worker_id);

CREATE TABLE IF NOT EXISTS org_member_alias (
    member_id   TEXT NOT NULL,
    alias       TEXT NOT NULL,
    PRIMARY KEY (member_id, alias)
);

CREATE INDEX IF NOT EXISTS idx_org_member_alias_alias ON org_member_alias(alias);

CREATE TABLE IF NOT EXISTS org_reports_to (
    member_id   TEXT NOT NULL,
    manager_id  TEXT NOT NULL,
    depth       INTEGER NOT NULL,
    PRIMARY KEY (member_id, manager_id)
);
