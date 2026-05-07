# V1.2 — QuickBooks Online MCP connector

**Owner**: codex. **Depends on**: C-mcp-wire (#806) merged.

## Why

Native QuickBooks Online surface so finance-flavoured agents
can read invoices/customers/items/balance-sheet and propose
new invoices behind an approval gate. Mirrors zendesk/discord
pattern.

## Scope

QuickBooks Online connector. OAuth2 refresh-token flow via
`QB_REFRESH_TOKEN` + `QB_CLIENT_ID` + `QB_CLIENT_SECRET`
against `https://quickbooks.api.intuit.com/v3/company/{realmId}`.
Realm ID supplied as method param (multi-company support).

- Methods: `list_invoices`, `get_invoice`, `create_invoice`
  (approval), `list_customers`, `list_items`,
  `get_balance_sheet` (report)
- Read methods autonomous-safe; `create_invoice`
  approval-gated

## Approach

REST client wrapping `reqwest`. `auth.rs` exchanges the
refresh token for an access token at startup and refreshes
on 401. `client.rs` issues GET/POST to the company-scoped
base URL with `Authorization: Bearer <access>`. `service.rs`
applies the approval gate on `create_invoice`.

## Files you may touch

NEW crate at `crates/argo-mcp-connector-quickbooks/` mirroring
`argo-mcp-connector-zendesk`:
- `crates/argo-mcp-connector-quickbooks/Cargo.toml` (NEW)
- `crates/argo-mcp-connector-quickbooks/src/lib.rs` (NEW)
- `crates/argo-mcp-connector-quickbooks/src/main.rs` (NEW)
- `crates/argo-mcp-connector-quickbooks/src/auth.rs` (NEW)
- `crates/argo-mcp-connector-quickbooks/src/client.rs` (NEW)
- `crates/argo-mcp-connector-quickbooks/src/schemas.rs` (NEW)
- `crates/argo-mcp-connector-quickbooks/src/service.rs` (NEW)
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
- [ ] OAuth2 refresh-token rotation on 401
- [ ] Realm-id (company-id) parameter on every method
- [ ] Approval-gate marks correct (only `create_invoice=true`)
- [ ] 4–8 unit tests + 1 wire test against a recorded fixture
- [ ] Each `src/*.rs` ≤ 500 LOC (HARD RULE 6)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR

- Branch: `codex-connector-quickbooks`
- PR title: `mcp: native QuickBooks Online connector (codex-connector-quickbooks)`

## Notes

- ~700 LOC total across 6 src files plus Cargo.toml.
- QBO returns SQL-ish strings via `/query?query=...` — wrap
  the user-supplied filter in a typed builder, do not pass
  raw SQL through the MCP surface.
- `get_balance_sheet` is `/reports/BalanceSheet` — return the
  raw JSON report; do not attempt to flatten it.
