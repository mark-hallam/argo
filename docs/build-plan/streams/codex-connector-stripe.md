# V1.2 — Stripe MCP connector

**Owner**: codex. **Depends on**: C-mcp-wire (#806) merged.

## Scope

Native Stripe connector exposed as an MCP server. Standard
shape: `argo connector stripe` runs the server; auth via
`STRIPE_API_KEY` env; methods follow existing connector
patterns (gmail, slack).

- Methods: `list_customers`, `get_customer`, `create_charge`,
  `list_charges`, `refund_charge`, `create_invoice`,
  `list_invoices`
- Read-only methods autonomous-safe; charge/refund/invoice
  routed through approval gate by default
- Use `reqwest` with bearer token; reuse existing MCP scaffold

## Files you may touch
- `crates/argo-mcp/src/connectors/stripe.rs` (NEW, ~400 LOC
  cap)
- `crates/argo-mcp/src/connectors/mod.rs` — narrowly: add
  `stripe` module
- `crates/argo-cli/src/handlers/connector_stripe.rs` (NEW)
- `crates/argo-cli/src/cli_connector_cmds.rs` — narrowly:
  `Stripe` subcommand variant
- BLANKET CLI ALLOWLIST applies

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
