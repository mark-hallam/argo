# mvp-audit-principal-stamp — stamp principal_id on every audit row

**Owner**: claude. **Depends on**: nothing. **Tier 1 MVP — closes mission claim "everything is auditable".**

**Authored against**: origin/main 7fdbbdb8 on 2026-05-10.

## Why

Audit rows currently lack `principal_id`. Without it, multi-tenant deployments can't attribute writes to a caller — failing the mission claim "everything is auditable". Bearer-auth landed in #946 (`mission-gateway-bearer-auth`); this brief plumbs the resolved principal through the audit sink.

## Approach (concrete)

1. **Schema**: add `principal_id TEXT NULL` to `audit_record` via a new migration `46__audit_principal.sql` (or next free slot — verify `ls crates/argo-store/migrations/ | tail`). Versioned wrapper `crates/argo-store/src/migrations/v46.rs`.
2. **Type**: extend `AuditRecord` (in `crates/argo-store/src/models.rs`) and the `AuditSink::write` trait (`crates/argo-audit/src/sink.rs`) to carry `principal_id: Option<String>`. NULL is allowed for system-internal writes.
3. **Resolver**: in `crates/argo-cli/src/handlers/gateway/routes_dispatch_post.rs` (where bearer-auth runs), read the resolved `Principal` from request context and pass `principal_id` into every audit-row emission downstream of bearer auth.
4. **Plumbing**: `argo-orchestrator`, `argo-skill`, etc. write audit rows via `AuditSink::write`. They must accept and forward `principal_id` from caller context. Use a `Default` impl on `AuditRecord` to avoid breaking call sites that don't have a principal (system internals).
5. **Tests**:
   - 1 unit test in `argo-audit`: write with `Some(principal_id)`, read back, assert column populated.
   - 1 integration smoke in `crates/argo-cli/tests/`: POST with bearer token → assert audit row has matching `principal_id`.

## Files you may touch

- `crates/argo-store/migrations/46__audit_principal.sql` (NEW)
- `crates/argo-store/src/migrations/v46.rs` (NEW, ≤ 80 LOC)
- `crates/argo-store/src/migrations/mod.rs` — narrowly: register v46
- `crates/argo-store/src/lib_tests.rs` — narrowly: schema-tip assertion 45 → 46
- `crates/argo-store/src/migration_reconciliation_tests.rs` — narrowly: bump assertions 45 → 46
- `crates/argo-store/src/models.rs` — narrowly: add field to `AuditRecord`
- `crates/argo-audit/src/sink.rs` — narrowly: extend `AuditSink::write` signature
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_post.rs` — narrowly: principal threading
- `crates/argo-cli/tests/audit_principal_smoke.rs` (NEW, ≤ 200 LOC)
- Any audit-write call site that needs the new arg (EXHAUSTIVE-MATCH amendment applies — only the new arg, no other edits)

## Files you must NOT touch

- Any other migration file
- `crates/argo-store/src/audit_record.rs` core impl beyond field addition
- Any vertical manifest

## Acceptance criteria

- [ ] Migration v46 lands cleanly and schema_version reaches 46
- [ ] `AuditSink::write` carries `principal_id: Option<String>`
- [ ] Bearer-auth POST writes the principal into the audit row (verified by smoke)
- [ ] System-internal writes that pass `None` work unchanged
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-store -p argo-audit -p argo-cli` clean (Boundary OK for HARD RULE 2 flakes)

## Branch + PR

- Branch: `mvp-audit-principal-stamp`
- PR title: `audit: stamp principal_id on every record (mvp-audit-principal-stamp)`
