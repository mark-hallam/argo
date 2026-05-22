# claude-mcp-server-health-check — periodic health pings to registered MCP servers; persist last_seen + last_error

**Owner**: claude. **Depends on**: `codex-mcp-server-registry-cli` (this brief writes to the registry's `last_seen` / `last_error` columns) — soft dep; can land first if it falls back to the static manifest.

## Why

Operators currently learn an MCP server is dead only when an action against it fails. The registry (cf. `codex-mcp-server-registry-cli`) has columns for `last_seen` and `last_error` but nothing populates them between action invocations. This brief adds a background health-ping task that proactively probes every registered server every N seconds and records observed health, so the registry list view shows true status.

## Scope

1. **NEW** `crates/argo-cli/src/handlers/mcp_health.rs` (~290 LOC):
   - `HealthCheckTask` struct holding `tick_interval: Duration`, `pool: Arc<McpPool>`, `registry: Arc<dyn McpRegistry>`
   - `run(shutdown: CancellationToken) -> ArgoResult<()>` — async loop that ticks every `tick_interval`, fans out one health ping per registered server (bounded `tokio::task::JoinSet`), updates `last_seen` on success and `last_error` on failure
   - Health ping: lightweight `list_tools` call with a 5s timeout (configurable). MCP doesn't define a dedicated `ping` verb so `list_tools` is the cheapest known-supported probe.
   - Concurrency cap: `max_concurrent: usize` (default 8) so we don't blow up on a tenant with 200 servers
   - Backoff: per-server exponential backoff after consecutive failures (capped at 5 minutes between retries) to avoid hammering a wedged endpoint
2. **Background task spawn** — narrow modify the gateway boot path (existing pattern; see `mission-otlp-exporter` shape) to spawn `HealthCheckTask::run` if `ARGO_MCP_HEALTH_CHECK=1` is set (default off until operators opt in)
3. **Surface** — `argo mcp registry list` already shows `last_seen` (per `codex-mcp-server-registry-cli`); this brief just populates it. No new CLI subcommand.
4. **Telemetry** — emit `mcp.health` events with `{server_id, status, latency_ms, error?}` so dashboards can build alerting on top

## Files you may touch

- `crates/argo-cli/src/handlers/mcp_health.rs` (NEW, ≤ 340 LOC)
- `crates/argo-cli/src/handlers/mcp_health_tests.rs` (NEW, ≤ 280 LOC sibling) — 7 unit tests:
  - tick fans out one ping per registered server
  - successful ping updates `last_seen`
  - failed ping updates `last_error`
  - exponential backoff after consecutive failures
  - max_concurrent honored (assert via channel-based fixture)
  - tenant isolation (no cross-tenant pings)
  - graceful shutdown via cancellation token
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 1 `pub mod` line
- `crates/argo-cli/src/gateway/boot.rs` (or equivalent boot file) — narrowly: spawn `HealthCheckTask::run` behind feature flag, ~20 LOC delta
- `crates/argo-cli/tests/mcp_health_smoke.rs` (NEW, ≤ 200 LOC) — 1 integration test seeding 3 fake servers (one fast, one slow, one dead), running 2 ticks, asserting registry state
- `manifests/connectors/mcp.toml` or new `manifests/connectors/mcp_health.toml` — narrowly: document tunables (tick interval, concurrency cap, timeout)

## Files you must NOT touch

- `argo-mcp-connector-*` crates
- `worker_actions_executor_mcp.rs` — only consume the connection pool via its existing public surface
- `argo-vertical/src/manifest.rs`
- `argo-store` migrations (registry schema is owned by `codex-mcp-server-registry-cli`)

## Acceptance criteria

- [ ] `HealthCheckTask::run` ticks every N seconds and pings every registered server
- [ ] Successful ping updates `last_seen`; failed ping updates `last_error`
- [ ] Exponential backoff caps at 5min for repeatedly-failing servers
- [ ] `max_concurrent` cap enforced
- [ ] Graceful shutdown via cancellation token (no orphaned tasks on SIGTERM)
- [ ] Telemetry event `mcp.health` fires per ping
- [ ] Feature flag `ARGO_MCP_HEALTH_CHECK=1` gates the spawn (off by default)
- [ ] 7 unit tests + 1 integration smoke
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli` clean
- [ ] Every file < 500 LOC

## Branch + PR

- Branch: `claude-mcp-server-health-check`
- PR title: `mcp: periodic health-ping task populates registry last_seen/last_error (claude-mcp-server-health-check)`

## Notes for the implementer

- Use `tokio_util::sync::CancellationToken` (existing workspace pattern) for shutdown — do not invent a new shutdown channel.
- Backoff state is in-memory (per-task lifetime); do not persist it to the store. On gateway restart, backoff resets — that's acceptable.
- If `codex-mcp-server-registry-cli` hasn't merged yet, abstract the registry behind a `McpRegistry` trait with two impls: `SqlMcpRegistry` (SKIP — depends on the other brief) and `StaticManifestMcpRegistry` (fallback that just enumerates the static `manifests/connectors/mcp.toml` entries; updates are no-ops). This keeps the brief unblocked.
- Fan-out via `tokio::task::JoinSet` and a `tokio::sync::Semaphore` to enforce `max_concurrent`.
- The `list_tools` ping is the cheapest probe today. If MCP introduces a dedicated `ping` verb later, swap it in via a one-line change.
