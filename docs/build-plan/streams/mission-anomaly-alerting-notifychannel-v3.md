# mission-anomaly-alerting-notifychannel-v3 — anomaly dispatch (rename to avoid AlertKind collision)

**Owner**: claude. **Depends on**: `argo-channels::NotifyChannel` trait; shipped `argo-cli/src/handlers/alerts.rs::AlertKind` (operator rule-based predicates).
**Authored against**: origin/main `94370ca5` on `2026-05-13`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-anomaly-alerting-notifychannel-v3`.

> **REWRITE 2026-05-13 by conductor (CEO-authorized after audit-3 INTENT-DRIFT finding).** V2 proposed `AnomalyAlert { kind: AlertKind, ... }` + `pub enum AlertKind { ChainBreak, ProviderStorm, BudgetCeiling, GatewayBootFail, Custom(String) }` in argo-channels — **name-colliding** with shipped `pub enum AlertKind { SpendTotal, ErrorRate, ApprovalTimeout }` at `crates/argo-cli/src/handlers/alerts.rs:44`. V3 renames to `AnomalyKind`/`AnomalyDispatch`/`AnomalyAlert` + adds explicit relationship section.

## Scope

Same scope as V2 — `NotifyChannelDispatch` adapter in argo-channels + wire the 3 unhandled triggers (provider 5xx, budget hit, boot fail) + rewire watchdog — but with **renamed types and an explicit complementarity model** to the shipped operator-rule alerts.

## Relationship to shipped `handlers/alerts.rs`

- **`handlers/alerts.rs::AlertKind`** (shipped) is for **operator-configured rule predicates** over time-windowed metric snapshots (`SpendTotal { tenant_id, threshold, window }`, `ErrorRate { route, threshold_pct, window }`, `ApprovalTimeout { window }`). Driven by operator config; fires from a snapshot loop.
- **`AnomalyKind`** (THIS BRIEF) is for **event-source-driven anomaly emission** at the moment a bad thing happens (`ProviderStorm` at breaker-Open, `BudgetCeiling` at ceiling-hit, `GatewayBootFail` at startup-check, `ChainBreak` at audit-watchdog). NOT operator-configurable predicates; the emission sites decide.
- **Composability** (for a future brief): the new `AnomalyDispatch` could feed an alert-rule input so the operator-rule engine sees both sources uniformly. NOT in this brief's scope.

Renamed types:
```rust
// in argo-channels — distinct names from handlers/alerts.rs::AlertKind
pub enum AnomalyKind { ChainBreak, ProviderStorm, BudgetCeiling, GatewayBootFail, Custom(String) }
pub struct AnomalyAlert { pub kind: AnomalyKind, pub severity: Severity, pub message: String, pub context: BTreeMap<String, String> }
pub trait AnomalyDispatch: Send + Sync { async fn dispatch(&self, alert: AnomalyAlert) -> ArgoResult<()>; }
pub struct MultiChannelDispatch { /* fan-out to N concrete NotifyChannels */ }
```

Otherwise behavior is identical to V2: wire `ProviderStorm` at `breaker_observability.rs:98`, `BudgetCeiling` at `spend_ceiling.rs`, `GatewayBootFail` at `startup_check.rs`, and route `ChainBreak` through the same surface for consistency.

**Not in scope**: changing `handlers/alerts.rs::AlertKind` (locked); building the composability bridge between AnomalyDispatch and the rule engine (separate v3.1 brief); per-tenant alert routing.

## Files you may touch

Same may-touch list as V2 (3 emission sites + boot wiring + argo-channels new module + config), only the type names change. ≤400 LOC.

## Acceptance criteria

- [ ] Types named `AnomalyKind` / `AnomalyDispatch` / `AnomalyAlert` (NOT `AlertKind` / `AlertDispatch` / etc.). No collision with `handlers/alerts.rs::AlertKind`.
- [ ] All 4 emission sites wired (breaker-Open / budget-ceiling / boot-fail / chain-break).
- [ ] Relationship section in PR body explains the operator-rule-alerts vs event-source-anomalies distinction.
- [ ] Disabled-config path returns `NoopDispatch` (zero overhead).
- [ ] 6 unit tests + 1 integration smoke.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-mission-anomaly-alerting-notifychannel-v3`
- PR title: `channels: AnomalyDispatch + wire 4 anomaly classes (mission-anomaly-alerting-notifychannel-v3)`

## Notes for the implementer

- The renames are load-bearing. Don't introduce `AlertKind` anywhere — it's reserved for `handlers/alerts.rs`.
- The relationship-to-alerts.rs section in the PR body is mandatory: explain the two-layer model so future readers don't re-trigger the audit confusion.
