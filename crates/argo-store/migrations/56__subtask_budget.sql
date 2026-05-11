-- Stream `mission-delegation-budget-tree` (MISSION_BACKLOG Section 3
-- Claim 2 #57 — per-delegation budget tree).
--
-- Today a delegating worker's budget is opaque to its children: each
-- child runs its own budget from scratch and total spend over a deep
-- decomposition tree is unbounded. This migration adds the three
-- columns that make a per-subtask budget tree possible:
--
--   * `budget_usd_cents`  — declared cap; NULL means "unbounded"
--   * `spent_usd_cents`   — accumulator the runtime adds to each turn
--   * `parent_subtask_id` — explicit parent pointer so a CTE roll-up
--                           can sum spend across descendants
--
-- Legacy rows (pre-migration) get `spent_usd_cents = 0` (NOT NULL
-- DEFAULT) and NULL `budget_usd_cents` / `parent_subtask_id`, so
-- existing planners and ledger callers keep working unchanged: NULL
-- budget = unbounded = no enforcement.
--
-- Versioned counterpart: `crates/argo-store/src/migrations/v50.rs`.
-- Idempotency lives in the versioned migration (PRAGMA column probes
-- before each ALTER); this raw SQL file is reference-only.

ALTER TABLE subtask_ledger ADD COLUMN budget_usd_cents INTEGER NULL;

ALTER TABLE subtask_ledger ADD COLUMN spent_usd_cents INTEGER NOT NULL DEFAULT 0;

ALTER TABLE subtask_ledger ADD COLUMN parent_subtask_id TEXT NULL;

CREATE INDEX IF NOT EXISTS idx_subtask_ledger_parent_subtask
    ON subtask_ledger(parent_subtask_id)
    WHERE parent_subtask_id IS NOT NULL;
