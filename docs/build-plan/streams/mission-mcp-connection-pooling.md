# Stream mission-mcp-connection-pooling — pooled MCP client (no per-turn spawn)

**Owner**: claude. **Depends on**: none.
**Authored against**: origin/main `8fe7aeec` on `2026-05-13`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-mcp-connection-pooling`.

> SOURCE: MISSION_BACKLOG.md Section 8 #252 ("Connection pooling for MCP connectors (currently per-turn)"). Today `McpClient::new(...)` in `crates/argo-mcp/src/client.rs` `tokio::process::Command::spawn()`s a fresh subprocess every call. Heavy connectors (OAuth-refreshing or session-warming ones like Gmail, GCal, Slack) pay 200-1500ms cold-start per turn. A turn that calls 3 different MCP tools pays 3 cold-starts.

## Scope

Introduce an `McpClientPool` that owns long-lived `McpClient` instances keyed by `(tenant_id, manifest_id)` and hands them out to callers via an `acquire(manifest) -> PoolGuard<McpClient>` borrow. On guard drop the client is returned to the pool, NOT torn down. The subprocess stays warm until pool eviction (idle TTL or capacity pressure).

Eviction: per-pool LRU with a config-driven `max_clients` (default 32) and `idle_ttl_secs` (default 600 = 10 min — long enough to survive turn-to-turn gaps, short enough that idle workers free FDs).

The dispatch sites that currently call `McpClient::new(...)` switch to `pool.acquire(manifest).await`. The pool itself is constructed once at AgentDeps wiring and shared via `Arc<McpClientPool>`.

**Not in scope**: persistent pool state across process restart; cross-process pooling (each binary instance gets its own pool); MCP-over-HTTP pooling (this brief is stdio-only); priority queueing or fair scheduling.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `<NEW: crates/argo-mcp/src/pool.rs>` — the pool (≤400 LOC). `McpClientPool` holds `Mutex<LruCache<PoolKey, Arc<McpClient>>>`. Public surface: `new(config) -> Self`, `acquire(&self, manifest_id) -> ArgoResult<PoolGuard>`, `evict_idle()` (called from a cron tick), `stats() -> PoolStats`.
- `<NEW: crates/argo-mcp/src/pool_tests.rs>` — unit tests (≤300 LOC). Covers: acquire→drop→re-acquire returns same instance; LRU eviction on `max_clients+1`; idle TTL expiry; per-tenant isolation.
- `crates/argo-mcp/src/lib.rs` — narrowly scoped: add ONE `pub mod pool;` line + cfg-test mod line. Re-export `McpClientPool, PoolConfig, PoolGuard, PoolStats`.
- `crates/argo-mcp/src/client.rs` — narrowly scoped: make `McpClient::shutdown()` idempotent (currently panics if called twice); the pool may call shutdown on eviction. ≤20 modified lines.
- `crates/argo-config/src/sections_runtime.rs` — narrowly scoped: add ONE `pub struct McpPoolConfig { pub max_clients: u32, pub idle_ttl_secs: u64 }` with defaults `32, 600`. Add ONE field on `RuntimeConfig`.
- One dispatch site (search `McpClient::new` in `crates/argo-tool/src/` or `crates/argo-runtime/src/`) — narrowly scoped: swap direct construction for `pool.acquire(...)`. If multiple dispatch sites exist, switch them all but keep the diff narrow per site.
- `Cargo.toml` — narrowly scoped: add ONE workspace dep on `lru = "0.12"` to `argo-mcp/Cargo.toml`. No other edits.

## Files you must NOT touch

- `crates/argo-mcp/src/protocol.rs` — protocol layer is invariant.
- `crates/argo-mcp/src/server.rs` — server side is unrelated.
- Any `argo-mcp-connector-*` crate — connectors are unaware of pooling; they implement `McpClient` clients identically. No connector changes.
- `Cargo.lock` (auto-updated only by cargo when adding the lru dep).

If your task genuinely requires a file not listed, **stop and mark this brief BLOCKED in BUILD_STATUS.md** with a one-line reason.

## Acceptance criteria

- [ ] `McpClientPool::new(config)` constructs an empty pool; `acquire` lazily spawns + caches the client.
- [ ] Sequential `acquire` of the same `manifest_id` on the same tenant returns a guard backed by the same `McpClient` instance (verify via process pid / connection counter).
- [ ] LRU eviction: pool with `max_clients=2`, three distinct manifests acquired in sequence, the oldest is shut down and its subprocess reaped.
- [ ] Idle TTL: a client acquired then left idle past `idle_ttl_secs` is removed on the next `evict_idle()` tick. Subprocess reaped, no zombie.
- [ ] Per-tenant isolation: `(tenant_a, manifest_x)` and `(tenant_b, manifest_x)` map to distinct `McpClient` instances.
- [ ] PoolGuard `Drop` returns the client to the pool without shutdown.
- [ ] `pool.stats()` reports `entries`, `idle_seconds_p50`, `idle_seconds_max`. Used by future operator-tooling.
- [ ] `cargo clippy -p argo-mcp --all-targets -- -D warnings` clean.
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean — claim explicitly in PR body.
- [ ] `cargo test -p argo-mcp` passes.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `claude-mission-mcp-connection-pooling`
- PR title: `mcp: connection pool for MCP clients (mission-mcp-connection-pooling)`

## Notes for the implementer

- The `lru` crate's `LruCache` is fine for v1; don't reach for a fancy concurrent-LRU yet. The mutex is unlikely to be the bottleneck.
- `Arc<McpClient>` allows the pool to evict while a guard is still outstanding — the dropped guard will simply not be re-poolable (it'll get reaped on drop). That's the simplest correct semantic.
- The `evict_idle()` cron tick lives outside this brief — wire it up in a follow-up (`mission-mcp-pool-evict-cron`). For this PR, only the on-acquire LRU eviction is required.
- Shutdown idempotence in `client.rs`: today the `Drop` impl `kill()`s the child; calling it again on a reaped client panics. Add a `shut: AtomicBool` (or equivalent) and early-return on second call.
- Per-tenant isolation is the **only correctness invariant**. Cross-tenant collision would leak credentials across tenants — tests must exercise this case explicitly.
- Do not introduce a global static pool. Wire through `AgentDeps` per CLAUDE.md's "AgentDeps is the only injection point" invariant.
