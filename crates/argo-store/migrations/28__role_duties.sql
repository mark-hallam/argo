-- Stream `claude-worker-duty-scheduler` (V1.3 — autonomous loop).
--
-- Persists the firable duties attached to a worker's role and
-- the (duty -> session) attribution chain for fired duties so
-- audit trails can answer "which duty started this session?".
--
-- Two tables:
--   * `role_duty`            — one row per scheduled duty.
--   * `duty_fired_session`   — link from a fired duty to the
--                              session it spawned.
--
-- Note: the in-memory `argo_identity::Duty` struct still carries
-- its (cron, entry_action, description, ...) fields so the
-- existing role JSON shape stays backwards compatible. This table
-- is the canonical source of truth for the scheduler.

CREATE TABLE IF NOT EXISTS role_duty (
    -- UUID-stringified duty id (sortable insert order).
    id TEXT PRIMARY KEY,
    -- Tenant scope (16-byte uuid blob, matches every other table).
    tenant_id BLOB NOT NULL,
    -- Owning worker.
    worker_id TEXT NOT NULL,
    -- Cron expression (5 or 6-field form accepted by the `cron` crate).
    cron TEXT NOT NULL,
    -- Handler name in the worker action vocabulary.
    entry_action TEXT NOT NULL,
    -- Human-readable description (operator-visible).
    description TEXT NOT NULL DEFAULT '',
    -- 1 = enabled, 0 = disabled. Disabled duties never fire.
    enabled INTEGER NOT NULL DEFAULT 1,
    -- Paused-until micros since epoch (NULL = not paused).
    paused_until INTEGER,
    -- Last-fired-at micros since epoch (NULL = never fired).
    last_fired_at INTEGER,
    -- Created-at micros since epoch.
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_role_duty_worker
    ON role_duty(tenant_id, worker_id, enabled);

CREATE INDEX IF NOT EXISTS idx_role_duty_active
    ON role_duty(enabled, paused_until);

CREATE TABLE IF NOT EXISTS duty_fired_session (
    -- Fired duty id.
    duty_id TEXT NOT NULL,
    -- Session spawned by the fire.
    session_id TEXT NOT NULL,
    -- Fired-at micros since epoch.
    fired_at INTEGER NOT NULL,
    -- Decision rendered by the AutoApprovalPolicy gate. One of
    -- `auto_approved`, `denied_gate`, `denied_paused`,
    -- `denied_disabled`, `manual_fire`. Lets the audit walker
    -- explain why some fires don't have a session row.
    decision TEXT NOT NULL DEFAULT 'auto_approved',
    PRIMARY KEY (duty_id, session_id)
);

CREATE INDEX IF NOT EXISTS idx_duty_fired_session_duty
    ON duty_fired_session(duty_id, fired_at DESC);
