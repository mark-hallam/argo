# mission-cross-tenant-test-audit-export

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `917290b98` on `2026-05-22`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-cross-tenant-test-audit-export`.

🟡 **P2 SECURITY HYGIENE.** Source: `mission-cross-tenant-audit-sweep` (this PR) — `POST /api/v1/audit/export` is the **top-1 data-exposure risk** in the cross-tenant coverage report at `docs/skycot/audit/cross-tenant-endpoint-coverage.md`. One leak here returns a signed download URL for a tarball / NDJSON / CSV bundle covering an arbitrary `[from, to]` window — i.e. potentially the entire audit log of another tenant. The sibling test file `routes_audit_export_tests.rs` covers the happy path (build → sign → download) but has NO cross-tenant negative test.

## Scope

Add three cross-tenant negative tests to `routes_audit_export_tests.rs`:

1. **POST /api/v1/audit/export tenant scoping.** Seed audit rows under
   tenant-A and tenant-B over the same time window. Tenant-A caller
   POSTs the export. The returned manifest's row count + bundle bytes
   contain ZERO tenant-B rows.
2. **POST /api/v1/audit/export/preview tenant scoping.** Same fixture;
   tenant-B caller previews; preview count + sample rows are tenant-B
   only.
3. **GET /api/v1/audit/export/<bundle_id>?sig=… cross-tenant fetch.**
   Tenant-A builds a bundle and gets a signed URL. Tenant-B caller
   attempts to fetch that URL — must return 403 / 404 (whichever the
   handler currently emits) without delivering bytes. The HMAC alone
   is not enough; the bundle's owner tenant must also be checked
   server-side.

If test 3 reveals the current handler ONLY checks the HMAC (the
`ARGO_AUDIT_EXPORT_SECRET` is process-global), this brief STOPS at the
failing test and posts a one-line BLOCKED note — the production fix
(per-tenant binding on the signed payload, or a per-tenant secret) is
out of scope and needs its own brief. Document the gap in the PR body
under "Findings" so the conductor can replenish.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_audit_export_tests.rs` — three new `#[tokio::test]` fns (additive only; do not edit existing tests).
- `crates/argo-cli/src/handlers/gateway/routes_audit_export.rs` — ONLY if test 3 surfaces a real cross-tenant bug AND the fix is ≤30 LOC + does not touch other endpoints. Otherwise: stop the brief, document the finding, and let the conductor queue a fix-brief.

## Files you must NOT touch

- Any other handler under `crates/argo-cli/src/handlers/gateway/`.
- `argo-audit` crate internals (`export_bundle.rs`, `AuditSink`).
- The on-disk bundle layout / signing scheme.
- `docs/skycot/audit/cross-tenant-endpoint-coverage.md` (the audit
  report's authoritative version is the one from
  `mission-cross-tenant-audit-sweep` — re-running the script in this
  brief is fine; rewriting the report is not).
- BUILD_STATUS.md outside the standard row flip.

## Acceptance criteria

- [ ] Three new `#[tokio::test]` fns in `routes_audit_export_tests.rs`, each named with the `cross_tenant_` prefix per the codebase convention (`tenant_scope_default_excludes_other_tenants` etc.).
- [ ] Tests use the existing `mem_store()` + `seed()` helpers; do not introduce a new fixture pattern.
- [ ] Tenant ids are `"tenant-a"` / `"tenant-b"` (literal, matches the heuristic in `scripts/audit-cross-tenant-coverage.sh`).
- [ ] `cargo test -p argo-cli routes_audit_export` passes (including new fns).
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.
- [ ] If test 3 surfaces a real cross-tenant fetch bug (HMAC-only path), brief is BLOCKED with a one-line finding in the BUILD_STATUS row; PR body's "Findings" section names the missing tenant-binding check; no production code edited.
- [ ] If no bug surfaces (handler already binds tenant), all three tests pass green and the report's top-1 row in `docs/skycot/audit/cross-tenant-endpoint-coverage.md` can be moved to the ✅ list by the next replenishment commit (NOT this brief).
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-mission-cross-tenant-test-audit-export`
- PR title: `test(gateway): cross-tenant negative for POST /api/v1/audit/export (mission-cross-tenant-test-audit-export)`

## Notes for the implementer

- ~200-300 LOC expected (mostly the three test fns + fixture seeds).
- Read `routes_audit_feed_tenant_tests.rs` first — that's the canonical
  pattern for tenant-scoped tests in this handler area. Mirror its
  helpers (`seed_row_for_tenant`, `parse_body`, `get_with_tenant`)
  rather than rolling your own.
- The audit-export signing secret is process-global
  (`ARGO_AUDIT_EXPORT_SECRET` env, default fallback for dev). Test 3
  is the one that exposes whether tenant-binding is enforced
  server-side OR if HMAC-alone is the only gate. Run the test BEFORE
  reasoning about the answer — the code is the source of truth.
- This is a TEST brief. Do NOT add a real fix mid-flight unless
  trivially scoped. Per the parent audit brief's pattern, fixes ship
  as their own follow-ups.
- Drift-prevention: no new pub types; no new files outside the
  may-touch list.
