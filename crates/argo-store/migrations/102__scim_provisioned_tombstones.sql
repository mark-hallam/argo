-- Mission `mission-scim-provisioning` — SCIM 2.0 user provisioning.
--
-- Two tables back the SCIM endpoints exposed by argo-cli's gateway:
--
--   * `scim_user` — canonical record of every user provisioned by an
--     IdP via `POST /scim/v2/Users`. The `id` blob is the SCIM
--     resource id surfaced over the wire; `external_id` is the
--     IdP-supplied stable id (Okta `externalId`, Azure AD `id`, etc.).
--     `active` mirrors the SCIM `active` boolean; deactivation
--     (`active = 0`) does NOT remove the row — audit references stay
--     intact. `deleted_at IS NOT NULL` indicates the row was
--     soft-deleted via `DELETE /scim/v2/Users/<id>`.
--
--   * `scim_provisioned_tombstones` — fast lookup index of every
--     soft-deleted SCIM user. One row per DELETE, written in the same
--     transaction that sets `scim_user.deleted_at`. Preserves the
--     audit-chain reference: an audit row authored by a deleted user
--     can still resolve their tenant + user_name without scanning the
--     full `scim_user` table for tombstoned rows.
--
-- Schema-version slot: Rust counter is v64 (next free after v63 =
-- `roster.policy_override_json`). Raw filename slot 75 avoids slot
-- 74, which is already owned by the roster policy override migration.
-- Filename and version counters advance independently per the v55
-- convention.
--
-- Idempotent: CREATE TABLE/INDEX IF NOT EXISTS. Down migration drops
-- the indexes and tables (see `migrations/v64.rs`).

CREATE TABLE IF NOT EXISTS scim_user (
    id            BLOB    PRIMARY KEY,
    tenant_id     BLOB    NOT NULL,
    external_id   TEXT,
    user_name     TEXT    NOT NULL,
    display_name  TEXT,
    emails_json   TEXT    NOT NULL DEFAULT '[]',
    active        INTEGER NOT NULL DEFAULT 1,
    created_at    INTEGER NOT NULL,
    updated_at    INTEGER NOT NULL,
    deleted_at    INTEGER
);

-- Hot path: SCIM `?filter=userName eq "..."` lookups and the
-- POST-side uniqueness check both query by (tenant, user_name) over
-- live rows. The partial-index expression is not portable, so the
-- index covers every row and the handler filters `deleted_at IS NULL`
-- in-query.
CREATE INDEX IF NOT EXISTS idx_scim_user_tenant_username
    ON scim_user(tenant_id, user_name);

CREATE INDEX IF NOT EXISTS idx_scim_user_external
    ON scim_user(tenant_id, external_id);

CREATE TABLE IF NOT EXISTS scim_provisioned_tombstones (
    id          BLOB    PRIMARY KEY,
    tenant_id   BLOB    NOT NULL,
    user_name   TEXT    NOT NULL,
    deleted_at  INTEGER NOT NULL,
    reason      TEXT    NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_scim_tombstones_tenant
    ON scim_provisioned_tombstones(tenant_id);
