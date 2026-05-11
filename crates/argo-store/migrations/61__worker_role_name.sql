-- Stream `mission-routing-skillmatch-rolematch` (Tier M4).
-- Additive `worker.role_name` column (TEXT NULL) for
-- `TargetSelector::RoleMatch(String)` resolution. Operators populate
-- the column sparsely via `argo worker onboard --role-name <s>`; the
-- routing engine ranks workers whose `role_name = <selector>` by
-- `trust_grade DESC, last_used_at DESC, worker_id ASC`. Empty result
-- is normal in v1 because most existing workers won't have it set.
--
-- Slot rationale: SQL filename slot 60 (`tenant_rate_limit`, brief
-- `mission-per-tenant-rate-limiting`) was the prior tail when this
-- brief landed, so 61 is the next free slot. Versioned reconciliation
-- lives in `v57.rs`.
ALTER TABLE worker ADD COLUMN role_name TEXT;

CREATE INDEX IF NOT EXISTS idx_worker_role_name
    ON worker(role_name);
