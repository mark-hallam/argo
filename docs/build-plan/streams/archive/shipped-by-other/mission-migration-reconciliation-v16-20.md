# Mission production blocker #4b — migration reconciliation v16..v20

**Owner**: claude. **Depends on**: mission-migration-reconciliation-v12-15 MERGED (back-compat shim must exist).

> SOURCE: deep-research findings tick 198, Section 2, items #24–28.
> Sub-brief 2/4. Reuses `migration_compat::promote_legacy_marker`
> from v12-15 sub-brief. First sub-brief that handles a numeric
> collision (`19__anomaly.sql` + `19__worker_fs_root.sql`).

## Scope (this sub-brief only)

Convert the SECOND file cluster — orchestrator + circuit_history +
workflow_runs + the FIRST collision pair (anomaly +
worker_fs_root) + skill_ab_tests — into versioned modules.

Files in this cluster:

| Raw file                              | New version |
| ------------------------------------- | ----------- |
| `16__orchestrator.sql`                | `v16.rs`    |
| `17__worker_circuit_history.sql`      | `v17.rs`    |
| `18__workflow_runs.sql`               | `v18.rs`    |
| `19__anomaly.sql` (collision)         | `v19.rs`    |
| `19__worker_fs_root.sql` (collision)  | `v20.rs`    |
| `20__skill_ab_tests.sql`              | `v21_skill_ab_tests.rs` (PROVISIONAL — see note below) |

> **Renumbering note**: the raw `20__skill_ab_tests.sql` originally
> shipped with prefix 20, but because we used v20 to break the
> 19-collision tie, this file lands as v21. The v21-27 sub-brief
> picks up from v22 onward. **DO NOT** ship a v20 module that
> conflicts. Document the renumbering in `migration_compat.rs`'s
> legacy-marker map (already scaffolded in v12-15 sub-brief —
> just add entries).

## Approach

1. Read the DDL of each raw SQL file in this cluster.
2. Write `v16.rs` … `v21_skill_ab_tests.rs` modules with
   up()/down()/tests, mirroring `v11.rs`.
3. Choose stable ordering for the 19-collision pair:
   - Recommendation: `v19.rs` carries `19__anomaly.sql` (per
     git-creation order — anomaly was authored first per `git
     log`); `v20.rs` carries `19__worker_fs_root.sql`.
   - If git-creation order disagrees, document the choice in the
     module doc comment.
4. Extend `migration_compat::promote_legacy_marker`'s legacy-
   marker map to recognise databases that ran the lazy-ensure
   pattern for these tables and promote correctly.
5. Register the new modules in
   `crates/argo-store/src/migrations/mod.rs` (additive).
6. Extend `migration_walk_smoke.rs` to cover v16..v21.
7. **DO NOT** delete the raw SQL files yet.

## Files you may touch

- `crates/argo-store/src/migrations/mod.rs` — narrowly: register
  v16..v21 modules
- `crates/argo-store/src/migrations/v16.rs` (NEW)
- `crates/argo-store/src/migrations/v17.rs` (NEW)
- `crates/argo-store/src/migrations/v18.rs` (NEW)
- `crates/argo-store/src/migrations/v19.rs` (NEW)
- `crates/argo-store/src/migrations/v20.rs` (NEW)
- `crates/argo-store/src/migrations/v21_skill_ab_tests.rs` (NEW)
- `crates/argo-store/src/migration_compat.rs` — narrowly: extend
  legacy-marker map with v16..v21 entries (NO new public API)
- `crates/argo-store/src/migration_compat_tests.rs` — narrowly:
  add 4-6 tests covering the new markers
- `crates/argo-cli/tests/migration_walk_smoke.rs` — narrowly:
  extend the walk to assert v16..v21 schema correctness

## Files you must NOT touch

- v1..v15 modules (stable from previous sub-brief)
- Any raw SQL file in this range — keep them on disk
- argo-store modules that call `ensure_schema()` (those land in
  the v28-37 sub-brief)

## Acceptance criteria

- [ ] `v16.rs` … `v21_skill_ab_tests.rs` exist with up()/down()/tests
- [ ] 19-collision split documented in module doc comments
- [ ] `migration_compat::promote_legacy_marker` recognises legacy
      markers for all 6 tables in this cluster
- [ ] Fresh-DB walk via `migration_walk_smoke.rs` runs v1..v21
      cleanly; final schema matches raw SQL oracle
- [ ] `roll_back_to(v16)` works
- [ ] `migration-consolidation-test` (#811) extended for v16..v21
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-store -p argo-cli` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-mission-migration-reconciliation-v16-20`
- PR title: `store: migration reconciliation v16..v21 (mission-migration-reconciliation-v16-20)`

## Notes for the implementer

- This is the first sub-brief that handles a real collision.
  Test EXTENSIVELY against the migration-consolidation-test
  before pushing.
- The "v21 = skill_ab_tests" choice is provisional. If a cleaner
  ordering emerges from reading the raw SQL git history, document
  it and update the legacy-marker map.
- Raw SQL files in this range remain on disk; do not delete.
