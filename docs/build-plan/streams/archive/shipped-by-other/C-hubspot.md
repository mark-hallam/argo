# Stream C — HubSpot MCP connector

**Owner**: codex. **Depends on**: C-mcp-wire merged.

## Scope

MCP server for HubSpot CRM: `hubspot.search_contacts`,
`hubspot.create_contact`, `hubspot.create_deal`,
`hubspot.log_activity`, `hubspot.get_pipeline`. Auth via
HubSpot private-app token.

## Files you may touch
- `crates/argo-mcp-connector-hubspot/**` (NEW)
- `Cargo.toml` (workspace member only — ONE line under `members = [...]`)
- `Cargo.lock` (auto-updated by cargo)
- `manifests/connectors/hubspot.toml`

## Files you must NOT touch
- Other connectors

## Acceptance criteria
- [ ] 5 tools wired
- [ ] Token via `HUBSPOT_PRIVATE_APP_TOKEN`
- [ ] Integration test (gated)
- [ ] Clippy clean
- [ ] Every file < 500 LOC

## Branch + PR
- Branch: `codex-C-hubspot`
- PR title: `mcp(connector): hubspot — contacts / deals / activity / pipeline (C-hubspot)`
