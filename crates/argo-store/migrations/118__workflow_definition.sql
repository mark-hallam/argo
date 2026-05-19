-- Stream `mission-pm-workflow-persistence-schema` — workflow
-- definition persistence (P1 #5). Today workflow definitions live as
-- TOML files in `verticals/<id>/workflows/*.workflow.toml`, loaded
-- once at session bootstrap. This adds the DB-side substrate that
-- downstream edit/rollback/diff briefs (P1 #6, P1 #7, P2 #9, P2 #10)
-- write against. Files remain authoritative for v1 — this layer is
-- additive until the edit API lands.
--
-- `workflow_definition` is one row per workflow VERSION:
--   * `id`              — uuid string, version-stable; downstream rows
--                         link parents through `parent_id` to form a
--                         per-(tenant, vertical, name) chain.
--   * `definition_toml` — full TOML body, parsed via
--                         `argo_vertical::workflow::parse_workflow` at
--                         the CRUD boundary. Stored as TEXT so the
--                         shape stays human-readable / copy-paste
--                         portable; downstream diff view (P2 #9) does
--                         a textual diff, not a per-step join.
--   * `version`         — monotonic INTEGER, computed by the CRUD
--                         layer as `MAX(version) + 1` per
--                         `(tenant_id, vertical_id, name)`.
--   * `lifecycle_state` — `draft` | `active` | `archived`. Validated
--                         at the CRUD layer; the column is kept
--                         CHECK-free so this migration stays additive
--                         (DROP CHECK requires a table rebuild on
--                         SQLite).
--   * `modified_by`     — Principal-serialised string. Brief leaves
--                         this opaque so we don't pre-commit to a
--                         Principal canonical form before P1 #6 needs
--                         it.
--
-- `workflow_definition_index` is the per-(tenant, vertical, name)
-- "currently active" pointer. Separate table (not a column on
-- `workflow_definition`) so the head can be cleared / swapped without
-- rewriting any version row.
--
-- Slot rationale: latest raw slot on origin/main is 117
-- (`approval_draft_content`); 118 is the next free slot. Versioned
-- migration counter is v72 — next free after v71. Filename and
-- version counters advance independently per the v55 convention.

CREATE TABLE IF NOT EXISTS workflow_definition (
    id              TEXT    PRIMARY KEY,
    tenant_id       TEXT    NOT NULL,
    vertical_id     TEXT    NOT NULL,
    name            TEXT    NOT NULL,
    definition_toml TEXT    NOT NULL,
    version         INTEGER NOT NULL,
    parent_id       TEXT,
    lifecycle_state TEXT    NOT NULL DEFAULT 'draft',
    created_at      TEXT    NOT NULL,
    modified_by     TEXT    NOT NULL,
    UNIQUE (tenant_id, vertical_id, name, version)
);

CREATE INDEX IF NOT EXISTS idx_workflow_definition_lookup
    ON workflow_definition(tenant_id, vertical_id, name);

CREATE INDEX IF NOT EXISTS idx_workflow_definition_parent
    ON workflow_definition(parent_id);

CREATE TABLE IF NOT EXISTS workflow_definition_index (
    tenant_id   TEXT NOT NULL,
    vertical_id TEXT NOT NULL,
    name        TEXT NOT NULL,
    head_id     TEXT NOT NULL,
    PRIMARY KEY (tenant_id, vertical_id, name)
);
