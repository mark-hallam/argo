# Mission load-bearing wire — wire CU sandbox pool reaper from gateway boot

**Owner**: claude. **Depends on**: `SandboxPool::reap_idle` exists (verified at `crates/argo-executor-computer-use/src/pool.rs:128`).
**Authored against**: origin/main `c91e37e5` on `2026-05-10`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-cu-sandbox-pool-reaper`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 5 item #96 ("Wire CU sandbox pool reaper — `reap_idle` exists, no caller"). The pool's idle-TTL reaper is wired in code at `pool.rs:128` with a unit test at line 193, but nothing in the gateway boot path spawns a periodic tick to call it. Result: idle E2B sandboxes accumulate per `(tenant, persona, profile)` key until process restart — the 10-min TTL is a marketing claim, not a runtime invariant. Mission Claim 5 ("real-world execution") needs the pool to actually reap, not just declare an intent.

## Scope

Add a periodic background task that calls `SandboxPool::reap_idle` on a configurable interval (default: 60s). Spawn it from `crates/argo-cli/src/handlers/gateway/boot.rs` alongside the existing `_sweeper` / `_alerts_loop` / `_cu_screenshot_sweep` handles, gated on whether the gateway has a `SandboxPool` available (the CU executor pool is constructed only when CU is enabled in the manifest).

Extract the spawn + tick loop into a small NEW module `crates/argo-executor-computer-use/src/pool_reaper.rs` (≤ 80 LOC) so the gateway boot path stays declarative: `let _cu_pool_reaper = pool.as_ref().map(|p| spawn_pool_reaper(p.clone(), Duration::from_secs(60)));`. The reaper logs reap counts at INFO when > 0 sandboxes were closed, otherwise at TRACE.

This is NOT in scope: changing the 10-min idle TTL itself; making the interval per-tenant configurable (the global default is fine for v1); adding metrics emission (do that in a follow-up brief if needed); reaper backpressure or jitter (a fixed-interval ticker is adequate at this load).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-executor-computer-use/src/pool_reaper.rs` (NEW, ≤ 80 LOC) — `pub fn spawn_pool_reaper(pool: SandboxPool, interval: Duration) -> tokio::task::JoinHandle<()>` that loops `tokio::time::interval(interval).tick().await; pool.reap_idle().await` with INFO/TRACE logging.
- `crates/argo-executor-computer-use/src/lib.rs` — narrowly scoped: ONE `pub mod pool_reaper;` line and ONE re-export `pub use pool_reaper::spawn_pool_reaper;`. No other edits.
- `crates/argo-cli/src/handlers/gateway/boot.rs` — narrowly scoped: ONE block adding `let _cu_pool_reaper = ...` next to the existing background handles. Use the same `Option`-mapping pattern. Confirm the `SandboxPool` accessor on the gateway state struct (verify in first 5 minutes; if no accessor exists, STOP and BLOCK with reason "gateway boot lacks SandboxPool accessor — needs upstream brief").
- `crates/argo-executor-computer-use/src/pool_reaper_tests.rs` (NEW) — 3 unit tests: reaper-ticks-and-calls-reap-idle (use a fake pool with counter); reaper-aborts-cleanly-on-drop (assert handle.abort + JoinHandle status); reaper-survives-reap-error (when reap_idle returns Err, log and continue, don't crash the loop).
- `crates/argo-cli/tests/cu_pool_reaper_smoke.rs` (NEW) — integration smoke: boot gateway with CU enabled, observe `_cu_pool_reaper` handle is alive after 2 ticks (use a 1-second interval for the test).

## Files you must NOT touch

- `crates/argo-executor-computer-use/src/pool.rs` — `reap_idle` itself is upstream contract; reuse, don't modify
- `crates/argo-executor-computer-use/src/e2b.rs` — provisioning is independent
- Any other gateway handler beyond `boot.rs`
- `crates/argo-store/migrations/**` — no schema change

## Acceptance criteria

- [ ] `pool_reaper.rs` exists with `spawn_pool_reaper(pool, interval) -> JoinHandle<()>` exported from `argo-executor-computer-use`
- [ ] The spawned task ticks at `interval` and calls `pool.reap_idle().await` each tick
- [ ] When `reap_idle` returns `Ok(n)` with `n > 0`, log at INFO with the reap count; when `n == 0`, log at TRACE
- [ ] When `reap_idle` returns `Err`, log at WARN with the error and continue the loop (do NOT panic, do NOT exit)
- [ ] `boot.rs` spawns `_cu_pool_reaper` from the gateway boot path when `SandboxPool` is available; behavior unchanged when CU is disabled
- [ ] 3 unit tests in `pool_reaper_tests.rs` cover happy-tick, clean-abort-on-drop, error-survival
- [ ] 1 integration smoke in `tests/cu_pool_reaper_smoke.rs` exercises end-to-end boot + 2 ticks
- [ ] `cargo build -p argo-cli -p argo-executor-computer-use` clean
- [ ] `cargo clippy -p argo-executor-computer-use --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-executor-computer-use` passes (skip pre-existing flakes per KNOWN_FLAKES.md)
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-cu-sandbox-pool-reaper`
- PR title: `cu-executor: spawn pool reaper from gateway boot (mission-cu-sandbox-pool-reaper)`

## Notes for the implementer

- The existing background handles in boot.rs use an `AbortOnDrop` wrapper (see `_alerts_loop` at line 274 area). Use the same wrapper for `_cu_pool_reaper` so the reaper aborts cleanly when the gateway shuts down.
- For test fixturing, build a fake pool via the existing `SandboxPool::new` constructor with a stub `E2bClient` (the pool is empty, `reap_idle` returns 0 each tick — that's fine for the spawn test).
- The interval default of 60s matches the idle TTL of 600s — the reaper sweeps 10× per TTL window, so a sandbox idle for >TTL is closed within one TTL+interval. Don't make this configurable in v1; document the choice in the doc-comment of `spawn_pool_reaper`.
- If `SandboxPool::reap_idle` doesn't actually return a count of reaped sandboxes (re-verify the signature first 5 minutes), adapt the logging accordingly. Today's signature returns `Result<usize, CuError>` per pool.rs:128.
