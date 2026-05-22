# claude-mcp-server-rate-limit — per-server token-bucket rate limiting on MCP dispatch

**Owner**: claude. **Depends on**: `mission-wire-mcp-action-executor` (#933) merged.

## Why

Today nothing prevents a runaway action from hammering an MCP server with thousands of `tool/call` invocations in a tight loop. The downstream server takes the hit, our reputation takes the hit, and other tenants sharing that server (per `codex-mcp-server-registry-cli`) see latency spikes. This brief introduces a **per-server token-bucket rate limiter** wired into the dispatch path so MCP calls back off when a server's rate budget is exhausted.

## Scope

1. **NEW** `crates/argo-cli/src/handlers/mcp_rate_limit.rs` (~320 LOC):
   - `TokenBucket` struct: `capacity`, `refill_rate_per_sec`, `tokens: AtomicF64`, `last_refill: Instant`
   - `try_acquire(n: f64) -> bool` — non-blocking check
   - `acquire_with_timeout(n: f64, max_wait: Duration) -> ArgoResult<()>` — async wait up to `max_wait`; returns `Err(ArgoError::Provider("rate limit exceeded for <server>"))` on timeout
   - `RateLimiter` registry: `Arc<DashMap<ServerId, Arc<TokenBucket>>>` (or `Mutex<HashMap>` if `dashmap` isn't a workspace dep)
   - `from_config(config: &McpRateLimitConfig) -> RateLimiter` — loads per-server caps; falls back to a default bucket for unspecified servers
2. **Wire into dispatch (narrow)** — modify `worker_actions_executor_mcp.rs`:
   - Before each `tool/call`, call `limiter.acquire_with_timeout(1.0, Duration::from_secs(30))` for the relevant `server_id`
   - On rate limit error, return `Err(ArgoError::Provider("rate limit exceeded for <server>"))` (clean fail; let upstream restart strategy decide)
3. **Config** — extend `manifests/connectors/mcp.toml` (or a sibling `mcp_rate_limit.toml`) with per-server `capacity` and `refill_rate_per_sec`. Defaults: 60 capacity, 10/sec refill (= 10 calls/s burst-tolerant up to 60).
4. **Telemetry** — emit `mcp.rate_limit` event with `{server_id, decision, wait_ms}` so dashboards can spot tenants throttling

## Files you may touch

- `crates/argo-cli/src/handlers/mcp_rate_limit.rs` (NEW, ≤ 380 LOC)
- `crates/argo-cli/src/handlers/mcp_rate_limit_tests.rs` (NEW, ≤ 280 LOC sibling) — 8 unit tests:
  - try_acquire returns true while tokens available
  - try_acquire returns false when bucket empty
  - acquire_with_timeout waits then succeeds when bucket refills
  - acquire_with_timeout returns rate-limit error after timeout
  - per-server isolation (server A throttled doesn't block server B)
  - default bucket for unconfigured servers
  - refill rate honored (mock clock; assert tokens after elapsed time)
  - capacity ceiling enforced (no overflow on long idle)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 1 `pub mod` line
- `crates/argo-cli/src/handlers/worker_actions_executor_mcp.rs` — narrowly: ~25 LOC delta around the `tool/call` site (DO NOT exceed 500 LOC ceiling — extract acquisition into a helper if needed)
- `manifests/connectors/mcp.toml` — narrowly: add `[rate_limit.<server_id>]` example block + parsing
- `crates/argo-cli/tests/mcp_rate_limit_smoke.rs` (NEW, ≤ 200 LOC) — 1 integration test that fires 100 calls in 1s against a server capped at 10/s and asserts ~10 succeed-immediately + ~90 either wait-or-error

## Files you must NOT touch

- `argo-mcp-connector-*` crates — protocol library
- `argo-vertical/src/manifest.rs`
- `argo-store` migrations / schema
- The `Executor` trait

## Acceptance criteria

- [ ] Per-server token bucket enforces configured `capacity` + `refill_rate_per_sec`
- [ ] `try_acquire` is non-blocking; `acquire_with_timeout` waits up to N
- [ ] Per-server isolation (A throttled doesn't block B)
- [ ] Rate-limit timeout → clean `ArgoError::Provider("rate limit exceeded for <server>")`
- [ ] Telemetry event `mcp.rate_limit` fires per acquisition
- [ ] Default bucket (60/10s) applied when server has no explicit config
- [ ] 8 unit tests + 1 integration smoke
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli` clean
- [ ] Every file < 500 LOC (including the modified `worker_actions_executor_mcp.rs`)

## Branch + PR

- Branch: `claude-mcp-server-rate-limit`
- PR title: `mcp: per-server token-bucket rate limit on dispatch (claude-mcp-server-rate-limit)`

## Notes for the implementer

- If `dashmap` is not yet a workspace dep, use `Arc<tokio::sync::RwLock<HashMap<...>>>` instead — do not add a new workspace dep just for this brief.
- Use `tokio::time::sleep` for the wait inside `acquire_with_timeout`. Tests use `tokio::time::pause()` + `advance()` for deterministic clock control.
- The 30s default `max_wait` matches the `DEFAULT_ACTION_TIMEOUT` from #933 — keep them in sync.
- If `worker_actions_executor_mcp.rs` is at-or-near 500 LOC, extract the acquisition wrapper into `worker_actions_executor_mcp_rate.rs` and call from `dispatch_action`.
- Backpressure-vs-fail decision: this brief takes "wait then fail" (bounded). Pure fail-fast (no waiting) and pure backpressure (infinite wait) are both wrong defaults; bounded wait splits the difference.
- If `claude-mcp-server-health-check` lands first, do NOT integrate the limiter with health-pings — health-pings should ignore rate limits (they're system traffic, not tenant traffic). Add a bypass path or simply don't call `acquire_with_timeout` from `mcp_health.rs`.
