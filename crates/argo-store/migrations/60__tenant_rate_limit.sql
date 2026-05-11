-- Stream `mission-per-tenant-rate-limiting` (MISSION_BACKLOG Section 1
-- #21) — per-tenant override of the gateway's per-tenant rate limit.
--
-- The gateway already token-buckets every tenant at a config-driven
-- default (`[gateway.rate_limit.tenant].requests_per_minute`). An
-- operator can replace that default per tenant by writing one row
-- here. Missing row → fall back to the config default.
--
-- Schema-version slot: this is migration v56 (the next free Rust slot
-- after v55 = `skill_flag`). Raw filename slot 60 follows the
-- standalone counter (59__skill_flag was the prior tail) and does
-- not have to match the version number — the convention documented
-- in `migrations/v55.rs`.

CREATE TABLE IF NOT EXISTS tenant_rate_limit (
    tenant_id            BLOB    PRIMARY KEY,
    requests_per_minute  INTEGER NOT NULL,
    updated_at           INTEGER NOT NULL
);
