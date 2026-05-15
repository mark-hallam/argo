# codex-marketplace-install-authz — Gate marketplace install POST behind bearer/JWT auth

**Owner**: codex. **Depends on**: nothing.
**Authored against**: origin/main `ea8f41d0` on `2026-05-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-marketplace-install-authz`.
**Source**: F-002 in `docs/skycot/audit/2026-05-15-codex-full-architecture-audit.md`.

## Scope

P1 security. The marketplace install POST endpoint currently trusts a spoofable `x-tenant-id` header as the principal. The route is intentionally mounted ahead of the bearer gate (`crates/argo-cli/src/handlers/gateway/routes.rs:93-98` comment block: "Same pre-bearer-gate mount as the dashboard MVP: browser POSTs work without per-tab tokens; tenant isolation is enforced via `x-tenant-id`"). The install handler at `crates/argo-cli/src/handlers/gateway/marketplace.rs:194` (`install_action`) derives authn from `x-tenant-id` at `:202` then globally toggles a skill row via `set_shared` at `:243`. Anyone with HTTP access can send `x-tenant-id: any-tenant` and install skills into other tenants.

**Operator decision (Q2, 2026-05-15):** POST install requires bearer/JWT; GET browsing stays anonymous (anonymous browse is a normal SaaS pattern for prospects).

Scope:
- In `routes.rs`: split the marketplace dispatch into GET (anonymous, pre-bearer-gate) vs POST (bearer-gated). POST install must NOT bypass `check_bearer_auth`.
- In `marketplace.rs`: refactor `install_action` to take the authenticated tenant from the bearer/JWT claims (not from `x-tenant-id` header). Reject the request if no authenticated principal.
- Update the comment block at `routes.rs:93-98` so future agents see the new design.
- Audit every install with the authenticated principal (existing audit chain).
- Tests: install POST without bearer returns 401; install POST with bearer-derived tenant != URL/body target returns 403; install POST with matching tenant succeeds; GET browse without bearer still returns 200.

LOC: ~220.

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes.rs` — split marketplace dispatch: GET stays pre-bearer; POST goes after `check_bearer_auth`. Update the design comment. ≤45 modified lines.
- `crates/argo-cli/src/handlers/gateway/marketplace.rs` — `install_action` accepts authenticated principal; remove `x-tenant-id` header trust; bump tests. ≤140 modified lines.
- Test sibling: existing `mod tests` at `marketplace.rs` (line ~314 onward) extends. Or a new sibling `marketplace_authz_tests.rs` if cleaner. ≤80 LOC.

## Files you must NOT touch

- Anything outside the may-touch list above.
- The bearer/JWT verification logic itself (`check_bearer_auth`, `jwt::verify_jwt`) — those are existing primitives this brief consumes.
- Skill share storage (`argo-store::skill_share`) — F-002 fix is at the auth boundary, not the storage layer.

## Acceptance criteria

- [ ] POST `/marketplace/<...>/install` returns 401 without bearer.
- [ ] POST install derives tenant from bearer/JWT claims; rejects mismatched URL/body tenant with 403.
- [ ] GET `/marketplace` and detail pages stay anonymous (no bearer required).
- [ ] Updated routes.rs comment block reflects the new design.
- [ ] Every install logs an audit row with the authenticated principal.
- [ ] `cargo test -p argo-cli marketplace` passes.
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `codex-marketplace-install-authz`
- PR title: `feat(gateway): gate marketplace install POST behind bearer/JWT auth (codex-marketplace-install-authz)`

## Notes for the implementer

- The fix is at the auth boundary, not the storage layer. `skill_share::set_shared` keying by (name, version) is F-002b-shaped (`add owner/provenance checks`) and may be a follow-up brief.
- For v1 the auth derivation pattern should match how `dispatch_post` (`routes.rs:108`) does it — reuse the existing pattern, don't invent a new one.
- Anonymous GET is a deliberate product choice; do NOT gate GET in this brief.
