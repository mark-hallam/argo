# Mission production blocker #4a — migration reconciliation v12..v15 + back-compat shim

**Owner**: claude. **Depends on**: none. **Sub-brief of**: mission-migration-reconciliation (was BLOCKED on may-touch under-scoping; split into 4 file-cluster sub-briefs).

> SOURCE: deep-research findings tick 198, Section 2, items #24–28.
> Two parallel migration systems exist. The original brief
> (`mission-migration-reconciliation.md`) is the architectural
> overview; this sub-brief carries the FIRST file cluster + the
> shared back-compat shim scaffolding that all later sub-briefs
> reuse.

## Scope (this sub-brief only)

Convert the FIRST four raw SQL migration files into versioned
`vN.rs` modules following the existing v1–v11 template, AND lay
down the shared back-compat shim in `migration_compat.rs` that
every subsequent sub-brief (v16-20, v21-27, v28-37) imports.

Files in this cluster:

| Raw file                          | New version |
| --------------------------------- | ----------- |
| `12__worker_identity.sql`         | `v12.rs`    |
| `13__memory_layers.sql`           | `v13.rs`    |
| `14__worker_budget.sql`           | `v14.rs`    |
| `15__shared_team_memory.sql`      | `v15.rs`    |

No collisions in this range — straight 1:1 promotion.

## Approach

1. For each of the 4 raw SQL files, read the DDL and write a
   `vN.rs` module with up()/down()/tests, mirroring the existing
   `v11.rs` shape exactly.
2. Register `v12..v15` in
   `crates/argo-store/src/migrations/mod.rs` (additive — no
   reorder of v1..v11).
3. **NEW** `crates/argo-store/src/migration_compat.rs`:
   - `pub fn promote_legacy_marker(pool: &SqlitePool) -> ArgoResult<()>` —
     reads `schema_version`; if any row reflects a legacy "raw SQL"
     applied state (signaled by the lazy-ensure pattern, e.g. an
     application module having called `ensure_schema()` and stored
     a digest row keyed by table name), atomically rewrites it to
     the new monotonic versioned state.
   - Recognises markers from ALL clusters (v12 through v37 — i.e.
     the shim is forward-compatible with later sub-briefs landing).
   - 4-6 unit tests covering: clean DB (no-op), legacy-marker DB
     (promote), partial-state DB (idempotent), unknown marker
     (error with audit log).
4. Wire the shim into `migrations::mod::run_pending` so it fires
   BEFORE the v12+ pending list is computed.
5. **DO NOT** delete the raw SQL files yet — they continue to work
   as a fallback path until v28-37 sub-brief lands.

## Files you may touch

- `crates/argo-store/src/migrations/mod.rs` — narrowly: register
  v12..v15; call `migration_compat::promote_legacy_marker` at top
  of `run_pending`
- `crates/argo-store/src/migrations/v12.rs` (NEW)
- `crates/argo-store/src/migrations/v13.rs` (NEW)
- `crates/argo-store/src/migrations/v14.rs` (NEW)
- `crates/argo-store/src/migrations/v15.rs` (NEW)
- `crates/argo-store/src/migration_compat.rs` (NEW; ≤ 320 LOC)
- `crates/argo-store/src/migration_compat_tests.rs` (NEW; ≤ 250 LOC)
- `crates/argo-store/src/lib.rs` — narrowly: `mod migration_compat;`
- `crates/argo-cli/tests/migration_walk_smoke.rs` (NEW; ≤ 200 LOC) —
  fresh-DB walk for the v12..v15 prefix only; later sub-briefs
  EXTEND this same test file (additive)

## Files you must NOT touch

- v1–v11 modules
- Any raw SQL file in this range — KEEP them on disk; the v28-37
  sub-brief deletes them after the full conversion is verified
- `crates/argo-store/migrations/16*.sql` and onward — those land
  in later sub-briefs

## Acceptance criteria

- [ ] `v12.rs` … `v15.rs` exist with up()/down()/tests mirroring
      v11.rs structure
- [ ] `migration_compat::promote_legacy_marker` works for clean
      DB + legacy-marker DB + partial-state DB
- [ ] Fresh-DB walk via `migration_walk_smoke.rs` runs v1..v15
      cleanly; final schema matches what the raw 12..15 SQL files
      would have produced (compared via `.schema` oracle)
- [ ] `roll_back_to(v12)` works (rollback paths defined for v12..v15)
- [ ] Existing `migration-consolidation-test` (#811) extended to
      cover v12..v15
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-store -p argo-cli` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-mission-migration-reconciliation-v12-15`
- PR title: `store: migration reconciliation v12..v15 + back-compat shim (mission-migration-reconciliation-v12-15)`

## Notes for the implementer

- The back-compat shim is the load-bearing piece for the entire
  4-sub-brief series. Test it extensively before asserting GREEN.
- Raw SQL files stay on disk through this sub-brief and the next
  two — `ensure_schema()` callers continue to work. Only the
  v28-37 sub-brief removes them.
- This is the smallest sub-brief in the cluster; treat it as the
  proof-of-pattern. v16-20 and v21-27 follow the same shape but
  with collision-handling.
