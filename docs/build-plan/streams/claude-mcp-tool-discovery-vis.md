# claude-mcp-tool-discovery-vis — `argo mcp tools <server>` lists available tools from a connected MCP server

**Owner**: claude. **Depends on**: `mission-wire-mcp-action-executor` (#933) merged. Soft dep on `codex-mcp-server-registry-cli` (use the registry if available; fall back to static manifest otherwise).

## Why

When operators wire a new MCP server they need to know what tools that server exposes — names, JSON schemas, descriptions — to write the matching action declarations. Today they read the server's docs (often stale) or curl `list_tools` by hand. This brief ships a first-class CLI surface that calls `list_tools` against a connected server and renders the result in our standard markdown-format family.

## Scope

1. **NEW** `crates/argo-cli/src/handlers/mcp_tools.rs` (~310 LOC):
   - `cmd_tools(tenant_id, server_id, opts) -> ArgoResult<()>` — top-level handler
   - Resolves the server connection via the existing `worker_actions_executor_mcp` pool (reuse the per-server `reqwest::Client`)
   - Calls `list_tools` (MCP standard endpoint)
   - Renders markdown by default: per-tool section with `name`, `description`, `input_schema` (pretty JSON), `output_schema` if present
   - `--format json` emits one tool per NDJSON line
   - `--filter <regex>` narrows by tool name
   - `--schema-only` omits descriptions for terse output
2. **CLI plumbing** — add `mcp tools <server_id>` clap subcommand under the `mcp` tree
3. **Help-smoke** — 1 row pinning the new variant
4. **Lookup precedence** — try the SQL registry (`codex-mcp-server-registry-cli`) if the feature flag is on; else fall back to the static manifest at `manifests/connectors/mcp.toml`

## Files you may touch

- `crates/argo-cli/src/handlers/mcp_tools.rs` (NEW, ≤ 360 LOC)
- `crates/argo-cli/src/handlers/mcp_tools_tests.rs` (NEW, ≤ 260 LOC sibling) — 7 unit tests:
  - markdown rendering for a 2-tool fixture
  - --format json emits NDJSON
  - --filter regex narrows tools
  - --schema-only omits descriptions
  - server not found → ArgoError::NotFound
  - server unreachable → clean Provider error with body excerpt
  - tenant isolation (cross-tenant server lookup → NotFound)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 1 `pub mod` line
- `crates/argo-cli/src/commands.rs` — narrowly: add `mcp tools` clap variant
- `crates/argo-cli/tests/mcp_tools_smoke.rs` (NEW, ≤ 220 LOC) — 1 integration test using a Python-stdlib MCP echo fixture (mirror of `mcp_action_executor_smoke.rs` + `tests/fixtures/mcp_echo_server.py` from #933)
- `crates/argo-cli/tests/help_smoke.rs` — narrowly: 1 row

## Files you must NOT touch

- `argo-mcp-connector-*` crates — protocol library; consume, don't extend
- `worker_actions_executor_mcp.rs` (the dispatch-side file from #933) — only consume the pool via its public surface; do NOT extend its API
- `argo-vertical/src/manifest.rs`
- `argo-store` migrations / schema

## Acceptance criteria

- [ ] `argo mcp tools <server_id>` calls `list_tools` and renders markdown
- [ ] `--format json` emits NDJSON one tool per line
- [ ] `--filter <regex>` narrows by name
- [ ] `--schema-only` omits descriptions
- [ ] Server unreachable → `Err(ArgoError::Provider(...))` with status + body excerpt
- [ ] Cross-tenant server → `Err(ArgoError::NotFound(...))`
- [ ] 7 unit tests + 1 integration smoke + 1 help-smoke row
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli` clean
- [ ] Every file < 500 LOC

## Branch + PR

- Branch: `claude-mcp-tool-discovery-vis`
- PR title: `cli: argo mcp tools for MCP tool discovery (claude-mcp-tool-discovery-vis)`

## Notes for the implementer

- Reuse the connection pool from `worker_actions_executor_mcp.rs` via its existing `default_pool()` accessor — do NOT spin up a new `reqwest::Client` per call.
- `list_tools` is part of the MCP protocol surface; the connector crate already implements it. If the trait method is named differently in our code (`enumerate_tools`, etc.) match the existing name.
- Markdown rendering mirrors the existing `argo skill list` output format (per-item section blocks). Keep tabular reserved for `argo mcp registry list`.
- For `--filter`, use `regex` (already a workspace dep) and validate the pattern eagerly — return clear error on invalid regex.
- If `claude-cu-screenshot-storage`'s `MarkdownTable` helper is in flight, do not depend on it; use the existing per-section markdown builder used elsewhere in the CLI.
