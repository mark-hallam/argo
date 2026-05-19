-- Stream `mission-pm-kill-switch-1-store-audit` — worker freeze table.
--
-- This is the persistence substrate for the PM kill-switch slices.
-- Runner, CLI, gateway, and web surfaces land in sibling briefs; this
-- slice only records whether a worker is currently frozen and who
-- toggled that state.
--
-- Slot rationale: raw SQL slots 117 (`approval_draft_content`) and
-- 118 (`workflow_definition`) already exist on current main. Slot 119
-- is the next free filename slot. The versioned migration counter is
-- v73; filename and version counters advance independently per the
-- v55 convention.

CREATE TABLE IF NOT EXISTS worker_freeze (
    tenant_id    TEXT      NOT NULL,
    worker_id    TEXT      NOT NULL,
    frozen_at    TIMESTAMP NOT NULL,
    frozen_by    TEXT      NOT NULL,
    reason       TEXT      NOT NULL,
    unfrozen_at  TIMESTAMP NULL,
    unfrozen_by  TEXT      NULL,
    PRIMARY KEY (tenant_id, worker_id)
);

CREATE INDEX IF NOT EXISTS idx_worker_freeze_active
    ON worker_freeze(tenant_id, unfrozen_at);
