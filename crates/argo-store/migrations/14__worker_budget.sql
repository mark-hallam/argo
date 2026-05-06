-- Stream H-1: worker budget ledger.
--
-- One row per recorded charge against a worker's spend budget.
-- The `argo-identity::worker_budget::WorkerBudget` aggregator
-- rolls these into daily / monthly totals at snapshot time and
-- feeds the auto-approval evaluator's `SpendSnapshot`.
--
-- Designed so the dispatch shim (Stream H-2) can persist a row
-- per dispatched action with `INSERT INTO worker_budget …`
-- without touching the rest of the schema.

CREATE TABLE IF NOT EXISTS worker_budget (
    -- Tenant boundary — every query is scoped here first.
    tenant_id BLOB NOT NULL,
    -- Worker that incurred the spend.
    worker_id TEXT NOT NULL,
    -- Action that was charged. References `worker_action.action_id`
    -- but no FK so the ledger is durable even if the action row
    -- is rotated out by retention policy.
    action_id TEXT NOT NULL,
    -- USD micros (1 USD = 1_000_000 micros). Always positive.
    usd_micros INTEGER NOT NULL,
    -- ISO-8601 UTC timestamp of the charge. Stored as INTEGER
    -- (epoch seconds) so range queries stay sargable.
    charged_at INTEGER NOT NULL,
    PRIMARY KEY (tenant_id, worker_id, action_id, charged_at)
);

-- Daily rollup index: every snapshot query walks this.
CREATE INDEX IF NOT EXISTS idx_worker_budget_day
ON worker_budget(tenant_id, worker_id, charged_at);
