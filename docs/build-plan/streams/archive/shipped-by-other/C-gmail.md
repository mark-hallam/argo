# Stream C — Gmail MCP connector

**Owner**: codex. **Depends on**: C-mcp-wire merged.

## Scope

Implement an MCP server that exposes Gmail operations as tools
the worker can call. Tools: `gmail.search`, `gmail.read`,
`gmail.draft`, `gmail.send`, `gmail.label`. Uses the official
Google Gmail API with OAuth2.

## Files you may touch
- `crates/argo-mcp-connector-gmail/**` (NEW crate)
- `Cargo.toml` (workspace member only — narrowly scoped: ONE
  line under `members = [...]`)
- `Cargo.lock` (auto-updated by cargo when adding the new
  crate's deps; commit the lockfile change)
- `manifests/connectors/gmail.toml` (declares OAuth scopes,
  required env vars, tool catalog)

## Files you must NOT touch
- Other connectors
- `argo-mcp/**` core (only the server harness if needed; ask
  in BUILD_STATUS.md if you need to)
- `Executor` trait or its impls
- `argo-store/**` — the keyring abstraction is v1.x; for v1
  use the env-var + workspace-file token cache shape below

## Acceptance criteria
- [ ] Crate builds, all tools listed via
      `argo mcp serve --server gmail --list-tools` (or
      equivalent test helper)
- [ ] OAuth flow via `oauth2` crate; refresh token from
      `ARGO_GMAIL_REFRESH_TOKEN` env var, access token cached
      in `<cwd>/.argo/connectors/gmail/access_token.json`
      (created with `0600` perms; gitignored at repo root).
      v1.x will swap the file cache for an `argo-store` keyring
      abstraction; do NOT build that abstraction here.
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
