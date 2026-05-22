# claude-alerts-config-system — Configurable alerts routed to channels

**Owner**: claude. **Depends on**: gateway boot (existing) — adds a
background alert evaluator task.

## Scope

Operators currently have to tail logs or run `argo doctor` to spot trouble.
This brief adds a declarative alerts file (`.skycot/alerts.toml`) that
defines threshold rules (spend, error rate, approval timeout) and routes
firings to channels (file log, gateway event, future webhook).

A background task in the gateway boot path evaluates each rule on a fixed
cadence (default 60s) and emits an alert event when a threshold is crossed.
Hysteresis prevents repeat-firing: once fired, a rule re-arms only after
the underlying metric falls back below threshold.

## Approach (concrete)

1. **NEW** `crates/argo-cli/src/handlers/alerts.rs` (~320 LOC):
   - `AlertsConfig` (serde) — root struct with `Vec<AlertRule>`
   - `AlertRule { name, kind: AlertKind, threshold, window, channels }`
   - `AlertKind` enum: `SpendTotal { window }`, `ErrorRate { route, window }`,
     `ApprovalTimeout { older_than }`
   - `evaluate_once(state: &GatewayState, cfg: &AlertsConfig) -> Vec<AlertFiring>`
   - `pub async fn run_alerts_loop(state: Arc<GatewayState>, cfg: AlertsConfig)`
     — `tokio::spawn` target, 60s tick via `tokio::time::interval`
   - `load_alerts_config(path: &Path) -> ArgoResult<AlertsConfig>` — reads
     `.skycot/alerts.toml`, returns empty cfg if file absent (not an error)
2. **Modify** gateway boot (`crates/argo-cli/src/handlers/gateway/boot.rs` or
   equivalent — search for `gateway::serve`):
   - After server bind: `tokio::spawn(alerts::run_alerts_loop(state.clone(), cfg))`
3. **Modify** `crates/argo-cli/src/handlers/mod.rs`: 1 pub mod line.

## Config example (`.skycot/alerts.toml`)

```toml
[[rule]]
name = "spend-day"
kind = "spend_total"
threshold_usd = 50.0
window = "24h"
channels = ["file:.skycot/alerts.log", "event"]

[[rule]]
name = "error-rate-actions"
kind = "error_rate"
route = "/v1/actions/dispatch"
threshold_pct = 5.0
window = "10m"
channels = ["file:.skycot/alerts.log"]
```

## Files you may touch

- `crates/argo-cli/src/handlers/alerts.rs` (NEW, ≤ 350 LOC)
- `crates/argo-cli/src/handlers/alerts_tests.rs` (NEW, ≤ 280 LOC) — 7 tests:
  - parses minimal config
  - parses full multi-rule config
  - missing config file → empty cfg (no error)
  - spend rule fires at threshold
  - spend rule does not re-fire while still above
  - error-rate rule fires
  - approval-timeout rule fires
- `crates/argo-cli/src/handlers/gateway/boot.rs` — narrow: spawn alerts loop
- `crates/argo-cli/src/handlers/mod.rs` — narrow: 1 pub mod line
- `.skycot/alerts.toml.example` (NEW) — the config example above

## Files you must NOT touch

- argo-store schema — derive metrics from existing spend/event tables
- argo-types — no new public types
- Any other handler — alerts module is self-contained
- Webhook delivery — out of scope for this brief (file + event channels only)

## Acceptance criteria

- [ ] `.skycot/alerts.toml` parses cleanly (or absent → empty cfg, no error)
- [ ] Background task starts on gateway boot, ticks every 60s
- [ ] Spend, error-rate, and approval-timeout rules each fire correctly
- [ ] Hysteresis: a fired rule does not re-fire until metric falls below
- [ ] Firings written to configured file channel as one JSON line per firing
- [ ] Firings emitted as gateway events when `channels` includes `"event"`
- [ ] 7 unit tests
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-alerts-config-system`
- PR title: `alerts: configurable rules + background evaluator (claude-alerts-config-system)`

## Notes for the implementer

- Keep the evaluator pure: `evaluate_once` takes a state snapshot + cfg and
  returns firings. The loop just wraps it with the timer and channel write.
- Use `time::Duration` parsed from `"24h"` / `"10m"` via a tiny helper —
  no new crate dep.
- Hysteresis state lives in-memory on the loop task (a `HashSet<rule_name>`
  of currently-fired rules). Crash-restart resets — that's fine for v1.
