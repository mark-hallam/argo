# mvp-identity-archive-worker — IdentityStore::archive_worker + archived filtering

**Owner**: claude. **Depends on**: nothing. **Tier P (production-readiness, MVP).**

**Authored against**: origin/main 5baf15a6 on 2026-05-10.

## Why

MISSION_BACKLOG items #39 + #40 call for typed `update_worker` /
`archive_worker` / `delete_worker` mutations on `IdentityStore`.
Today there is no typed archive path: an operator who wants to retire
a worker has to either DELETE (loses audit history) or hand-edit the
table. Soft-delete via `archived_at` timestamp lets us keep history
while removing the worker from default lookups.

This brief covers ONLY archive (the lighter half — soft-delete is
preferable to hard-delete for v1 anyway). Hard-delete (#40 second
half) can land in a follow-up.

## Approach (concrete)

1. **Migration v47 / raw `47__worker_archived_at.sql`.** The next
   free slot is 47 (39-46 occupied with multiple competing v45
   variants on main as of 5baf15a6). Add a nullable
   `archived_at TEXT` column to the `worker` table. Backfill is
   trivial (NULL = active).
2. **Versioned migration:** add `crates/argo-store/src/migrations/v47.rs`
   following the pattern of the most recent existing `vNN.rs` file
   (read `v46.rs` for shape — match arm in `mod.rs`, schema-tip
   assertion bumps in `lib_tests.rs` + `migration_reconciliation_tests.rs`
   under the EXHAUSTIVE-MATCH amendment).
3. **`IdentityStore::archive_worker(worker_id) -> Result<()>`** in
   `crates/argo-identity/src/lib.rs` (currently 67 LOC) or a sibling
   if `lib.rs` would cross 500 LOC. Sets `archived_at = now()` via
   UPDATE-WHERE-archived_at-IS-NULL (idempotent; second call is a
   no-op).
4. **Filter active workers by default.** Update `IdentityStore::list_workers`
   (or whatever the canonical list API is named) to add
   `WHERE archived_at IS NULL` to its query. Add an
   `include_archived: bool` parameter to the same method (additive,
   default false in callers — find every caller and pass `false`
   explicitly under EXHAUSTIVE-MATCH).
5. **5 unit tests** in `crates/argo-identity/src/`:
   - `archive_worker_sets_timestamp`
   - `archive_worker_is_idempotent` (second call is a no-op, still Ok)
   - `archive_worker_unknown_id_returns_not_found` (use
     `ArgoError::Provider` — `NotFound` doesn't exist per tick418
     observed deviation)
   - `list_workers_excludes_archived_by_default`
   - `list_workers_include_archived_returns_all`

## Files you may touch

- `crates/argo-store/migrations/47__worker_archived_at.sql` (NEW)
- `crates/argo-store/src/migrations/v47.rs` (NEW, ≤ 80 LOC)
- `crates/argo-store/src/migrations/mod.rs` (1 match arm, 1 mod decl)
- `crates/argo-store/src/lib_tests.rs` + `crates/argo-store/src/migration_reconciliation_tests.rs` (schema-tip 46 → 47)
- `crates/argo-identity/src/lib.rs` (new method + list filter)
- `crates/argo-identity/src/<sibling>.rs` (NEW, only if `lib.rs` would
  cross 500 LOC)
- Any callsite of `list_workers` workspace-wide, ONLY to pass the new
  `include_archived: false` argument (EXHAUSTIVE-MATCH amendment)

## Files you must NOT touch

- Any unrelated migration file
- Any other identity field / API
- Any vertical manifest

## Acceptance criteria

- [ ] Migration v47 applies cleanly on a fresh DB AND a v46-state DB.
- [ ] `archive_worker` sets `archived_at` and is idempotent.
- [ ] `list_workers` excludes archived by default; `include_archived=true`
      returns all.
- [ ] 5 new unit tests pass.
- [ ] `cargo clippy -p argo-store -p argo-identity --all-targets --
      -D warnings` clean.
- [ ] `cargo test -p argo-store -p argo-identity` green.
- [ ] No file crosses 500 LOC.

## Branch + PR

- Branch: `claude-mvp-identity-archive-worker`
- PR title: `identity: archive_worker mutation + soft-delete filtering (mvp-identity-archive-worker)`
