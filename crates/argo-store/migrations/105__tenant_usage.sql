-- claude-pricing-plans-config — per-tenant action counter scoped to a
-- billing window. Keyed by tenant id (text) to match the existing
-- tenant_quota row encoding so the pricing-tier reads can JOIN cleanly.
-- period_start is the Unix epoch (seconds) of the current
-- calendar-month boundary, reset whenever the gate observes a new period.

CREATE TABLE IF NOT EXISTS tenant_usage (
    tenant_id     TEXT    PRIMARY KEY,
    period_start  INTEGER NOT NULL,
    used_actions  INTEGER NOT NULL DEFAULT 0,
    updated_at    INTEGER NOT NULL
);
