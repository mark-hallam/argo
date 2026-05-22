# V1.1 — Worker behavior anomaly detector

**Owner**: claude. **Depends on**: H-2 (#800),
H-action-replay merged.

## Scope

Detect when a worker is behaving differently from its own
historical baseline. Per (worker_id, action_kind), compute a
running mean + stddev of latency, retry count, and
fail_rate. On every action emission, z-score the new sample
against the baseline; if any |z| > threshold (default 3.0),
write an anomaly event and surface via CLI.

This complements codex-circuit-breaker (which trips on
consecutive failures); the anomaly detector catches subtler
drifts like sudden latency spikes or higher retry counts
that don't yet result in failures.

- New `worker_behavior_baseline(worker_id, action_kind, mean_*,
  stddev_*, sample_count)` table; updated incrementally
  (Welford)
- New `worker_anomaly_event(id, worker_id, action_id, kind,
  z_score, dimension, observed_at)` table
- Hook in `WorkerAction::on_complete()` — update baseline,
  compute z, insert event if anomalous
- New CLI: `argo worker anomalies <id> [--since <duration>]`
  + `argo worker anomalies-watch <id>` (tail mode)
- Threshold env-overridable: `ARGO_ANOMALY_Z_THRESHOLD`
- TUI overlay key: `Z` opens anomaly feed for selected
  worker

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- EXHAUSTIVE-MATCH amendment applies
- `crates/argo-store/migrations/<next>__behavior_baseline.sql`
  (NEW)
- `crates/argo-store/migrations/<next+1>__anomaly_events.sql`
  (NEW)
- `crates/argo-store/src/anomaly.rs` (NEW)
- `crates/argo-runtime/src/anomaly.rs` (NEW) — Welford updater
  + z-score check
- `crates/argo-runtime/src/worker_action.rs` — narrowly:
  on_complete hook call
- `crates/argo-cli/src/handlers/worker_anomalies.rs` (NEW)
- `crates/argo-cli/src/cli_worker_cmds.rs` — `Anomalies` +
  `AnomaliesWatch` variants
- `crates/argo-tui/src/overlays/anomaly.rs` (NEW)
- `crates/argo-tui/src/key_router.rs` — narrowly: `Z` →
  anomaly overlay

## Acceptance criteria
- [ ] Welford running stats updated per action complete
- [ ] Anomaly events inserted when |z| > threshold
- [ ] CLI lists + tails events
- [ ] TUI `Z` overlay shows recent anomalies
- [ ] Threshold env-overridable + reads at startup
- [ ] 8 unit tests (Welford, z-score, threshold) + 2 CLI
      smoke + 1 TUI render snapshot
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR
- Branch: `claude-anomaly-detector`
- PR title: `worker: behavior anomaly detection with z-score baseline (claude-anomaly-detector)`
