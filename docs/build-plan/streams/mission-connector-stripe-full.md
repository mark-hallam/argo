# mission-connector-stripe-full — Stripe MCP connector — full REST API (beyond stripe-events)

**Owner**: codex. **Depends on**: nothing.
**Authored against**: origin/main `f4fd2459` on `2026-05-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-connector-stripe-full`.

## Scope

v1.2 capability breadth. Existing `argo-mcp-connector-stripe-events` covers webhooks. New full-API connector for payments/customers/invoices/subscriptions/refunds. Bearer token via secrets.toml.

LOC: ~500.

## Files you may touch

- NEW `crates/argo-mcp-connector-stripe/src/lib.rs` — REST client wrapper + 10+ tools. ≤400 LOC.
- NEW `crates/argo-mcp-connector-stripe/src/lib_tests.rs` — mock HTTP unit tests. ≤180 LOC.
- NEW `crates/argo-mcp-connector-stripe/Cargo.toml`.
- Workspace `Cargo.toml` — add member. ≤3 lines.
- `crates/argo-mcp/src/registry.rs` — register connector. ≤15 modified lines.

## Files you must NOT touch

- Anything outside the may-touch list above.
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] Implementation matches scope above.
- [ ] All new tests pass.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] No file > 500 LOC.
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `codex-mission-connector-stripe-full`
- PR title: `feat: Stripe MCP connector — full REST API (beyond stripe-events) (mission-connector-stripe-full)`

## Notes for the implementer

- Priority: v1.2 connector breadth.
- Authored as part of post-v1 backlog batch 2026-05-15.
