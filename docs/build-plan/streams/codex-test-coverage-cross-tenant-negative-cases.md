# codex-test-coverage-cross-tenant-negative-cases

**Authored against**: `origin/main` `652a1825` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-test-coverage-cross-tenant-negative-cases`.

**Fast-mode eligible**: yes — mechanical audit + additive test cases.

## Summary

Mission claim 6 (multi-channel approval / multi-tenant) hinges on cross-tenant data isolation. Every gateway integration test SHOULD include at least one negative case: a request from tenant B that touches tenant A's data must 403. Audit the existing gateway tests for cross-tenant negative coverage; fill the gaps.

#1702 set the precedent (cross-tenant negative test for skill-library); #1901 closed the cross-tenant audit gate. This brief makes negative-case coverage uniform.

## Acceptance

- [ ] Enumerate every gateway integration test file (typically `crates/argo-gateway/tests/routes_*.rs` and `crates/argo-cli/src/handlers/gateway/routes_*_tests.rs`).
- [ ] For each test file, classify each `#[test]` / `#[tokio::test]` function:
  - **HAS-NEGATIVE** — already includes a cross-tenant 403/404 assertion.
  - **NO-NEGATIVE** — pure happy-path or single-tenant test → flag for fill-in.
  - **NOT-MULTI-TENANT** — tests a non-tenant-scoped surface (e.g. health check) → skip.
- [ ] PR body includes a markdown table: file | total tests | has-negative | no-negative | not-multi-tenant.
- [ ] For each NO-NEGATIVE row where the surface IS multi-tenant: add a minimal cross-tenant negative case (request from tenant_b → assert 403 or 404). Use the canonical fixture pattern from #1702 / #1901.
- [ ] No semantic change to existing tests. Pure addition.
- [ ] Coverage report ends in PR body: before/after counts.

## May touch

- Test files under `crates/argo-gateway/tests/` and `crates/argo-cli/src/handlers/gateway/*_tests.rs`.
- Shared test fixture helpers if a new helper is genuinely needed (additive only).

## Must NOT touch

- Production code (gateway handlers, RBAC enforcement). If the audit reveals a missing cross-tenant guard in production code, spawn a follow-up brief.
- Non-gateway tests (runner, store, audit) — that's a different codex sweep.

## Notes

- Reference patterns: #1702 (`web-rbac-cross-tenant-negative-test`), #1901 (`web-rbac-jwt-role-claim-and-gateway-enforce`).
- Pre-flight: read 2-3 existing tests with cross-tenant assertions first to lock in the idiomatic shape.
- Sizing: 200–500 LOC. Most tests will need a 3–10-line negative variant; the bulk is in count.
- Procurement-grade leverage — this is what audit-time reviewers look for.
