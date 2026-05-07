# claude-cu-fail-restart-strategy — snapshot / reset / retry-with-context on unrecoverable CU errors

**Owner**: claude. **Depends on**: `argo-executor-computer-use` (#787) merged.

## Why

Today, when a CU executor hits an unrecoverable error mid-trajectory (model emits malformed action, sandbox session times out, screenshot capture fails), the action returns `HumanHandoff` and the run dies. Operators report the most-common failure modes are transient — a stale browser tab, a renderer hiccup, a model response that referred to coordinates from an earlier screenshot — all things a fresh sandbox + a re-prompt with last known goal would recover.

This brief codifies a **snapshot / reset / retry-with-context** restart strategy so transient failures recover automatically up to a bounded retry budget, while genuinely-broken runs still fall through to handoff.

## Scope

1. **NEW** `crates/argo-executor-computer-use/src/restart.rs` (~320 LOC):
   - `RestartStrategy` enum: `None`, `RetryWithContext { max_attempts, backoff_ms }`, `SnapshotResetRetry { max_attempts, snapshot_every_n_steps }`
   - `RestartCoordinator` struct holding strategy + attempt counter + last-known-good snapshot
   - `should_restart(err, attempts) -> RestartDecision { Continue, Retry { with_context }, Bail }`
   - `take_snapshot(session) -> Snapshot { last_screenshot, last_action_idx, goal_summary }`
   - `restore_from(snapshot, fresh_session) -> ArgoResult<()>` — re-seeds new session with goal summary + last screenshot in system prompt
2. **Wire into the CU executor loop** — narrowly modify `argo-executor-computer-use/src/executor.rs`:
   - Around the existing per-step error path, consult `RestartCoordinator::should_restart`
   - On `Retry` decision: tear down current sandbox session, open fresh, restore from snapshot, continue from snapshot index
   - On `Bail` decision: existing `HumanHandoff` path unchanged
3. **CU manifest** — extend `manifests/executors/computer_use.toml` with optional `restart` subsection. Use the existing TOML schema (it already accepts ad-hoc executor profile fields) — no `argo-vertical` schema change.
4. **Telemetry** — emit a `cu.restart` event on every restart with `{reason, attempt, success}` so operators can tune budgets

## Files you may touch

- `crates/argo-executor-computer-use/src/restart.rs` (NEW, ≤ 380 LOC)
- `crates/argo-executor-computer-use/src/restart_tests.rs` (NEW, ≤ 280 LOC sibling) — 7 unit tests:
  - `should_restart` exhausts retry budget then bails
  - `should_restart` distinguishes retryable vs fatal error kinds
  - `take_snapshot` captures the right step index
  - `restore_from` re-seeds the prompt with goal summary
  - SnapshotResetRetry with `snapshot_every_n_steps = 3` snapshots at the right cadence
  - RetryWithContext (no snapshot) preserves trajectory log across attempts
  - None strategy is a no-op
- `crates/argo-executor-computer-use/src/executor.rs` — narrowly: ~30 LOC delta around the per-step error branch
- `crates/argo-executor-computer-use/src/lib.rs` — narrowly: `mod restart;` declaration
- `manifests/executors/computer_use.toml` — narrowly: add commented-out `[restart]` example block + parsing in the loader
- `crates/argo-executor-computer-use/tests/restart_smoke.rs` (NEW, ≤ 200 LOC) — 1 integration test with a flaky-mock sandbox that fails twice then succeeds; asserts retry path

## Files you must NOT touch

- `argo-vertical/src/manifest.rs` — restart config rides in the executor manifest, not the vertical manifest schema
- The `Executor` trait
- Other executor crates (`argo-executor-computer-use-macos`, future `-linux`) — they get the restart layer when they migrate to the trait shape; out of scope here
- `argo-store` / `argo-store-attached`

## Acceptance criteria

- [ ] `RestartStrategy::SnapshotResetRetry` recovers a synthetic transient failure on the second attempt
- [ ] `RestartStrategy::RetryWithContext` recovers without taking a fresh sandbox
- [ ] Retry budget exhaustion triggers the existing `HumanHandoff` path unchanged
- [ ] Telemetry event `cu.restart` fires with `{reason, attempt, success}`
- [ ] `restart` config in `computer_use.toml` parses cleanly; absent config defaults to `None`
- [ ] 7 unit tests + 1 integration smoke
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-executor-computer-use` clean
- [ ] Every file < 500 LOC

## Branch + PR

- Branch: `claude-cu-fail-restart-strategy`
- PR title: `executor(cu): snapshot/reset/retry-with-context restart strategy (claude-cu-fail-restart-strategy)`

## Notes for the implementer

- Distinguish retryable vs fatal errors via the existing `ArgoError` taxonomy: `Provider` errors carrying `transport`, `timeout`, `parse` are retryable; `Provider("budget exhausted")`, `Provider("user_aborted")`, and `Permission(_)` are fatal.
- Goal summary for snapshot is the original action's `description` field plus the last 200 chars of the trajectory log — no model call required.
- Default strategy if config is absent: `None` (preserve existing behaviour). Operators opt-in via `computer_use.toml`.
- Retry budget across the whole action — not per-step. Restarts that succeed reset the budget? **No**: cumulative cap of `max_attempts` to bound worst-case runtime.
- Telemetry uses the existing `tracing` macros; do not add new exporters.
