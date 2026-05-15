# mission-connector-salesforce — Salesforce MCP connector — CRM (CRUD + SOQL)

**Owner**: codex. **Depends on**: nothing.
**Authored against**: origin/main `f4fd2459` on `2026-05-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-connector-salesforce`.

## Scope

v1.2 capability breadth. NEW connector for Salesforce (accounts/contacts/opportunities/leads/custom objects). OAuth or private-app token. Tools: CRUD + bulk + SOQL queries.

LOC: ~1100.

## Files you may touch

- NEW `crates/argo-mcp-connector-salesforce/src/lib.rs` — OAuth + REST + 15+ tools. ≤700 LOC.
- NEW `crates/argo-mcp-connector-salesforce/src/soql.rs` — SOQL helper. ≤200 LOC.
- NEW `crates/argo-mcp-connector-salesforce/src/lib_tests.rs` — mock + integration. ≤200 LOC.
- Workspace `Cargo.toml` — add member.
- `crates/argo-mcp/src/registry.rs` — register.

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

- Branch: `codex-mission-connector-salesforce`
- PR title: `feat: Salesforce MCP connector — CRM (CRUD + SOQL) (mission-connector-salesforce)`

## Notes for the implementer

- Priority: v1.2 enterprise connector.
- Authored as part of post-v1 backlog batch 2026-05-15.
