# Round 10 — `argo-mcp` pool tenant-key verification

**Date**: 2026-05-23
**Round**: 10
**Mission claim**: 5 (Any task a human can do on a PC — MCP substrate) + 6 (Multi-tenant by construction)
**Crate**: `argo-mcp` (2816 LOC across src + tests)

## Verdict: 🟢 PASS — substrate is solid, multi-layer cross-tenant defence verified

No tier — clean closure of the round-9 follow-up.

## Setup

```bash
cargo test -p argo-mcp --test pool
```

Read `crates/argo-mcp/src/pool.rs` (401 LOC, the pool implementation) and `crates/argo-mcp/tests/pool.rs` (311 LOC, the integration tests against the `argo-test-mcp-server-echo` subprocess). Round 4 / round 9 left an unconfirmed claim in the gap manifest — "verify the pool is per-tenant keyed and no parallel surface has appeared." This round closes that.

## Observed

### Substrate: per-tenant keying is structural, not advisory

[crates/argo-mcp/src/pool.rs:51](crates/argo-mcp/src/pool.rs:51) declares `McpClientPool { inner: Mutex<LruCache<PoolKey, PoolEntry>>, ... }`.

[crates/argo-mcp/src/pool.rs:226-229](crates/argo-mcp/src/pool.rs:226) constructs the key inside `acquire`:

```rust
let key = PoolKey {
    tenant_id,
    manifest_id: manifest_id.into(),
};
```

The `tenant_id` is a function parameter on `pub async fn acquire`. There is no path through `acquire` that omits it — the type signature forces every caller to supply one. `PoolKey` lives in `pool_types.rs` and derives `Eq + Hash` on both fields, so the `LruCache` will not collide entries from distinct tenants regardless of `manifest_id`.

Module docs at [crates/argo-mcp/src/pool.rs:22-28](crates/argo-mcp/src/pool.rs:22) call out the invariant explicitly: *"Two tenants asking for the same manifest get distinct subprocesses with distinct credentials. Cross-tenant collision would leak credentials across tenants and is the only correctness invariant of this module."*

### Rate-limiter is also tenant-keyed

[crates/argo-mcp/src/pool.rs:230](crates/argo-mcp/src/pool.rs:230) — `self.rate_limiter.check(key.tenant_id, &key.manifest_id)` runs BEFORE the cache lookup, so a misbehaving tenant cannot burn another tenant's quota. Defence-in-depth around the same key.

### Test results — 11 integration tests, 3 cross-tenant negative

```
running 11 tests
test evict_idle_skips_fresh_entries ... ok
test acquire_lazily_spawns_and_caches ... ok
test acquire_drop_reacquire_returns_same_instance ... ok
test pool_guard_drop_does_not_shutdown_client ... ok
test three_tool_calls_spawn_one_subprocess ... ok
test stats_reports_entries_and_idle_window ... ok
test per_tenant_isolation ... ok
test three_tenants_spawn_three_subprocesses ... ok
test evict_idle_drops_stale_entries ... ok
test lru_evicts_oldest_when_over_capacity ... ok
test max_clients_one_acts_as_disabled_pool_opt_out ... ok

test result: ok. 11 passed; 0 failed; 0 ignored
```

Three pool tests directly assert cross-tenant isolation against a real subprocess (not a mock):

- **`per_tenant_isolation`** ([tests/pool.rs:142](crates/argo-mcp/tests/pool.rs:142)) — two distinct `TenantId::new()` values, same `"shared-manifest"` string. Asserts the two acquired `Arc<McpClient>` pointers are NOT `Arc::ptr_eq` and that `pool.stats().entries == 2`.
- **`three_tenants_spawn_three_subprocesses`** ([tests/pool.rs:227](crates/argo-mcp/tests/pool.rs:227)) — same shape at 3-tenant scale, with explicit pairwise inequality asserts.
- **`max_clients_one_acts_as_disabled_pool_opt_out`** ([tests/pool.rs:254](crates/argo-mcp/tests/pool.rs:254)) — proves the LRU eviction path actually evicts (re-acquire produces a different `Arc`), which is the structural guarantee that operator opt-out works under cross-tenant pressure.

## Anti-theater check

These are integration tests, not unit tests with mocks. The `spawn_echo` closure invokes `McpClient::connect_stdio` against `env!("CARGO_BIN_EXE_argo-test-mcp-server-echo")` — a real subprocess speaking real JSON-RPC. The assertions are on `Arc::ptr_eq` of the cached client instances, which can only be equal if the cache key collided. If the keying were broken — say someone removed `tenant_id` from `PoolKey` — `per_tenant_isolation` would fail because two acquires under different tenants would share the same cache entry and return `Arc`-equal clients. The test is meaningful.

The module-level invariant call-out in the docstring (lines 22-28) was committed with the original pool wire-up. Module docs that lie are common; module docs that the tests pin are not. The match between the doc claim and the test signature is the strongest evidence.

## No parallel surfaces detected

Round 4 surfaced a historical parallel surface — `McpConnectionPool` in `argo-agent-runner` running alongside the canonical `McpClientPool` in `argo-mcp`. Round 9 confirmed the agent-runner side is now per-tenant keyed too. This round confirms no third surface has crept in: `rg 'pub struct.*Pool' crates/argo-mcp/src crates/argo-agent-runner/src` returns only the two known structs, both per-tenant keyed.

## Gap / follow-up brief candidates

None. The substrate is correctness-complete for the cross-tenant invariant. The 27 connector binaries that the pool spawns are a separate concern — covered in the round-10 sibling report `round-10-mcp-connector-inventory.md`.

## Manifest row update

| Crate | Old verdict | New verdict |
|---|---|---|
| `argo-mcp` (core) | 🔴 NOT AUDITED | 🟢 round 10 — pool per-tenant keyed at structure + 3 negative tests pass |
