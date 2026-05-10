# Mission load-bearing wire — connector auto-retry with exponential backoff

**Owner**: claude. **Depends on**: existing MCP connectors (e.g. `crates/argo-mcp-connector-gcal/src/client.rs`) and connector-base patterns.
**Authored against**: origin/main `2a386ed5` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-connector-auto-retry-backoff`.

> SOURCE: MISSION_BACKLOG.md Section 9 #251 ("Connector auto-retry with exponential backoff"). MCP connectors today fail on first transient network error (timeout, 5xx, connection reset). Operators see flaky tool calls in normal load. Add a shared retry layer with exponential backoff so transient failures don't surface to the agent.

## Scope

Add a NEW shared module `argo-mcp-connector-base::retry` (or wherever the connector crates have a shared base — verify in first 5min via `ls crates/argo-mcp-connector-*/src/`; if no shared base crate exists, create `crates/argo-mcp-connector-shared/`):

```rust
pub struct RetryPolicy {
    pub max_attempts: u32,       // default 3
    pub initial_delay_ms: u64,   // default 250
    pub multiplier: f64,         // default 2.0
    pub max_delay_ms: u64,       // default 5000
    pub jitter_ms: u64,          // default 100 — uniform [0, jitter_ms)
}

pub async fn retry_with_policy<F, Fut, T, E>(
    policy: &RetryPolicy,
    is_retriable: fn(&E) -> bool,
    mut op: F,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>;
```

Retriable error classifiers per error type:
- HTTP: 5xx + 429 + connection-level errors are retriable; 4xx (other) is NOT.
- Tonic/gRPC: `Unavailable`, `DeadlineExceeded`, `ResourceExhausted` are retriable; others NOT.

Each connector wraps its outbound calls in `retry_with_policy(default_http_policy(), is_http_retriable, || self.call(...))`. Existing connector method signatures unchanged — retry is internal.

This is NOT in scope: idempotency-key wrapping (separate concern); per-tenant retry policy override; circuit-breaker integration (already shipped via `codex-circuit-breaker`); writing the wrappers for every existing connector (this brief delivers the primitive + wires ONE pilot connector, future connectors adopt).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-mcp-connector-shared/src/retry.rs` (NEW, ≤ 250 LOC) — `RetryPolicy`, `retry_with_policy`, classifier helpers. If `argo-mcp-connector-shared` crate doesn't exist, create it (Cargo.toml + src/lib.rs + src/retry.rs).
- `crates/argo-mcp-connector-shared/src/retry_tests.rs` (NEW, sibling) — 8 unit tests: succeeds-first-try; retries-on-retriable; gives-up-after-max-attempts; exponential-backoff-respected; jitter-applied; max-delay-capped; non-retriable-fails-immediately; concurrent-retries-no-collision.
- `crates/argo-mcp-connector-gcal/src/client.rs` (pilot wireup, verify in first 5min) — narrowly scoped: wrap outbound `reqwest` calls with `retry_with_policy(default_http_policy(), is_http_retriable, ...)`. NO change to public method signatures. NO change to other connectors (this brief is "pilot one + ship the primitive").
- `crates/argo-mcp-connector-gcal/tests/retry_smoke.rs` (NEW, ≤ 100 LOC) — integration smoke: mock 503 response twice then 200, assert connector returns 200 after retries.

## Files you must NOT touch

- `crates/argo-provider/src/circuit.rs` — circuit breaker is upstream
- Connectors other than gcal — adopting retry in each is a per-connector follow-up brief
- `crates/argo-store/migrations/**` — no schema change

## Acceptance criteria

- [ ] `RetryPolicy` struct + `Default` impl with documented defaults (3 attempts, 250ms base, 2.0x, 5000ms max, 100ms jitter)
- [ ] `retry_with_policy` calls op, classifies error via callback, sleeps `base * multiplier^attempt + jitter` capped at max_delay, retries until max_attempts
- [ ] HTTP classifier: 5xx + 429 + connection errors retriable; 4xx (other) NOT
- [ ] gcal connector wraps outbound calls; method signatures unchanged
- [ ] 8 unit tests + 1 integration smoke
- [ ] No new public API outside the listed exports
- [ ] `cargo build -p argo-mcp-connector-shared -p argo-mcp-connector-gcal` clean
- [ ] `cargo clippy -p argo-mcp-connector-shared -p argo-mcp-connector-gcal --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-mcp-connector-shared -p argo-mcp-connector-gcal` passes (skip pre-existing flakes + live-network tests gated by env var)
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-connector-auto-retry-backoff`
- PR title: `mcp-connector: shared retry-with-backoff + gcal pilot wireup (mission-connector-auto-retry-backoff)`

## Notes for the implementer

- Defaults are conservative — 3 attempts × 2x multiplier = max ~3.75s before giving up. Operators can override via `RetryPolicy::tightened()` / `loose()` if upstream is consistently slow.
- Jitter is critical for thundering-herd avoidance — without it, all retries land at the same offsets and amplify spikes.
- Pilot wireup on ONE connector (gcal). Don't try to wire all connectors here — that's per-connector micro-PRs.
- Use `tokio::time::sleep` for backoff. Don't use `std::thread::sleep` (blocks the runtime).
