# V1.2 — Stripe Events MCP connector (read-only)

**Owner**: codex. **Depends on**: C-mcp-wire (#806) merged.

## Why

Read-only Stripe surface for agent observability — list events,
charges, customers, subscriptions — separate from any future
write-side Stripe connector so the boundary is clear. Mirrors
zendesk/discord pattern.

## Scope

Stripe events connector. Auth via restricted/read API key
(`STRIPE_API_KEY`) against `https://api.stripe.com/v1`.

- Methods: `list_events`, `get_event`, `list_charges`,
  `list_customers`, `get_customer`, `list_subscriptions`
- **All read-only** — no approval-gated methods in this
  connector. A separate write-side Stripe connector covers
  charges/refunds.

## Approach

REST client wrapping `reqwest` with `Authorization: Bearer
<key>` header. Stripe uses form-encoded request bodies —
client.rs converts typed params into `application/x-www-form-
urlencoded` (use `serde_urlencoded`). `service.rs` exposes the
MCP tool surface; every method marked autonomous-safe.

## Files you may touch

NEW crate at `crates/argo-mcp-connector-stripe-events/`
mirroring `argo-mcp-connector-zendesk`:
- `crates/argo-mcp-connector-stripe-events/Cargo.toml` (NEW)
- `crates/argo-mcp-connector-stripe-events/src/lib.rs` (NEW)
- `crates/argo-mcp-connector-stripe-events/src/main.rs` (NEW)
- `crates/argo-mcp-connector-stripe-events/src/auth.rs` (NEW)
- `crates/argo-mcp-connector-stripe-events/src/client.rs` (NEW)
- `crates/argo-mcp-connector-stripe-events/src/schemas.rs` (NEW)
- `crates/argo-mcp-connector-stripe-events/src/service.rs` (NEW)
- `Cargo.toml` (workspace) — narrowly: add the new crate to
  the `members` array

NO CLI subcommand — connectors are invoked via the MCP
manifest's `[mcp_servers]` section.

## Files you must NOT touch

- Any other connector crate (incl. existing
  `argo-mcp-connector-stripe`)
- `crates/argo-cli/` and `crates/argo-gateway/`
- Workspace `Cargo.lock` is regenerated on first build —
  matches #951/#953 boundary pattern (don't hand-edit)

## Acceptance criteria

- [ ] All 6 methods callable end-to-end
- [ ] Bearer auth via `STRIPE_API_KEY`
- [ ] All methods marked autonomous-safe (read-only)
- [ ] Cursor-pagination passthrough on every `list_*`
- [ ] 4–8 unit tests + 1 wire test against a recorded fixture
- [ ] Each `src/*.rs` ≤ 500 LOC (HARD RULE 6)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR

- Branch: `codex-connector-stripe-events`
- PR title: `mcp: native Stripe events read-only connector (codex-connector-stripe-events)`

## Notes

- ~700 LOC total across 6 src files plus Cargo.toml.
- This is a **read-only** connector — refuse on a restricted
  key with write scope by surfacing the Stripe error verbatim;
  no client-side scope check.
- Stripe uses form-encoded bodies, not JSON — don't copy the
  JSON path from zendesk/discord client.rs.
