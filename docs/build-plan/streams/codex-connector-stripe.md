# V1.2 — Stripe MCP connector

**Owner**: codex. **Depends on**: C-mcp-wire (#806) merged.

## Scope

Native Stripe connector exposed as an MCP server, following
the existing per-connector-crate pattern (mirror layout of
`argo-mcp-connector-gmail` / `argo-mcp-connector-slack`).
Auth via `STRIPE_API_KEY`; methods follow existing connector
shapes.

- Methods: `list_customers`, `get_customer`, `create_charge`,
  `list_charges`, `refund_charge`, `create_invoice`,
  `list_invoices`
- Read-only methods autonomous-safe; charge/refund/invoice
  routed through approval gate by default
- Use `reqwest` with bearer token; reuse existing MCP scaffold

## Files you may touch

NEW crate at `crates/argo-mcp-connector-stripe/` mirroring
the file layout of `argo-mcp-connector-slack`:
- `crates/argo-mcp-connector-stripe/Cargo.toml` (NEW)
- `crates/argo-mcp-connector-stripe/src/lib.rs` (NEW)
- `crates/argo-mcp-connector-stripe/src/main.rs` (NEW)
- `crates/argo-mcp-connector-stripe/src/auth.rs` (NEW)
- `crates/argo-mcp-connector-stripe/src/client.rs` (NEW)
- `crates/argo-mcp-connector-stripe/src/schemas.rs` (NEW)
- `crates/argo-mcp-connector-stripe/src/service.rs` (NEW)
- `Cargo.toml` (workspace) — narrowly: add the new crate to
  the `members` array

NO CLI subcommand needed — connectors are invoked via the
MCP manifest's `[mcp_servers]` section, not via
`argo connector ...` (that surface does not exist; existing
gmail/slack/notion connectors all ship as standalone crates
without a CLI subcommand).

## Acceptance criteria
- [ ] All 7 methods callable via MCP wire
- [ ] Bearer-token auth via `STRIPE_API_KEY`
- [ ] Charge/refund/invoice mark `approval_required: true`
- [ ] List/get methods autonomous-safe
- [ ] 4 unit tests (mocked HTTP) + 1 wire test
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR
- Branch: `codex-connector-stripe`
- PR title: `mcp: native Stripe connector (codex-connector-stripe)`
