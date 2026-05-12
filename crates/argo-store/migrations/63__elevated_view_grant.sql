-- Stream J16 — `elevated-view-request` endpoint + grant table.
--
-- Per-principal, per-category, time-bounded "show me the raw value"
-- override consulted by the J2 redact-policy resolver. The brief auto-
-- grants in v1 (request → grant emitted on the same call); the
-- `mission-enterprise-elevated-view-approval` follow-up inserts an
-- operator-approval step between the request and the row insert.
--
-- Schema-version slot: Rust counter is v59 (next free after v58 =
-- `operator_risk_overrides`); raw filename slot 63 follows the
-- standalone counter (62 = `operator_risk_overrides`). Filename and
-- version counters advance independently per the v55 convention.
--
-- Columns map to the brief's spec; SQLite has no native `timestamptz`
-- or `text[]`, so the v0.x SQLite store encodes them as:
--   • timestamps  → INTEGER (unix seconds, UTC)
--   • text[]      → TEXT (JSON-array string, e.g. '["email","phone"]')
-- The redact-policy resolver and the J16 handler both work in
-- `chrono::DateTime<Utc>` + `Vec<String>`, decoding at the boundary.
--
-- Idempotent: CREATE TABLE/INDEX IF NOT EXISTS. Down migration drops
-- the indexes and the table (see `migrations/v59.rs`).

CREATE TABLE IF NOT EXISTS elevated_view_grant (
    id              BLOB PRIMARY KEY,
    tenant_id       BLOB NOT NULL,
    principal_id    TEXT NOT NULL,
    categories_json TEXT NOT NULL,
    reason          TEXT NOT NULL,
    granted_at      INTEGER NOT NULL,
    expires_at      INTEGER NOT NULL,
    revoked_at      INTEGER
);

-- Hot path: J2's resolver looks up active grants by principal. The
-- partial-index shape (`principal_id, revoked_at`) lets the redact
-- query short-circuit on revoked rows without scanning the full
-- table per request.
CREATE INDEX IF NOT EXISTS idx_elevated_view_grant_principal
    ON elevated_view_grant(principal_id, revoked_at);

CREATE INDEX IF NOT EXISTS idx_elevated_view_grant_tenant
    ON elevated_view_grant(tenant_id);
