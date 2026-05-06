# V1.2 — Airtable MCP connector

**Owner**: codex. **Depends on**: C-mcp-wire (#806) merged.

## Scope

Airtable connector. Auth via PAT (`AIRTABLE_PAT`).

- Methods: `list_bases`, `list_tables` (base-scoped),
  `list_records` (table-scoped, filterByFormula passthrough),
  `get_record`, `create_record`, `update_record`,
  `delete_record`
- Read methods autonomous-safe; create/update/delete
  approval-gated

## Files you may touch

NEW crate at `crates/argo-mcp-connector-airtable/` mirroring
the file layout of `argo-mcp-connector-asana`:
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

## Acceptance criteria
- [ ] All 7 methods callable
- [ ] filterByFormula passthrough on `list_records`
- [ ] Approval-gate marks correct
- [ ] 4 unit tests + 1 wire test
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR
- Branch: `codex-connector-airtable`
- PR title: `mcp: native Airtable connector (codex-connector-airtable)`
