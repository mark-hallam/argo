# Stream C — Google Calendar MCP connector

**Owner**: codex. **Depends on**: C-mcp-wire merged.

## Scope

MCP server for Google Calendar: `gcal.list_events`,
`gcal.create_event`, `gcal.update_event`, `gcal.suggest_time`,
`gcal.respond_to_invite`. OAuth2 (same provider as gmail —
share the token store if convenient).

## Files you may touch
- `crates/argo-mcp-connector-gcal/**` (NEW)
- `Cargo.toml`
- `manifests/connectors/gcal.toml`

## Files you must NOT touch
- Other connectors

## Acceptance criteria
- [ ] All 5 tools wired
- [ ] OAuth via `oauth2` (share refresh logic with gmail if
      possible — but keep crates separate)
- [ ] Integration test (`GCAL_TEST_TOKEN`)
- [ ] Clippy clean
- [ ] Every file < 500 LOC

## Branch + PR
- Branch: `codex-C-gcal`
- PR title: `mcp(connector): gcal — list / create / update / suggest_time / respond (C-gcal)`
