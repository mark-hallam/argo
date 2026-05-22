# codex-mcp-server-registry-cli — `argo mcp registry list/add/remove` per-tenant MCP server registry

**Owner**: codex. **Depends on**: `mission-wire-mcp-action-executor` (#933) merged — that brief introduced the per-server connection pool but left server registration ad-hoc (read from a static manifest at boot).

## Why

Today every tenant ships with the same set of MCP server connection strings declared in `manifests/connectors/mcp.toml`. To onboard a tenant-specific MCP server (their internal Jira, their Snowflake MCP bridge) the operator hand-edits the manifest and redeploys. That doesn't scale and it conflates tenant config with system config.

This brief introduces a **per-tenant MCP server registry** stored in `argo-store`, with a CLI surface for `list / add / remove`. The dispatch layer (#933) reads from the registry at request time instead of the static manifest.

## Scope

1. **NEW** `crates/argo-cli/src/handlers/mcp_registry.rs` (~340 LOC):
   - `cmd_list(tenant_id, opts) -> ArgoResult<()>` — markdown / json table of `{server_id, kind, base_url, status, last_seen}`
   - `cmd_add(tenant_id, server_id, kind, base_url, secrets_ref) -> ArgoResult<()>` — inserts a row; validates URL syntax; pings server (best effort) and records initial `last_seen`
   - `cmd_remove(tenant_id, server_id, force: bool) -> ArgoResult<()>` — removes a row; refuses to remove a server with active actions unless `--force`
   - All three honor existing `--format json` / markdown family conventions
2. **Schema migration** — NEW `crates/argo-store/migrations/<n>_mcp_registry.sql`:
   - Table `mcp_server_registry { tenant_id, server_id, kind, base_url, secrets_ref, registered_at, last_seen, last_error }`
   - PK `(tenant_id, server_id)`, index on `(tenant_id, last_seen)`
3. **CLI plumbing** — add `mcp registry {list,add,remove}` clap subcommand tree under `mcp` (existing top-level)
4. **Wire the dispatch path (narrow)** — `worker_actions_executor_mcp.rs` (#933) currently looks up server connection by `server_id` against a static map. Add a thin `MpcRegistryLookup` trait + the SQL-backed impl behind a feature flag so the change is risk-bounded; default keeps the static-manifest path until ops flip the flag
5. **Help-smoke** — pin every clap variant

## Files you may touch

- `crates/argo-cli/src/handlers/mcp_registry.rs` (NEW, ≤ 380 LOC)
- `crates/argo-cli/src/handlers/mcp_registry_tests.rs` (NEW, ≤ 280 LOC sibling) — 8 unit tests:
  - add inserts row + records `registered_at`
  - add rejects invalid URL
  - list returns rows for the right tenant only
  - list markdown shape matches markdown-format family
  - remove deletes; refuses with active actions; `--force` overrides
  - remove returns NotFound on missing server_id
  - tenant isolation
  - duplicate server_id → conflict error
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 1 `pub mod` line
- `crates/argo-store/migrations/<next>_mcp_registry.sql` (NEW)
- `crates/argo-store/src/queries/mcp_registry.rs` (NEW, ≤ 300 LOC) — typed query helpers
- `crates/argo-store/src/queries/mod.rs` — narrowly: 1 `pub mod` line
- `crates/argo-cli/src/commands.rs` — narrowly: add `mcp registry` clap variants
- `crates/argo-cli/src/handlers/worker_actions_executor_mcp.rs` — narrowly: lookup-trait seam + feature-flag gated SQL impl, ~40 LOC delta (DO NOT exceed 500 LOC ceiling — extract helpers if needed)
- `crates/argo-cli/tests/mcp_registry_smoke.rs` (NEW, ≤ 220 LOC) — 1 integration test exercising add → list → remove
- `crates/argo-cli/tests/help_smoke.rs` — narrowly: 3 rows pinning new variants

## Files you must NOT touch

- `argo-mcp-connector-*` crates — registry stores connection metadata; doesn't touch protocol
- `argo-vertical/src/manifest.rs`
- The `Executor` trait
- Other migrations

## Acceptance criteria

- [ ] `argo mcp registry add <server_id> --kind=<k> --url=<u>` inserts a row and pings the server
- [ ] `argo mcp registry list` returns markdown table by default; `--format json` returns NDJSON
- [ ] `argo mcp registry remove <server_id>` deletes a row; refuses if active actions; `--force` overrides
- [ ] Tenant isolation enforced (cross-tenant operations → NotFound)
- [ ] Feature flag `ARGO_MCP_REGISTRY_BACKED=1` switches dispatch from static manifest to SQL registry
- [ ] 8 unit tests + 1 integration smoke + 3 help-smoke rows
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli -p argo-store` clean
- [ ] Every file < 500 LOC

## Branch + PR

- Branch: `codex-mcp-server-registry-cli`
- PR title: `mcp: per-tenant server registry + list/add/remove CLI (codex-mcp-server-registry-cli)`

## Notes for the implementer

- The pre-swarm snapshot lists `argo-mcp` family as already extensive; check `argo mcp --help` for existing subcommands and slot `registry` in alongside them.
- `secrets_ref` is an opaque string pointing at the existing secrets backend (do not store raw secrets in the registry table).
- Markdown-format family parity: rows go through the existing `MarkdownTable` builder used by `argo session messages`, etc.
- The dispatch-side trait seam is the only behaviour change — keep it OFF by default (feature flag). Operators flip it after they've migrated their static manifest entries to the registry.
- If the dispatch file is at-or-near 500 LOC, extract the registry lookup into `worker_actions_executor_mcp_lookup.rs` rather than bloating it.
