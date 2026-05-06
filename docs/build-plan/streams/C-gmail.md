# Stream C — Gmail MCP connector

**Owner**: codex. **Depends on**: C-mcp-wire merged.

## Scope

Implement an MCP server that exposes Gmail operations as tools
the worker can call. Tools: `gmail.search`, `gmail.read`,
`gmail.draft`, `gmail.send`, `gmail.label`. Uses the official
Google Gmail API with OAuth2.

## Files you may touch
- `crates/argo-mcp-connector-gmail/**` (NEW crate)
- `Cargo.toml` (workspace member only)
- `manifests/connectors/gmail.toml` (declares OAuth scopes,
  required env vars, tool catalog)

## Files you must NOT touch
- Other connectors
- `argo-mcp/**` core (only the server harness if needed; ask
  in BUILD_STATUS.md if you need to)
- `Executor` trait or its impls

## Acceptance criteria
- [ ] Crate builds, all tools listed via
      `argo mcp serve --server gmail --list-tools` (or
      equivalent test helper)
- [ ] OAuth flow via `oauth2` crate; tokens persisted via the
      argo-store keyring abstraction
- [ ] All 5 tools work against a real Gmail account in an
      integration test (gated behind `GMAIL_TEST_TOKEN` env)
- [ ] `cargo clippy -p argo-mcp-connector-gmail -- -D warnings`
      clean
- [ ] Every file < 500 LOC

## Branch + PR
- Branch: `codex-C-gmail`
- PR title: `mcp(connector): gmail — search / read / draft / send / label (C-gmail)`

## Notes
- MCP protocol: see `crates/argo-mcp/` for the server-side
  harness. Mirror the shape of any existing in-tree connector
  (search the repo for `argo-mcp-connector-*` to find the
  first-mover; if none exists yet, you're the first-mover and
  set the pattern).
- Token refresh on 401; never log raw tokens.
- Tool args + return types: define in JSON Schema, validated
  by the MCP harness before dispatch.
