-- Stream `claude-spend-anomaly-alerts` — anomaly events fired by
-- the hourly z-score check.
--
-- One row per (worker, hour) bucket whose observed spend exceeded
-- |z| > $ARGO_SPEND_ANOMALY_Z (default 4.0) standard deviations
-- from the baseline-for-hour-of-day. `argo budget anomalies` reads
-- this table; the dispatcher (CLI tick) inserts on detection +
-- fires NotifyChannel alerts in the same path.

CREATE TABLE IF NOT EXISTS worker_spend_anomaly (
    -- UUIDv7 string (sortable insert order).
    id TEXT PRIMARY KEY,
    -- Worker that triggered the anomaly.
    worker_id TEXT NOT NULL,
    -- USD-per-hour observed in the most recent hour bucket.
    observed_usd_per_hour REAL NOT NULL,
    -- Baseline mean at detection time.
    baseline_mean_usd REAL NOT NULL,
    -- Baseline stddev at detection time (`sqrt(M2 / (n-1))`).
    baseline_stddev_usd REAL NOT NULL,
    -- (observed - mean) / stddev. Sign preserved so callers can
    -- distinguish a spike (`+`) from a drought (`-`).
    z_score REAL NOT NULL,
    -- 0..=23 — UTC hour-of-day this anomaly fires for.
    hour_of_day INTEGER NOT NULL,
    -- UTC microseconds at which the anomaly was recorded.
    observed_at_micros INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_spend_anomaly_worker_recent
    ON worker_spend_anomaly(worker_id, observed_at_micros DESC);
