# Per-route rate limit on gateway

**Owner**: claude. **Depends on**: none.

> SOURCE: production hardening Phase C. The codebase has a
> per-MCP-server rate limit (in `argo-mcp`), but the gateway's
> own routes have no per-route or per-tenant rate limit. A
> single noisy tenant can flood `session.create` and degrade
> service for others. This brief adds a token-bucket limiter
> per (route, tenant_id) at the gateway dispatch boundary.

## Scope

Add a token-bucket rate limiter to the gateway. Configuration:

- Per-route default: 100 requests / minute / tenant (i.e. each
  tenant has its own bucket per route).
- Hot routes get tighter defaults: `forge.run` 10/min/tenant,
  `skill.execute` 30/min/tenant.
- Globals are tunable via env: `ARGO_RATE_LIMIT_DEFAULT_RPM=100`,
  per-route overrides via `ARGO_RATE_LIMIT_<ROUTE>_RPM=...`.

Implementation: in-memory `DashMap<(Route, TenantId), Bucket>`
with a background sweep task that evicts buckets idle > 10
minutes. No persistence — this is operational backpressure, not
business state. Restart resets buckets (acceptable: it's a
load-shedding mechanism, not a quota).

When a bucket is empty, return HTTP 429 with body
`{"error": "rate_limited", "retry_after_ms": <n>}` and set the
`Retry-After` header.

This is **separate** from `claude-tenant-quota-enforcement-stricter`:
quota is monthly-ish business limits; rate limit is per-minute
load shedding. Both can return 429 — distinguish by the `error`
field in the body.

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/rate_limit.rs` (NEW,
  ~180 LOC) — `RateLimiter` struct, `Bucket`, `acquire(route, tenant_id)`
  returning `Result<(), RetryAfter>`, the eviction sweep task
- `crates/argo-cli/src/handlers/gateway/mod.rs` — narrowly:
  declare module + spawn the sweep task during gateway init
- `crates/argo-cli/src/handlers/gateway/dispatch_route.rs` —
  narrowly: a single call to `rate_limiter.acquire(...)?` at
  the top of dispatch (before the action runs and before the
  quota gate from the other brief, if both land)
- `crates/argo-cli/src/handlers/gateway/error_response.rs` (or
  equivalent) — narrowly: map `RetryAfter` to 429 with the
  documented body + header
- `crates/argo-cli/src/handlers/gateway/rate_limit_tests.rs`
  (NEW) — 6 tests: bucket fills + drains, refill rate is
  correct, eviction removes idle buckets, env override works,
  unknown route falls back to default, concurrent acquires are
  thread-safe

## Files you must NOT touch

- `argo-mcp`'s per-MCP-server rate limiter — different
  concern, different layer.
- Quota enforcement files — separate brief.
- Routes themselves — no per-handler changes; the gate is in
  `dispatch_route`.

## Acceptance criteria

- [ ] `RateLimiter::acquire` is correctly thread-safe under
      DashMap (test with 100 concurrent acquires)
- [ ] Default 100 rpm/tenant; `forge.run` 10; `skill.execute` 30
- [ ] Env overrides honored
- [ ] Sweep task evicts idle buckets > 10 min after no activity
- [ ] 429 response with body + `Retry-After` header
- [ ] `error: rate_limited` distinguishable from
      `error: quota_exceeded` from the other brief
- [ ] 6 unit tests + 1 integration test (issue 11 requests
      back-to-back to a 10/min route, observe 1 reject)
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-rate-limit-per-route`
- PR title: `gateway: per-route token-bucket rate limit (claude-rate-limit-per-route)`

## Notes for the implementer

- Token bucket math: `tokens += elapsed_secs * (rpm / 60.0)`,
  capped at the bucket's `max_tokens` (set max = rpm so a
  cold-start tenant can burst its full rpm).
- Use `tokio::time::Instant` for elapsed; do NOT use system
  time (clock changes break the math).
- The eviction sweep should run on a `tokio::time::interval(60s)`
  and remove entries where `last_seen < now - 10min`.
- DO NOT introduce a "burst" config knob in this brief — keep
  the surface tight; max_tokens = rpm is fine for v1.
