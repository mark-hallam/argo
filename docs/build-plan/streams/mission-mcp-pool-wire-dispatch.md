# Stream mission-mcp-pool-wire-dispatch — wire #1540 McpClientPool into the 5 dispatch sites

**Owner**: claude. **Depends on**: mission-mcp-connection-pooling merged (#1540, after chain-rebase).
**Authored against**: origin/main `173e24dc` on `2026-05-13`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-mcp-pool-wire-dispatch`.

> SOURCE: deferred from #1540 PR-body "Dispatch-site swap deferred" section. The pool is in `argo_mcp::pool::McpClientPool` but the 5 sites that today spawn cold subprocesses still call `McpClient::connect_stdio` directly. Until those swap, the pool is dead code in production.

## Scope

The MERGED #1540 brief shipped the `McpClientPool` primitive but punted dispatch-site integration because the original brief's may-touch list cited `argo-tool` and `argo-runtime` (neither of which calls `McpClient`). The actual call sites are:

| File | Line | Call | Context |
|---|---|---|---|
| `crates/argo-cli/src/handlers/worker_actions_executor_browser.rs` | 194 | `McpClient::connect_stdio(&program, &args)` | browser-action MCP dispatch |
| `crates/argo-cli/src/handlers/worker_actions_executor_mcp.rs` | 251 | `McpClient::connect_stdio(&server_decl.command, &server_decl.args)` | generic MCP-tool dispatch |
| `crates/argo-cli/src/handlers/mcp_tools.rs` | 131 | `McpClient::connect_stdio(&decl.command, &decl.args)` | `argo mcp tools` discovery |
| `crates/argo-cli/src/handlers/mcp_health.rs` | 213 | `McpClient::connect_stdio_with_timeout(...)` | `argo mcp health` (KEEP — health check should NOT reuse warm clients; the whole point is to test cold-start) |
| `crates/argo-agent-runner/src/dispatch.rs` | 29 | `McpClient::connect_stdio(command, &args)` | agent-runner outer dispatch |

For each integration site (4 of the 5; mcp_health stays cold-by-design), swap the direct construction for:

```rust
let guard = pool.acquire(tenant_id, manifest_id, || async {
    McpClient::connect_stdio(&program, &args).await
}).await?;
let client: &McpClient = &*guard;
// ... existing call_tool / list_tools logic against &client ...
// guard drops here -> client returned to pool, NOT torn down
```

Plumb `Arc<McpClientPool>` through `AgentDeps` (per CLAUDE.md "AgentDeps is the only injection point"). Build at startup from `RuntimeConfig::mcp_pool` (the `McpPoolConfig` already shipped in #1540).

**Disabled-pool path**: if operators want to opt out, support `RuntimeConfig::mcp_pool.max_clients = 0`; the pool implementation already clamps to ≥1 internally so we need a separate `enabled: bool` knob OR document that `max_clients = 1` is the practical opt-out (every acquire evicts the previous). Pick the cleanest of the two and document.

**Acceptance gate**: after wire-up, a 3-tool turn against the same MCP manifest spawns 1 subprocess (not 3); a 3-tenant turn against the same manifest spawns 3 distinct subprocesses (per-tenant isolation invariant).

**Not in scope**: an idle-tick cron driver (the `evict_idle()` method exists but needs a periodic caller — that's `mission-mcp-pool-evict-cron`, follow-up brief). HTTP-MCP pooling (this brief is stdio-only). Health-check call site stays cold by design.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-cli/src/handlers/worker_actions_executor_browser.rs` — narrowly scoped: line 194 + surrounding 10-15 lines. Convert to pool.acquire pattern. Keep the surrounding tool-discovery + call_tool logic.
- `crates/argo-cli/src/handlers/worker_actions_executor_mcp.rs` — narrowly scoped: line 251 + 10-15 lines. Same conversion.
- `crates/argo-cli/src/handlers/mcp_tools.rs` — narrowly scoped: line 131 + 10-15 lines. Same conversion.
- `crates/argo-agent-runner/src/dispatch.rs` — narrowly scoped: line 29 + 10-15 lines. Same conversion.
- `crates/argo-orchestrator/src/deps.rs` (or wherever AgentDeps lives — confirm via `rg -l 'struct AgentDeps' crates/`) — narrowly scoped: add ONE `pub mcp_pool: Arc<McpClientPool>` field; construct in `new()` from `RuntimeConfig::mcp_pool`.
- `<NEW: crates/argo-mcp/tests/pool_integration.rs>` or extension of `crates/argo-mcp/tests/pool.rs` — add the 2 acceptance smoke tests (1-subprocess-per-3-tools, 3-subprocesses-per-3-tenants).

## Files you must NOT touch

- `crates/argo-mcp/src/pool.rs` — primitive is locked from #1540.
- `crates/argo-mcp/src/client.rs` — `connect_stdio` API is the underlying spawn primitive; the pool wraps it.
- `crates/argo-cli/src/handlers/mcp_health.rs` — health check must stay cold (one subprocess per probe). Leave it alone.
- Any MCP-connector crate — connectors are unaware of pooling; nothing changes there.

If your task genuinely requires a file not listed, **stop and mark this brief BLOCKED in BUILD_STATUS.md** with a one-line reason.

## Acceptance criteria

- [ ] All 4 in-scope dispatch sites use `pool.acquire(tenant, manifest, || ...)` instead of direct `McpClient::connect_stdio(...)`.
- [ ] `mcp_health.rs` line 213 is unchanged (cold-by-design).
- [ ] `AgentDeps` carries `mcp_pool: Arc<McpClientPool>`; constructed at startup from `RuntimeConfig::mcp_pool`.
- [ ] Per-tenant isolation smoke: a 3-tenant turn against the same `manifest_id` spawns 3 distinct subprocesses. Assert via `pool.stats().entries == 3`.
- [ ] Warm-reuse smoke: a 3-tool turn for one `(tenant, manifest)` pair spawns 1 subprocess. Assert via `pool.stats().entries == 1` after.
- [ ] Disabled-pool path (`max_clients = 1` or explicit `enabled: false` knob — pick one and document) recovers pre-brief behavior.
- [ ] `cargo clippy -p argo-cli -p argo-agent-runner -p argo-mcp --all-targets -- -D warnings` clean.
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean — claim explicitly in PR body.
- [ ] `cargo test -p argo-mcp` passes.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-mission-mcp-pool-wire-dispatch`
- PR title: `mcp: wire warm-pool into 4 dispatch sites (mission-mcp-pool-wire-dispatch)`

## Notes for the implementer

- The pool's `acquire` takes a spawn closure that returns the new `McpClient` on cache miss. Keep the closure tiny — pull `program` / `args` / `env` from the existing call-site logic into the closure verbatim.
- `manifest_id` should be a stable string. For browser + mcp + agent-runner dispatch, the manifest entry's `id` field is the natural choice. For `mcp_tools` discovery, use the same `decl.id` if available.
- The `Arc<McpClientPool>` lives in `AgentDeps`; each dispatch site already has access to `AgentDeps` via its existing parameter threading — no new top-level plumbing.
- Tests should drive `argo-test-mcp-server-echo` (the existing test fixture) — the integration test pattern is already established in `crates/argo-mcp/tests/pool.rs` from #1540.
- This brief is the operator-visible value of #252. Without this wire-up the #1540 pool is dead code.
