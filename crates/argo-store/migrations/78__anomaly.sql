-- Stream `claude-anomaly-detector`: per-(worker, action_kind)
-- behavioural baselines + anomaly events.

CREATE TABLE IF NOT EXISTS worker_behavior_baseline (
    worker_id TEXT NOT NULL,
    action_kind TEXT NOT NULL,
    latency_mean REAL NOT NULL DEFAULT 0,
    latency_m2 REAL NOT NULL DEFAULT 0,
    retry_mean REAL NOT NULL DEFAULT 0,
    retry_m2 REAL NOT NULL DEFAULT 0,
    sample_count INTEGER NOT NULL DEFAULT 0,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (worker_id, action_kind)
);

CREATE TABLE IF NOT EXISTS worker_anomaly_event (
    id BLOB PRIMARY KEY,
    worker_id TEXT NOT NULL,
    action_id TEXT NOT NULL,
    action_kind TEXT NOT NULL,
    dimension TEXT NOT NULL,
    observed REAL NOT NULL,
    baseline_mean REAL NOT NULL,
    baseline_stddev REAL NOT NULL,
    z_score REAL NOT NULL,
    threshold REAL NOT NULL,
    sample_count INTEGER NOT NULL,
    observed_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_anomaly_worker_at
    ON worker_anomaly_event(worker_id, observed_at DESC);

CREATE INDEX IF NOT EXISTS idx_anomaly_action
    ON worker_anomaly_event(action_id);
