# mission-worker-archive-cascade-tenant-pinned

**Owner**: claude.
**Depends on**: nothing.
**Authored against**: origin/main `3df52142a` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-worker-archive-cascade-tenant-pinned`.

🟡 **P2 LATENT-LEAK + doc-comment-stronger-than-SQL.** The
`worker_archive::archive_worker` transaction cascades three DELETEs
that key on `worker_id` alone, even though those tables carry
`tenant_id`:

- `worker_archive.rs:133` — `DELETE FROM duty WHERE worker_id = ?`
- `worker_archive.rs:138` — `DELETE FROM role WHERE worker_id = ?`
- `worker_archive.rs:143` — `DELETE FROM persona WHERE worker_id = ?`

The fourth `worker` DELETE at line 148 DOES bind tenant_id, and the
doc-comment at line 251 claims "Cascade scope (per-tenant only —
never crosses tenants)." The SQL is weaker than the doc.

## Why

Re-verification pass in
`eval-argo-store-tenant-bind-sweep-remaining-tables` §3.3 caught
PR #2116 §2.1's misclassification of these rows as 🟢 BIND. The
audit re-read the literal SQL and the cascade is **not**
tenant-bound. UUID-unique `worker_id` covers practical attack but
the SQL should match the doc-comment's guarantee.

## Scope

- Add `tenant_id` bind to the duty/role/persona DELETEs:
  `DELETE FROM <table> WHERE worker_id = ? AND tenant_id = ?`.
- Same change in `delete_with_grace` if it has analogous cascade
  DELETEs (audit at claim-time).
- No new signature change required (`archive_worker` already takes
  `tenant_id`).
- Pin test: synthetic UUID-clash scenario.
- ~80-150 LOC.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-store/src/worker_archive.rs`
- `crates/argo-store/tests/worker_archive_cascade_tenant_pin.rs` (new)
- `docs/build-plan/BUILD_STATUS.md`

## Acceptance criteria

- [ ] All three cascade DELETEs bind `tenant_id` in SQL.
- [ ] Pin test passes.
- [ ] `cargo clippy -p argo-store --all-targets -- -D warnings` clean.
- [ ] Every modified file < 500 LOC.
