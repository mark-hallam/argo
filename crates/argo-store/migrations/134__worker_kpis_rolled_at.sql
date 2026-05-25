-- Stream `mission-kpi-rollup-from-audit-trail` (mission claim 2): adds
-- the per-worker `kpis_rolled_at` watermark used by the KPI rollup
-- scheduler in `argo-orchestrator::kpi_rollup` to skip workers whose
-- audit activity has not advanced since the last tick.
--
-- The brief originally reserved Rust slot v81 + raw slot 134, but slot
-- v81 landed first as `routing_decision_tenant_id` (#2264). Rust
-- counter advances to v82 here; the raw filename slot 134 is
-- independently assigned per the v55-onward convention (see
-- `scripts/check-migration-slots.sh`).
--
-- Additive + nullable: historical workers stay NULL so the first
-- rollup tick processes every worker with audit rows. Down-migration
-- drops the column (lives in `migrations/v82.rs`).

ALTER TABLE worker ADD COLUMN kpis_rolled_at INTEGER;

CREATE INDEX IF NOT EXISTS idx_worker_kpis_rolled_at
    ON worker(tenant_id, kpis_rolled_at);
