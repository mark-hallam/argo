# V1.2 — Square MCP connector

**Owner**: codex. **Depends on**: C-mcp-wire (#806) merged.

## Why

Native Square surface for read-side payments/customers/catalog
/inventory observability. Mirrors zendesk/discord pattern.

## Scope

Square connector. Auth via `SQUARE_ACCESS_TOKEN` against
`https://connect.squareup.com/v2`.

- Methods: `list_payments`, `get_payment`, `list_customers`,
  `list_catalog_items`, `get_inventory` (catalog-object-id
  scoped)
- **All read-only** — every method autonomous-safe in this
  connector. A future write-side Square connector handles
  catalog/inventory mutation.

## Approach

REST client wrapping `reqwest` with `Authorization: Bearer
<token>` and `Square-Version: 2024-04-17` headers. `client.rs`
exposes a typed method per public surface; `service.rs` marks
all methods autonomous-safe.

## Files you may touch

NEW crate at `crates/argo-mcp-connector-square/` mirroring
`argo-mcp-connector-zendesk`:
- `crates/argo-mcp-connector-square/Cargo.toml` (NEW)
- `crates/argo-mcp-connector-square/src/lib.rs` (NEW)
- `crates/argo-mcp-connector-square/src/main.rs` (NEW)
- `crates/argo-mcp-connector-square/src/auth.rs` (NEW)
- `crates/argo-mcp-connector-square/src/client.rs` (NEW)
- `crates/argo-mcp-connector-square/src/schemas.rs` (NEW)
- `crates/argo-mcp-connector-square/src/service.rs` (NEW)
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

- [ ] All 5 methods callable end-to-end
- [ ] Bearer auth via `SQUARE_ACCESS_TOKEN`
- [ ] All methods marked autonomous-safe (read-only)
- [ ] Cursor-pagination passthrough on every `list_*`
- [ ] 4–8 unit tests + 1 wire test against a recorded fixture
- [ ] Each `src/*.rs` ≤ 500 LOC (HARD RULE 6)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR

- Branch: `codex-connector-square`
- PR title: `mcp: native Square connector (codex-connector-square)`

## Notes

- ~700 LOC total across 6 src files plus Cargo.toml.
- Pin `Square-Version` in a `const` — do NOT take it from env.
- `get_inventory` = GET `/inventory/{catalog_object_id}` —
  parameter is the catalog-object-id, not a SKU.
