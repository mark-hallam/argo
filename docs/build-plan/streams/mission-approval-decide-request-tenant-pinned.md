# mission-approval-decide-request-tenant-pinned

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `a3a881fa3` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-approval-decide-request-tenant-pinned`.

🔴 **P0 CLAIM-5/6 LATENT-LEAK.** `approvals::decide_request` filters on `(id, status)` with NO `tenant_id` bind. Same shape as the audit_detail bug fixed in #1992. Currently safe because no production caller wires the helper — but the leak materialises the moment a route handler does.

## Why

PR #2116 (eval-argo-store-tenant-bind-sweep, MERGED) §4 finding 1 names this as the single most-load-bearing finding from a 77-table sweep. The brief is the named follow-up.

## Scope

- Add `tenant_id` parameter to `approvals::decide_request` SQL + Rust signature.
- Add cross-tenant negative test: tenant_a creates approval, tenant_b calls `decide_request` with tenant_b → expect zero rows affected / NotFound error.
- Audit any (likely zero) callsites and pass tenant through.
- Migration if needed (likely additive: backfill `tenant_id` on `approval_request` if missing from PK / WHERE coverage).
- ~150-250 LOC.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-store/src/approvals/` (or wherever `decide_request` lives — grep at claim-time)
- `crates/argo-store/tests/approval_decide_tenant_bind_pin.rs` (already authored in #2116 as #[ignore]'d FINDINGS-ONLY — un-ignore on un-block)
- `crates/argo-store/src/migrations/` if column-add needed
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- Other crates outside the may-touch list.

## Acceptance criteria

- [ ] `decide_request` rejects cross-tenant calls at SQL level.
- [ ] Existing #[ignore]'d pin test in `approval_decide_tenant_bind_pin.rs` un-ignored + passing.
- [ ] New cross-tenant negative test passing.
- [ ] `cargo clippy -p argo-store --all-targets -- -D warnings` clean.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.
