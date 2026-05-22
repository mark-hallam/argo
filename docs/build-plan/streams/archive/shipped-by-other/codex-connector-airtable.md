# V1.2 — Airtable MCP connector

**Owner**: codex. **Depends on**: C-mcp-wire (#806) merged.

## Why

Native Airtable surface for agent reads + gated mutation.
Mirrors zendesk/discord pattern.

## Scope

Airtable connector. Auth via API key (`AIRTABLE_API_KEY`)
against `https://api.airtable.com/v0`.

- Methods: `list_bases`, `list_tables` (base-scoped),
  `list_records` (table-scoped, filterByFormula passthrough),
  `get_record`, `create_record`, `update_record`,
  `delete_record` (approval — destructive)
- Read methods autonomous-safe; create/update approval-gated;
  `delete_record` approval-gated as destructive

## Approach

REST client wrapping `reqwest` with `Authorization: Bearer
<key>` header. `client.rs` exposes one method per public
surface and passes `filterByFormula` and pagination cursors
through verbatim. `service.rs` applies the approval gate.

## Files you may touch

NEW crate at `crates/argo-mcp-connector-airtable/` mirroring
`argo-mcp-connector-zendesk`:
- `crates/argo-mcp-connector-airtable/Cargo.toml` (NEW)
- `crates/argo-mcp-connector-airtable/src/lib.rs` (NEW)
- `crates/argo-mcp-connector-airtable/src/main.rs` (NEW)
- `crates/argo-mcp-connector-airtable/src/auth.rs` (NEW)
- `crates/argo-mcp-connector-airtable/src/client.rs` (NEW)
- `crates/argo-mcp-connector-airtable/src/schemas.rs` (NEW)
- `crates/argo-mcp-connector-airtable/src/service.rs` (NEW)
- `Cargo.toml` (workspace) — narrowly: add the new crate to
  the `members` array

NO CLI subcommand — connectors are invoked via the MCP
manifest's `[mcp_servers]` section.

## Files you must NOT touch

- Any other connector crate
- `crates/argo-cli/` and `crates/argo-gateway/`
- Workspace `Cargo.lock` is regenerated on first build —
  matches #951/#953 boundary pattern (don't hand-edit)

## Acceptance criteria

- [ ] All 7 methods callable end-to-end
- [ ] Bearer auth via `AIRTABLE_API_KEY`
- [ ] `filterByFormula` + cursor passthrough on `list_records`
- [ ] Approval-gate marks correct (read=false,
      write/delete=true; delete clearly flagged destructive)
- [ ] 4–8 unit tests + 1 wire test against a recorded fixture
- [ ] Each `src/*.rs` ≤ 500 LOC (HARD RULE 6)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR

- Branch: `codex-connector-airtable`
- PR title: `mcp: native Airtable connector (codex-connector-airtable)`

## Notes

- ~700 LOC total across 6 src files plus Cargo.toml.
- `delete_record` is the only method in the connector layer
  that's destructive — annotate the approval gate accordingly
  so the gateway can show a stronger confirmation prompt.
