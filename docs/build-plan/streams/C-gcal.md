# Stream C — Google Calendar MCP connector

**Owner**: codex. **Depends on**: C-mcp-wire merged.

## Scope

MCP server for Google Calendar: `gcal.list_events`,
`gcal.create_event`, `gcal.update_event`, `gcal.suggest_time`,
`gcal.respond_to_invite`. OAuth2 (same provider as gmail —
share the token store if convenient).

## Files you may touch
- `crates/argo-mcp-connector-gcal/**` (NEW)
- `Cargo.toml` (workspace member only — ONE line under `members = [...]`)
- `Cargo.lock` (auto-updated by cargo)
- `manifests/connectors/gcal.toml`

## Files you must NOT touch
- Other connectors
- `argo-store/**` — same v1 deferral as C-gmail

## Acceptance criteria
- [ ] All 5 tools wired
- [ ] OAuth via `oauth2`; refresh token from
      `ARGO_GCAL_REFRESH_TOKEN` env var; access token cached
      in `<cwd>/.argo/connectors/gcal/access_token.json` (same
      v1 shape as C-gmail, separate file). v1.x swaps in a
      shared keyring; do NOT build that here.
- [ ] Integration test (`GCAL_TEST_TOKEN`)
- [ ] Clippy clean
- [ ] Every file < 500 LOC

## Branch + PR
- Branch: `codex-C-gcal`
- PR title: `mcp(connector): gcal — list / create / update / suggest_time / respond (C-gcal)`
