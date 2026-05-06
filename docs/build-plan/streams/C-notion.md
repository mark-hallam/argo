# Stream C — Notion MCP connector

**Owner**: codex. **Depends on**: C-mcp-wire merged.

## Scope

MCP server for Notion: `notion.search`, `notion.read_page`,
`notion.create_page`, `notion.append_block`,
`notion.update_database`. Auth via Notion integration token.

## Files you may touch
- `crates/argo-mcp-connector-notion/**` (NEW)
- `Cargo.toml` (workspace member only — ONE line under `members = [...]`)
- `Cargo.lock` (auto-updated by cargo)
- `manifests/connectors/notion.toml`

## Files you must NOT touch
- Other connectors

## Acceptance criteria
- [ ] 5 tools wired
- [ ] Token via `NOTION_INTEGRATION_TOKEN`
- [ ] Integration test (gated)
- [ ] Clippy clean
- [ ] Every file < 500 LOC

## Branch + PR
- Branch: `codex-C-notion`
- PR title: `mcp(connector): notion — search / read / create / append / update_db (C-notion)`
