-- Stream `claude-spend-anomaly-alerts` — per-(worker, hour-of-day)
-- spend baseline used by the hourly z-score check.
--
-- Baseline samples are USD-per-hour aggregates of `worker_budget`
-- charges, bucketed by the UTC hour-of-day they fired in. The
-- Welford rolling-stats fields (mean, M2 for variance) update
-- incrementally so the table stays one-row-per-(worker, hour)
-- regardless of history depth.

CREATE TABLE IF NOT EXISTS worker_spend_baseline (
    -- Worker the baseline tracks (matches `worker_budget.worker_id`).
    worker_id TEXT NOT NULL,
    -- 0..=23 — UTC hour-of-day this baseline summarises.
    hour_of_day INTEGER NOT NULL,
    -- Welford running mean of USD-per-hour spend.
    mean_usd_per_hour REAL NOT NULL,
    -- Welford running M2 (sum of squared deviations). stddev is
    -- derived as sqrt(M2 / (n-1)) at read time.
    m2_usd_per_hour REAL NOT NULL,
    -- Number of samples folded into this baseline.
    sample_count INTEGER NOT NULL,
    -- UTC microseconds at which this row was last touched.
    updated_at_micros INTEGER NOT NULL,
    PRIMARY KEY (worker_id, hour_of_day)
);
