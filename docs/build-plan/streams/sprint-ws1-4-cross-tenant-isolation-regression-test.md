# sprint-ws1-4-cross-tenant-isolation-regression-test

**Owner**: claude or codex.
**Depends on**: #2676 (MERGED — JWT tenant resolver + the 6 migrated read routes).
**Authored against**: origin/main `c9ff99509` on `2026-06-04` (call sites re-verified 2026-06-04: resolver `resolve_tenant_for_request` @ `crates/argo-cli/src/handlers/gateway/resolve_tenant.rs:54`; sibling tests `resolve_tenant_tests.rs` + `crates/argo-cli/tests/tenant_resolve_from_session_jwt.rs` present; keystone #2676 MERGED).
**Validate before claim**: `bash scripts/check-brief-freshness.sh sprint-ws1-4-cross-tenant-isolation-regression-test`.

## Why

The keystone (#2676) makes each user's reads resolve to their own tenant via the session
JWT. Its adversarial review (run `wf_02bda054-211`) flagged that negative coverage is thin:
the existing tests prove the resolver picks A's tenant, but there is NO end-to-end
integration test asserting that A's session JWT, hitting the actual migrated routes, cannot
see B's data. This brief locks the keystone's guarantee as a permanent regression test so a
future change that re-introduces cwd-pooling fails CI loudly.

## What

Add an integration test (in `crates/argo-cli/tests/`, sibling to
`tenant_resolve_from_session_jwt.rs`) that, for the migrated read surfaces, proves
**A-cannot-read-B**:
- Provision two real users → tenants A and B.
- Seed each tenant with distinct data it owns (e.g. a worker, a skill, an audit row) via the
  real per-tenant write/seed path.
- For each migrated route (`/api/v1/team`, `/workers` overview+detail, `/skills`,
  `/tenant/usage`, `/audit` default-scope, `/approvals`): a request bearing **A's**
  `__Host-session` JWT returns ONLY A's data — never B's, never the cwd/demo tenant's.
- Symmetrically, B's JWT returns only B's.
- (Bonus, ties off review follow-up #4) one case asserting an attacker-supplied
  `X-Argo-Tenant-Id: <B>` on an A-session does NOT return B's rows on the routes where the
  JWT must win.

The test MUST be written so it would FAIL if the resolver regressed to "always cwd" or if a
route reverted to the old `resolve_workspace_tenant`.

## May-touch
- `crates/argo-cli/tests/cross_tenant_isolation_jwt_routes.rs` (NEW)
- test-only helpers if needed (no production-code change — this is a test brief)
- `docs/build-plan/BUILD_STATUS.md` (ledger flip)

Do NOT modify production code; if the test surfaces a real leak, STOP and flag it as a P0
(do not paper over it by weakening the assertion).

## Acceptance criteria
- [ ] New integration test GREEN: A's JWT sees only A across all 6 migrated routes; B only B; neither sees cwd/demo.
- [ ] The X-Argo-Tenant-Id-override case asserts the JWT wins (no B data leaks to an A-session).
- [ ] Sanity: temporarily reverting one migrated route to `resolve_workspace_tenant` makes the test FAIL (note this in the PR body as the regression-proof; revert the revert before pushing).
- [ ] `cargo test -p argo-cli` for the new test GREEN (report pass line + wall-clock).
- [ ] No `unwrap`/`expect` outside `#[cfg(test)]` (this is a test file, so fine within it).

## Out of scope
- The audit-header capability gate + JWT-secret boot guard (separate WS1-1b follow-ups; this is the test that would have caught a regression, not the hardening itself).

## Parallel-surface check
- `rg 'cross_tenant|seed_cross_tenant_invisibility' crates/argo-cli/tests/` — extend the established cross-tenant test pattern; don't fork a parallel harness.

## Source
Adversarial review of #2676 (`wf_02bda054-211`), follow-up #4. Audit: `docs/skycot/audit/2026-06-04-day0-sprint-gap-audit.md` (WS1-4).
