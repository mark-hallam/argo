# eval-mcp-client-pool-tenant-keyed

**Owner**: codex
**Priority**: P2 - claim 5 substrate hardening
**Authored against**: origin/main `7354f039d` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-mcp-client-pool-tenant-keyed`.

## Source

`crates/argo-mcp/src/pool.rs` documents `(TenantId, manifest_id)` as the client
pool key. Earlier evaluator rounds flagged historical tenant-blind MCP client
pooling as a cross-tenant credential risk. The invariant needs integration-level
coverage, not only in-module tests.

## Goal

Add an integration contract that proves the MCP client pool is tenant-keyed and
that factory-supplied credentials do not cross tenants.

## May Touch

- `crates/argo-mcp/tests/cross_tenant_pool_isolation_test.rs` (NEW)
- `crates/argo-mcp/tests/fixtures/**` only for existing echo/mock server reuse
- `docs/skycot/evals/scheduled/2026-05-24/eval-mcp-client-pool-tenant-keyed.md` (NEW report, optional if findings appear)
- `docs/build-plan/BUILD_STATUS.md`

## Must Not Touch

- Do not change MCP connector crates.
- Do not change production pool code unless the test exposes a real bug and the fix is tiny.
- Do not add network calls outside local test fixtures.

## Acceptance

- [ ] Two tenants requesting the same manifest receive distinct client identities.
- [ ] The same tenant requesting the same manifest reuses the expected client.
- [ ] Factory-supplied credentials or auth labels cannot cross tenant boundaries.
- [ ] A compile/signature sentinel fails loudly if tenant is removed from the acquisition API.
- [ ] `cargo test -p argo-mcp --test cross_tenant_pool_isolation_test` passes, or the PR body records any unrelated clean-main failure with a focused green command.

## Notes

Test-only unless a real isolation bug is found.
