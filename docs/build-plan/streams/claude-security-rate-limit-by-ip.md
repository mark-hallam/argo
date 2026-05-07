# Mission Phase 2 — Per-IP rate limiting (token bucket)

**Owner**: claude. **Depends on**: none (independent — layers alongside existing
per-route rate limit, does not replace it).

## Scope

Argo's gateway has per-route rate limiting today. That defends against a single
abusive endpoint, but a single source IP can still spread traffic across many
endpoints to evade the cap. This brief adds a *per-IP* token bucket
middleware that limits any single source IP to N requests per window across
ALL routes — independent of the per-route cap.

Token bucket: each IP gets a bucket with capacity `IP_BUCKET_CAPACITY`
(default 120) and refill rate `IP_BUCKET_REFILL_PER_SEC` (default 2). Bucket
empty → 429 with `Retry-After`. Buckets live in an in-memory `DashMap` with
LRU eviction (cap entries to ~100k, evict oldest on insert when full).

## Approach (concrete)

1. **NEW** `crates/argo-cli/src/handlers/gateway/rate_limit_ip.rs` (~280 LOC):
   - `pub struct IpRateLimiter { buckets: Arc<DashMap<IpAddr, Bucket>>, cap: u32, refill: f64 }`
   - `struct Bucket { tokens: f64, last_refill: Instant }` — `take(&mut self, now: Instant) -> bool`
     refills lazily and tries to consume 1 token
   - `pub async fn ip_rate_limit_middleware(...) -> Response` axum middleware:
     - Extracts source IP: prefer `x-forwarded-for` first hop (when behind LB,
       configurable via `ARGO_TRUST_XFF=1`), else `ConnectInfo::<SocketAddr>`
     - Calls `limiter.check(ip)`; on deny → `429` with
       `Retry-After: <seconds>` (computed from refill rate)
   - LRU eviction: when `buckets.len() > IP_BUCKET_MAX_ENTRIES` (100k), evict
     entries with `last_refill` older than `IP_BUCKET_EVICT_AGE` (5 min)
   - Env knobs:
     - `ARGO_IP_RATE_LIMIT=0` disables (default on)
     - `ARGO_IP_RATE_CAPACITY=<n>` overrides capacity
     - `ARGO_IP_RATE_REFILL=<f>` overrides refill rate (tokens/sec)
     - `ARGO_TRUST_XFF=1` honours `x-forwarded-for` first hop
2. **Modify** `crates/argo-cli/src/handlers/gateway/mod.rs`:
   - Layer the middleware via `axum::middleware::from_fn_with_state(...)`
   - Layer order: `ip_rate_limit` runs BEFORE per-route rate limit (cheap
     reject before route work)
3. **Metric hook**:
   - Increment a `tracing::warn!` (rate-limited per-IP) counter on each 429
     so ops can spot abuse

## Manifest contract / wire format

No wire change. 429 responses include `Retry-After` per RFC 7231 §7.1.3.
Body is a small JSON: `{"error":"rate_limited","scope":"ip","retry_after":<n>}`.

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/rate_limit_ip.rs` (NEW, ≤ 320 LOC)
- `crates/argo-cli/src/handlers/gateway/rate_limit_ip_tests.rs`
  (NEW, ≤ 250 LOC) — 7 unit tests:
  - first request from new IP → allow
  - burst up to capacity → allow
  - exceed capacity → 429 with Retry-After
  - refill after wait → allow again
  - different IPs have separate buckets
  - LRU evicts old entries when over cap
  - `ARGO_IP_RATE_LIMIT=0` short-circuits (always allow)
- `crates/argo-cli/src/handlers/gateway/mod.rs` — narrowly: 1 layer line +
  1 pub mod line
- `crates/argo-cli/tests/gateway_ip_rate_limit_smoke.rs` (NEW, ≤ 200 LOC) —
  1 integration test that hammers a route from a single IP and observes 429
- `crates/argo-cli/Cargo.toml` — narrowly: add `dashmap` (verify whether it
  is already a workspace dep first)

## Files you must NOT touch

- Existing per-route rate limit code — this brief is additive
- Bearer auth (#946) — orthogonal
- `argo-types` — `IpRateLimiter` lives in the gateway handler module

## Acceptance criteria

- [ ] Per-IP token bucket enforces capacity + refill correctly
- [ ] 429 includes `Retry-After` header
- [ ] LRU eviction caps memory at ~100k entries
- [ ] `ARGO_TRUST_XFF=1` honours `x-forwarded-for` first hop
- [ ] `ARGO_IP_RATE_LIMIT=0` disables the layer cleanly
- [ ] Layered BEFORE per-route limit (cheaper reject path)
- [ ] 7 unit tests + 1 integration smoke
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-security-rate-limit-by-ip`
- PR title: `gateway: per-IP token-bucket rate limit (claude-security-rate-limit-by-ip)`

## Notes for the implementer

- `ConnectInfo<SocketAddr>` is the axum source-address extractor. Make sure
  the gateway server is bound with `into_make_service_with_connect_info` —
  if not already, this brief widens the scope; coordinate with the conductor
  before changing server bind.
- `x-forwarded-for` parsing: trust ONLY the first hop; rejecting `X-F-F` from
  untrusted callers is the default (`ARGO_TRUST_XFF=0`). Document the trust
  model in the PR description.
- DashMap is preferable to `Mutex<HashMap>` for hot-path concurrency. If not
  already a workspace dep, add it; otherwise reuse.
- Token-bucket math is simple but easy to off-by-one on refill. Write the
  refill test first (TDD on the refill arithmetic).
