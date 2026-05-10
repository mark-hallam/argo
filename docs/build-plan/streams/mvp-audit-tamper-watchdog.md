# mvp-audit-tamper-watchdog — periodic background verifier with NotifyChannel alarm

**Owner**: claude. **Depends on**: nothing. **Tier P (production-readiness, MVP).**

**Authored against**: origin/main 5baf15a6 on 2026-05-10.

## Why

MISSION_BACKLOG item #37: "Add audit-chain watchdog that periodically
verifies and pages on tamper." Today `verify_all` / `verify_session`
exist (`crates/argo-audit/src/verify.rs`, 166 LOC) and surface
`ChainBreak` reports, but nothing calls them on a schedule — chain
tampering would be invisible until a manual `argo audit verify`. For
a v1 production gateway we want continuous detection.

## Approach (concrete)

1. **New module `crates/argo-audit/src/watchdog.rs`** (≤ 200 LOC).
   Public API:
   ```rust
   pub struct WatchdogConfig {
       pub interval: Duration,        // default 5 min
       pub max_consecutive_failures: u32, // default 3 before alarm
   }
   pub async fn spawn_watchdog(
       store: Arc<SessionStore>,
       cfg: WatchdogConfig,
       on_break: Arc<dyn Fn(ChainBreak) + Send + Sync>,
   ) -> tokio::task::JoinHandle<()>;
   ```
   Loop: every `interval`, call `verify_all(&store).await`. On any
   `ChainBreak`, invoke `on_break` and increment a failure counter.
   On consecutive_failures ≥ `max_consecutive_failures`, also emit a
   `tracing::error!` with structured fields. Reset counter on a clean
   verify.
2. **Wire it into gateway boot.** In
   `crates/argo-cli/src/handlers/gateway/boot.rs`, alongside the
   existing `_sweeper` / `_alerts_loop` / `_cu_screenshot_sweep`
   handles, add `let _audit_watchdog = store.as_ref().map(|s|
   audit::watchdog::spawn_watchdog(Arc::clone(s), Default::default(),
   default_alarm_callback()));`. Default callback writes to
   `tracing::error!`; an operator-supplied NotifyChannel hook is
   out-of-scope here (follow-up brief).
3. **Re-export from `crates/argo-audit/src/lib.rs`**: `pub use
   watchdog::{spawn_watchdog, WatchdogConfig};`.
4. **3 unit tests** in `watchdog.rs` (inline `#[cfg(test)]`):
   - `clean_chain_no_alarm` — verify loop runs 3× on a clean store,
     callback never fires.
   - `tampered_chain_fires_callback` — manually corrupt one row,
     callback receives a `ChainBreak`.
   - `consecutive_failure_threshold` — counter resets on clean
     verify after a streak.

## Files you may touch

- `crates/argo-audit/src/watchdog.rs` (NEW, ≤ 200 LOC)
- `crates/argo-audit/src/lib.rs` (add 1 mod + 1 pub use line; ample
  headroom — currently 45 LOC)
- `crates/argo-cli/src/handlers/gateway/boot.rs` (1 spawn call near
  the other background handles; currently 427 LOC, headroom ~70)
- `crates/argo-audit/Cargo.toml` (only if a tokio dependency feature
  needs widening — likely already set)

## Files you must NOT touch

- `crates/argo-audit/src/verify.rs` — leave the verify primitive
  unchanged. The watchdog is a consumer.
- `crates/argo-audit/src/sink.rs` — no chain protocol changes.
- Any non-gateway boot path.

## Acceptance criteria

- [ ] Watchdog runs in the background of every `argo gateway start`
      invocation (visible via a single startup log line).
- [ ] Tampered chain triggers the callback within `interval` seconds.
- [ ] 3 new unit tests pass.
- [ ] `cargo clippy -p argo-audit -p argo-cli --all-targets -- -D
      warnings` clean.
- [ ] `cargo test -p argo-audit` green.
- [ ] No file crosses 500 LOC.

## Branch + PR

- Branch: `claude-mvp-audit-tamper-watchdog`
- PR title: `audit: periodic chain-verify watchdog with alarm callback (mvp-audit-tamper-watchdog)`
