# V1.2 — Shopify Admin MCP connector

**Owner**: codex. **Depends on**: C-mcp-wire (#806) merged.

## Why

Native Shopify Admin surface so agents can read products/orders
/customers and gate fulfilment behind approval. Mirrors
zendesk/discord pattern.

## Scope

Shopify Admin connector. Auth via shop URL + access token
(`SHOPIFY_SHOP` + `SHOPIFY_TOKEN`) against
`https://{SHOPIFY_SHOP}.myshopify.com/admin/api/2024-04`.

- Methods: `list_products`, `get_product`, `list_orders`,
  `get_order`, `fulfill_order` (approval), `list_customers`
- Read methods autonomous-safe; `fulfill_order`
  approval-gated

## Approach

REST client wrapping `reqwest` with
`X-Shopify-Access-Token: <token>` header (NOT bearer). Base
URL is shop-derived — `auth.rs` must validate `SHOPIFY_SHOP`
matches `^[a-z0-9-]+$` to prevent host injection. `service.rs`
applies the approval gate on `fulfill_order` only.

## Files you may touch

NEW crate at `crates/argo-mcp-connector-shopify/` mirroring
`argo-mcp-connector-zendesk`:
- `crates/argo-mcp-connector-shopify/Cargo.toml` (NEW)
- `crates/argo-mcp-connector-shopify/src/lib.rs` (NEW)
- `crates/argo-mcp-connector-shopify/src/main.rs` (NEW)
- `crates/argo-mcp-connector-shopify/src/auth.rs` (NEW)
- `crates/argo-mcp-connector-shopify/src/client.rs` (NEW)
- `crates/argo-mcp-connector-shopify/src/schemas.rs` (NEW)
- `crates/argo-mcp-connector-shopify/src/service.rs` (NEW)
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
- [ ] Header auth via `X-Shopify-Access-Token`
- [ ] `SHOPIFY_SHOP` validated `^[a-z0-9-]+$`
- [ ] Approval-gate marks correct (only `fulfill_order=true`)
- [ ] 4–8 unit tests + 1 wire test against a recorded fixture
- [ ] Each `src/*.rs` ≤ 500 LOC (HARD RULE 6)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR

- Branch: `codex-connector-shopify`
- PR title: `mcp: native Shopify Admin connector (codex-connector-shopify)`

## Notes

- ~700 LOC total across 6 src files plus Cargo.toml.
- Pin Shopify API version (e.g. `2024-04`) in a `const` —
  do NOT take it from env.
- `fulfill_order` is POST to `/orders/{id}/fulfillments.json`
  — surface non-201 errors as MCP errors verbatim.
