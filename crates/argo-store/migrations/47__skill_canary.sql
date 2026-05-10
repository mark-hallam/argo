-- Stream `claude-skill-canary-deployment` — canary rollout for
-- promoted skill versions.
--
-- One row per skill_id (the skill family name) records an
-- in-flight canary: the operator has promoted `canary_version` and
-- wants `traffic_percent` of invocations routed to it instead of
-- `baseline_version`. `expires_at` (NULL = no expiry) lets ops set
-- a TTL after which we always fall through to baseline.
--
-- Single-row per skill_id by design: starting a new canary
-- replaces the prior config (operator drives the rollout, we don't
-- chain canaries on top of canaries).
--
-- Slot 47 — the next free raw-SQL filename slot after the audit-principal
-- migration at 46. Wrapper lives at `crates/argo-store/src/migrations/v43.rs`
-- (filename slot and `schema_version` are independent counters per the v42
-- wrapper's docstring).

CREATE TABLE IF NOT EXISTS skill_canary (
    skill_id          TEXT PRIMARY KEY,
    baseline_version  TEXT NOT NULL,
    canary_version    TEXT NOT NULL,
    traffic_percent   INTEGER NOT NULL CHECK (traffic_percent BETWEEN 0 AND 100),
    started_at        INTEGER NOT NULL,
    expires_at        INTEGER
);

CREATE INDEX IF NOT EXISTS idx_skill_canary_expires
    ON skill_canary(expires_at);
