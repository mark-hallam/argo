-- Stream `codex-budget-alert-channel` — push notifications when
-- worker spend crosses configured budget thresholds.

CREATE TABLE IF NOT EXISTS worker_budget_alert (
    id TEXT PRIMARY KEY,
    worker_id TEXT NOT NULL,
    threshold INTEGER NOT NULL,
    fired_on TEXT NOT NULL,
    fired_at INTEGER NOT NULL,
    spend_amount INTEGER NOT NULL,
    headroom_amount INTEGER NOT NULL,
    notified_via TEXT NOT NULL,
    synthetic INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_worker_budget_alert_recent
    ON worker_budget_alert(fired_at DESC);

CREATE INDEX IF NOT EXISTS idx_worker_budget_alert_worker
    ON worker_budget_alert(worker_id, fired_at DESC);

CREATE UNIQUE INDEX IF NOT EXISTS idx_worker_budget_alert_dedup
    ON worker_budget_alert(worker_id, threshold, fired_on)
    WHERE synthetic = 0;
