# Mission load-bearing wire — `IdentityStore::update_worker` typed mutation API

**Owner**: claude. **Depends on**: `IdentityStore` exists at `crates/argo-identity/src/store.rs:90` with `create_worker`/`get_worker`/`list_workers`.
**Authored against**: origin/main `6da4af77` on `2026-05-10`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-identity-update-worker`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 1 item #39 ("Add `IdentityStore::update_worker` typed mutation API"). Today the store offers create/get/list but no typed update path. Workers can only be edited via raw SQL or full re-create. Mission Claim 1 ("real teammate") implies the worker is a mutable identity (persona evolution, role changes, last-active updates). The store should expose a typed API for the mutations callers actually need.

## Scope

Add `IdentityStore::update_worker(worker_id, updates: WorkerUpdate) -> ArgoResult<Worker>` where `WorkerUpdate` is a partial-update struct with `Option<T>` fields:

```rust
pub struct WorkerUpdate {
    pub display_name: Option<String>,
    pub persona: Option<Persona>,
    pub role_id: Option<RoleId>,
    // Add only fields that callers actually need today; resist
    // exposing every column. Future fields land additively.
}
```

Implementation: read-modify-write via existing `get_worker` then `UPDATE … WHERE id = ?` with COALESCE-style SQL so unset fields preserve current values atomically. Returns the post-update `Worker` row. Errors: `ArgoError::NotFound` if worker_id is unknown; `ArgoError::Validation` if any update violates a constraint (e.g. empty display_name).

This is NOT in scope: deleting workers (separate brief, `mvp-identity-archive-worker` already in flight); cross-tenant updates; bulk update; persona-evolution audit row (separate brief #46).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-identity/src/store.rs` — narrowly scoped: ADD `pub async fn update_worker(...)` method on `IdentityStore` impl block. Sit it next to `create_worker` / `get_worker`. NO change to existing methods.
- `crates/argo-identity/src/worker_update.rs` (NEW, ≤ 80 LOC) — `pub struct WorkerUpdate` with the partial-update fields + `Default` impl + builder helpers (`with_display_name`, `with_persona`, etc.) for ergonomic call sites.
- `crates/argo-identity/src/lib.rs` — narrowly scoped: ONE `pub mod worker_update;` line + ONE `pub use worker_update::WorkerUpdate;` re-export.
- `crates/argo-identity/src/store_tests.rs` (or `worker_update_tests.rs` sibling if a NEW tests module is preferred) — narrowly scoped: append 6 NEW unit tests: update-display-name-preserves-other-fields; update-persona-only; update-multiple-fields-atomically; update-unknown-worker-errors-NotFound; update-empty-display-name-errors-Validation; update-no-fields-set-is-noop-returns-current.

## Files you must NOT touch

- `crates/argo-store/migrations/**` — no schema change (the existing `worker` table already has the columns)
- `crates/argo-cli/handlers/worker_*.rs` — CLI wiring is separate brief
- Any vertical or skill crate
- `IdentityStore::create_worker` / `get_worker` / `list_workers` — leave existing methods alone

## Acceptance criteria

- [ ] `WorkerUpdate` struct with `Default` + builder helpers exists in `argo-identity` and is publicly re-exported
- [ ] `IdentityStore::update_worker(worker_id, updates)` exists, async, returns `ArgoResult<Worker>`
- [ ] Implementation uses single SQL `UPDATE` (NOT read-modify-write race-prone two-step) — relies on `COALESCE($field, current_value)` per column
- [ ] Returns the post-update `Worker` row by re-reading after UPDATE (one extra SELECT is fine; not a perf bottleneck for control-plane mutations)
- [ ] `ArgoError::NotFound` when worker_id unknown
- [ ] `ArgoError::Validation` for empty display_name (other validations follow `create_worker`'s rules — reuse the existing validators)
- [ ] No-op call (all fields None) returns current row without erroring
- [ ] 6 new unit tests cover the matrix
- [ ] No new public API on `IdentityStore` beyond the one new method
- [ ] `cargo build -p argo-identity` clean
- [ ] `cargo clippy -p argo-identity --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-identity` passes (skip pre-existing flakes per KNOWN_FLAKES.md)
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-identity-update-worker`
- PR title: `identity: typed IdentityStore::update_worker partial-update API (mission-identity-update-worker)`

## Notes for the implementer

- Use `WorkerUpdate { ..Default::default() }` style at call sites — that's the Rust ergonomic pattern for partial updates and matches the existing builder convention in this crate.
- The COALESCE pattern in SQLite: `UPDATE worker SET display_name = COALESCE(?, display_name), persona = COALESCE(?, persona) WHERE id = ?`. Bind `Option<T>` directly via sqlx — `Some(v)` becomes the new value, `None` preserves current.
- For `Persona` and `RoleId` (which are likely complex types, not strings), confirm the existing serialization in `create_worker` first 5 minutes; reuse the same path.
- DO NOT change row updated_at semantics if the table has one — let the existing trigger or default handle it. If no trigger exists and the brief `mission-identity-archive-worker` has already added one, coordinate.
- The 6 unit tests should reuse the test fixtures from `store_tests.rs` (look for `seed_test_worker` or similar). Don't hand-roll new fixtures.
