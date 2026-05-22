# V1.1 — Spend anomaly alerts

**Owner**: claude. **Depends on**: H-spend-persistence
(#809), H-budget-cli (#820), claude-anomaly-detector
(in-flight) merged.

## Scope

A worker burning budget faster than its historical baseline
should ALERT before it actually breaches the cap. The
`claude-anomaly-detector` brief tracks per-action behavior
metrics. This complementary brief specifically covers SPEND:
sudden burn-rate spikes (e.g. "this worker has averaged
$2/hour for 30 days; in the last hour it burned $20").

- New `worker_spend_baseline(worker_id, hour_of_day,
  mean_usd_per_hour, stddev_usd_per_hour, sample_count)`
  table; updated rolling
- Hourly tick (driven by existing audit-stream broadcaster
  or a fresh tokio interval) computes z-score for the
  current hour's spend vs. baseline-for-hour-of-day
- |z| > threshold (default 4.0; env override
  `ARGO_SPEND_ANOMALY_Z`) inserts a `worker_spend_anomaly`
  row + fires NotifyChannel alert (re-uses existing
  channels)
- New CLI: `argo budget anomalies [--worker <id>]
  [--since <duration>]`

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-store/migrations/<next>__spend_baseline.sql`
  (NEW)
- `crates/argo-store/migrations/<next+1>__spend_anomaly.sql`
  (NEW)
- `crates/argo-store/src/spend_anomaly.rs` (NEW)
- `crates/argo-cli/src/handlers/budget.rs` — narrowly:
  hourly anomaly-check tick OR delegate to a new sibling
- `crates/argo-cli/src/handlers/budget_anomalies.rs` (NEW)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly:
  `pub mod` for new handler
- `crates/argo-cli/src/cli_subcommands.rs` — narrowly:
  add `Anomalies` variant to the `Budget` subcommand

## Acceptance criteria
- [ ] Welford rolling stats per (worker, hour-of-day)
- [ ] z-score check writes anomaly rows + fires notify
- [ ] Threshold env-overridable
- [ ] `argo budget anomalies` lists + filters
- [ ] `--json` universal output
- [ ] 8 unit tests (rolling stats + z-score + threshold) +
      1 integration test
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR
- Branch: `claude-spend-anomaly-alerts`
- PR title: `budget: hourly spend anomaly detection + alerts (claude-spend-anomaly-alerts)`
