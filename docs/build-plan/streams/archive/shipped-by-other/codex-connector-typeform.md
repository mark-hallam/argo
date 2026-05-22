# V1.2 — Typeform MCP connector

**Owner**: codex. **Depends on**: C-mcp-wire (#806) merged.

## Why

Native Typeform surface so agents can read forms/responses and
gate form mutation behind approval. Mirrors zendesk/discord
pattern.

## Scope

Typeform connector. Auth via PAT (`TYPEFORM_TOKEN`) against
`https://api.typeform.com`.

- Methods: `list_forms`, `get_form`, `list_responses`
  (form-scoped), `get_response`, `create_form` (approval),
  `update_form` (approval)
- Read methods autonomous-safe; create_form/update_form
  approval-gated

## Approach

REST client wrapping `reqwest` with `Authorization: Bearer
<token>` header. `client.rs` exposes one method per public
surface; `service.rs` applies the approval gate on
create/update.

## Files you may touch

NEW crate at `crates/argo-mcp-connector-typeform/` mirroring
`argo-mcp-connector-zendesk`:
- `crates/argo-mcp-connector-typeform/Cargo.toml` (NEW)
- `crates/argo-mcp-connector-typeform/src/lib.rs` (NEW)
- `crates/argo-mcp-connector-typeform/src/main.rs` (NEW)
- `crates/argo-mcp-connector-typeform/src/auth.rs` (NEW)
- `crates/argo-mcp-connector-typeform/src/client.rs` (NEW)
- `crates/argo-mcp-connector-typeform/src/schemas.rs` (NEW)
- `crates/argo-mcp-connector-typeform/src/service.rs` (NEW)
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

- [ ] All 6 methods callable end-to-end
- [ ] Bearer auth via `TYPEFORM_TOKEN`
- [ ] Approval-gate marks correct (read=false,
      create_form/update_form=true)
- [ ] 4–8 unit tests + 1 wire test against a recorded fixture
- [ ] Each `src/*.rs` ≤ 500 LOC (HARD RULE 6)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR

- Branch: `codex-connector-typeform`
- PR title: `mcp: native Typeform connector (codex-connector-typeform)`

## Notes

- ~700 LOC total across 6 src files plus Cargo.toml.
- `update_form` is PUT `/forms/{id}` with the full form
  body — the connector does NOT do partial-update merging;
  callers must supply the complete form definition.
- `list_responses` should pass `since`/`until`/`page_size`
  through verbatim — useful for downstream pagination.
