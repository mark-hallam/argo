# mission-anomaly-alerting-notifychannel-v2 — anomaly alerting via NotifyChannelDispatch adapter

**Owner**: claude. **Depends on**: `argo-channels::NotifyChannel` trait (shipped); `argo-audit::watchdog` (shipped #1270 — pattern reference).
**Authored against**: origin/main `39b92666` on `2026-05-13`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-anomaly-alerting-notifychannel-v2`.

> **REWRITE 2026-05-13 by conductor (CEO-authorized Cluster D #5).** V1 (`mission-anomaly-alerting-notifychannel`) referenced nonexistent `NotifyChannelDispatch` type and assumed channels carry anomaly-shaped notifications (they carry approval-shaped). V2 adds the `NotifyChannelDispatch` adapter in argo-channels as a reusable abstraction, then wires the 3 unhandled triggers (provider 5xx storm, budget-ceiling hit, gateway boot fail).

## Scope

The audit-chain watchdog (#1270 / `argo-audit::watchdog`) is the existing precedent: it watches one signal (chain break), holds an `Arc<dyn Fn(ChainBreak) + Send + Sync>` callback, and fires alerts when the predicate trips. This brief generalizes that pattern across 3 more anomaly classes.

1. **Add `NotifyChannelDispatch` trait in argo-channels** — a small Send+Sync surface:
   ```rust
   #[async_trait]
   pub trait NotifyChannelDispatch: Send + Sync {
       async fn dispatch(&self, alert: AnomalyAlert) -> ArgoResult<()>;
   }
   pub struct AnomalyAlert {
       pub kind: AlertKind,
       pub severity: Severity,
       pub message: String,
       pub context: BTreeMap<String, String>,
   }
   pub enum AlertKind { ChainBreak, ProviderStorm, BudgetCeiling, GatewayBootFail, Custom(String) }
   pub enum Severity { Warn, Critical }
   ```

2. **Default impl `MultiChannelDispatch`** wraps N concrete `NotifyChannel` instances and fan-outs by `AlertKind` → channel mapping (config-driven).

3. **Wire 3 unhandled emission sites** to call `NotifyChannelDispatch::dispatch`:
   - `argo-provider::breaker_observability.rs:98` — on `circuit_breaker_state_transition` to Open: dispatch `ProviderStorm` alert. ≤15 LOC.
   - `argo-store::spend_ceiling.rs` — on ceiling-hit return path: dispatch `BudgetCeiling` alert. ≤20 LOC.
   - `argo-cli/src/handlers/gateway/startup_check.rs` — on `Err(ArgoError::Provider(...))` return: dispatch `GatewayBootFail` alert. ≤15 LOC.

4. **Existing watchdog rewires** to dispatch `ChainBreak` through the new adapter so all 4 anomaly classes share one notification surface. ≤20 LOC.

5. **Config**: `[anomaly_alerting]` section in `sections_context.rs` with `enabled: bool`, `default_severity_threshold: Severity`, `channel_routing: BTreeMap<AlertKind, Vec<String>>` (alert-kind → channel-id list).

**Not in scope**: anomaly *detection* beyond the 4 emission sites (no metric-threshold rules / event-filters from v1); UI for editing routing config; per-tenant alert isolation (v1 is process-global); custom AlertKinds beyond the 4 named.

## Files you may touch

- `<NEW: crates/argo-channels/src/dispatch.rs>` (≤200 LOC) — `NotifyChannelDispatch` trait + `AnomalyAlert` + `MultiChannelDispatch` default impl.
- `<NEW: crates/argo-channels/src/dispatch_tests.rs>` (≤200 LOC) — 6 unit tests: dispatch-fans-out, severity-filter, missing-channel-skip, error-on-one-channel-isolated, kind-routing, debounce-when-implemented.
- `crates/argo-channels/src/lib.rs` — narrowly: `pub mod dispatch;` + re-exports.
- `crates/argo-provider/src/breaker_observability.rs` — narrowly: ≤15 modified lines for breaker-Open dispatch hook.
- `crates/argo-store/src/spend_ceiling.rs` — narrowly: ≤20 modified lines for budget-ceiling dispatch.
- `crates/argo-cli/src/handlers/gateway/startup_check.rs` — narrowly: ≤15 modified lines for boot-fail dispatch.
- `crates/argo-cli/src/handlers/gateway/boot_spawns.rs` — narrowly: construct `MultiChannelDispatch` at boot, plumb to the 4 emitters via `AgentDeps` or equivalent. ≤30 modified lines.
- `crates/argo-config/src/sections_context.rs` — narrowly: add `AnomalyAlertingConfig` (defaults: enabled=true, threshold=Warn, empty routing). ≤30 modified lines.
- `crates/argo-audit/src/watchdog.rs` — narrowly: ≤20 modified lines to route ChainBreak through the new adapter.

## Files you must NOT touch

- `crates/argo-channels/src/{discord,email,slack,matrix,imessage}.rs` — concrete channel impls are locked.
- `crates/argo-audit/src/sink.rs` — audit emission contract is locked.
- `crates/argo-cli/src/handlers/worker_actions_notify.rs` — CLI-private notify path stays put.

## Acceptance criteria

- [ ] `NotifyChannelDispatch` trait + `AnomalyAlert` + `MultiChannelDispatch` impl shipped in argo-channels.
- [ ] All 4 emission sites (breaker-Open / budget-ceiling / boot-fail / chain-break) call `dispatch` on the right alert kind.
- [ ] Boot wiring constructs `MultiChannelDispatch` from `AnomalyAlertingConfig` + the per-channel `NotifyChannel` registry.
- [ ] Disabled-config path (`enabled = false`) returns a `NoopDispatch` that drops alerts silently — zero overhead when off.
- [ ] 6 unit tests + 1 integration smoke (boot with 1 dummy channel; trigger all 4 alert kinds; assert all 4 dispatched).
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-mission-anomaly-alerting-notifychannel-v2`
- PR title: `channels: NotifyChannelDispatch adapter + wire 4 anomaly classes (mission-anomaly-alerting-notifychannel-v2)`

## Notes for the implementer

- The watchdog precedent (#1270) is the right pattern reference. Mirror its `Arc<dyn Fn>` callback shape but extend to the trait-object dispatch (not just one callback).
- Anomaly detection BEYOND the 4 named emission sites (e.g. metric-threshold rules, event-filter subscribers) is explicitly v2.1 — don't expand scope.
- Per-tenant alert isolation is also v2.1.
- Make the 3 new emission sites configurable: each can be disabled via `AnomalyAlertingConfig.channel_routing` so operators can opt out of any specific alert class without recompiling.
