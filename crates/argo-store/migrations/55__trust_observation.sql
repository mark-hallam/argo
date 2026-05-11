-- Stream `mission-cross-tenant-trust-graph` — cross-tenant trust
-- graph primitive (MISSION_BACKLOG Section 3 Claim 4 #81).
--
-- Immutable, append-only observation table. One row per (source
-- tenant, consumer tenant, skill) quality observation. The
-- in-memory `argo_skill::trust_graph::TrustGraph` materializes
-- pairwise weights from a query window (default last 90 days)
-- using `mean(quality_score)` clamped to 0.0..=1.0 with linear
-- time-decay.
--
-- Tenant ids are stored as 16-byte BLOBs to match the existing
-- UUIDv7 plumbing on `workspace.tenant_id`, `approval_request.
-- tenant_id`, etc. — `TenantId(Uuid)` round-trips through
-- `.as_bytes().as_slice()` and `Uuid::from_slice`.
--
-- Slot picked: 54 — the next free raw filename slot after
-- 52__persona_revision.sql and 53__forge_run_cost.sql (both
-- MERGED). Versioned reconciliation lives in v50.rs (the next
-- free `schema_version` slot after v48/v49).
--
-- Idempotent: CREATE TABLE/INDEX IF NOT EXISTS. Down migration
-- drops the indexes and table (see `migrations/v50.rs`).
CREATE TABLE IF NOT EXISTS trust_observation (
    id BLOB PRIMARY KEY,
    source_tenant_id BLOB NOT NULL,
    consumer_tenant_id BLOB NOT NULL,
    skill_id BLOB NOT NULL,
    quality_score REAL NOT NULL,
    observed_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS trust_observation_consumer_idx
    ON trust_observation(consumer_tenant_id, observed_at);

CREATE INDEX IF NOT EXISTS trust_observation_source_idx
    ON trust_observation(source_tenant_id, observed_at);

CREATE INDEX IF NOT EXISTS trust_observation_observed_at_idx
    ON trust_observation(observed_at);
