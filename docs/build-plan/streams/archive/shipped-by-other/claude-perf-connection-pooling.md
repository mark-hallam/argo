# claude-perf-connection-pooling — Tune sqlx + reqwest pool sizing

**Owner**: claude. **Depends on**: none.

> Argo creates sqlx and reqwest pools with default sizing today. The
> defaults aren't bad, but they aren't documented or tunable, which
> means swarm-scale runs hit pool starvation silently. This brief
> documents current defaults, exposes env knobs for tuning, and adds
> small `pool_config.rs` modules to centralise the math.

## Why

Two pools matter for argo perf:
1. **sqlx (sqlite)**: a too-small pool serialises concurrent reads;
   too-large wastes file handles. The right number depends on
   parallel agent count and read/write mix.
2. **reqwest (provider HTTP)**: shared per-base-URL clients reuse
   TCP + TLS sessions. Without a configured pool, idle providers
   keep too many sockets; with too small a pool, swarm fan-out
   queues requests.

Without env knobs, ops can't tune either without recompiling.

## Scope

- **NEW** `crates/argo-store/src/pool_config.rs` (≤ 150 LOC):
  - `pub struct PoolConfig { max_connections, min_connections,
    acquire_timeout, idle_timeout }`
  - `pub fn from_env() -> PoolConfig` reading
    `ARGO_STORE_POOL_MAX`, `ARGO_STORE_POOL_MIN`,
    `ARGO_STORE_POOL_ACQUIRE_TIMEOUT_MS`,
    `ARGO_STORE_POOL_IDLE_TIMEOUT_S` with sensible defaults
  - documentation comment listing defaults + recommended values
- **NEW** `crates/argo-provider/src/pool_config.rs` (≤ 150 LOC):
  - `pub struct ProviderPoolConfig { pool_max_idle_per_host,
    pool_idle_timeout, connect_timeout, request_timeout }`
  - `pub fn from_env() -> ProviderPoolConfig` reading
    `ARGO_HTTP_POOL_MAX_IDLE_PER_HOST`,
    `ARGO_HTTP_POOL_IDLE_TIMEOUT_S`,
    `ARGO_HTTP_CONNECT_TIMEOUT_S`,
    `ARGO_HTTP_REQUEST_TIMEOUT_S`
- **Modify** sqlx `PoolOptions` builder site in
  `crates/argo-store/src/lib.rs` to read from `PoolConfig`
- **Modify** reqwest `Client::builder()` sites in
  `crates/argo-provider/src/transport.rs` (and any other
  client-construction sites — keep edits narrow) to read from
  `ProviderPoolConfig`

## Approach

1. **Audit** existing sqlx + reqwest client construction. Note
   today's effective defaults in the PR body.
2. Pick documented defaults that match observed swarm behaviour:
   - sqlx max=10, min=1, acquire_timeout=5s, idle_timeout=60s
   - reqwest pool_max_idle_per_host=8, pool_idle_timeout=90s,
     connect_timeout=10s, request_timeout=120s
   (Adjust based on what the audit shows.)
3. Wire env reads. Invalid env values fall back to default + emit a
   `tracing::warn!`.
4. Document each knob in the file's module-doc comment.

## Files you may touch

- `crates/argo-store/src/pool_config.rs` (NEW, ≤ 150 LOC)
- `crates/argo-store/src/pool_config_tests.rs` (NEW, ≤ 100 LOC) —
  3 tests: defaults, env-override, invalid-env-warns-and-defaults
- `crates/argo-store/src/lib.rs` — narrowly: read PoolConfig at
  pool construction
- `crates/argo-provider/src/pool_config.rs` (NEW, ≤ 150 LOC)
- `crates/argo-provider/src/pool_config_tests.rs` (NEW, ≤ 100 LOC)
- `crates/argo-provider/src/transport.rs` — narrowly: apply
  ProviderPoolConfig to reqwest builder

## Files you must NOT touch

- Provider-specific code (anthropic/, bedrock/, openai_*) — these
  consume the shared `transport` layer; no per-provider tuning here
- Schema / migrations
- Other crates' Cargo.tomls

## Acceptance criteria

- [ ] Both `pool_config.rs` modules with `from_env()` + documented
      defaults
- [ ] sqlx + reqwest construction reads from the new configs
- [ ] PR body lists: today's effective defaults, new defaults,
      and the env knobs
- [ ] 6 unit tests (3 per crate)
- [ ] No behavioural regression: existing tests pass unchanged
- [ ] Invalid env values warn + default; do not panic
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-store -p argo-provider` clean
- [ ] No file > 500 LOC

## Branch + PR

- Branch: `claude-perf-connection-pooling`
- PR title: `store+provider: documented pool defaults with env knobs (claude-perf-connection-pooling)`

## Notes for the implementer

- Don't try to **discover** the optimal numbers in this brief —
  document what's there now and expose the knobs. Tuning comes from
  ops + load testing later.
- Keep both `pool_config.rs` files structurally identical so a future
  refactor can extract a shared trait if it's worth it.
