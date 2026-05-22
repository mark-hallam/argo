# Mission production blocker #4d — migration reconciliation v34..v40 + ensure_schema removal

**Owner**: claude. **Depends on**: mission-migration-reconciliation-v21-27 MERGED.

> SOURCE: deep-research findings tick 198, Section 2, items #24–28.
> Sub-brief 4/4 — the closer. Converts the final raw SQL files,
> deletes ALL raw SQL once verified, and removes lazy
> `ensure_schema()` calls from argo-store + argo-cli modules.

## Scope (this sub-brief only)

The closer. Three things in one PR (or split into 2 PRs at
implementer's discretion — see notes):

1. Convert the FINAL file cluster (27/28/29 prefixes) into
   versioned modules.
2. Delete ALL raw SQL files (12..29) now that the full
   v12..v40 ladder exists.
3. Remove lazy `ensure_schema()` calls from argo-store modules
   AND from `argo-cli/src/handlers/worker_create.rs`
   (`FS_ROOT_SCHEMA` + `IDENTITY_SCHEMA` `include_str!` paths).

Files in this cluster:

| Raw file                              | New version  |
| ------------------------------------- | ------------ |
| `27__handoff_auctions.sql` (collision)| `v34.rs`     |
| `27__spend_anomaly.sql` (collision)   | `v35.rs`     |
| `28__recovery_cache.sql` (collision)  | `v36.rs`     |
| `28__role_duties.sql` (collision)     | `v37.rs`     |
| `28__tenant_quota.sql` (collision)    | `v38.rs`     |
| `29__subtask_ledger.sql`              | `v39.rs`     |
| (header reserved for future use)      | `v40.rs` (no-op tombstone — safety margin so future raw SQL drops don't accidentally collide; document) |

## Approach

1. Read DDL of each remaining raw SQL file; write `v34.rs` …
   `v39.rs` modules following `v11.rs` shape.
2. Add `v40.rs` as a documented no-op tombstone (`up()` returns
   `Ok(())`, `down()` ditto) with a doc comment explaining why.
3. Extend `migration_compat::promote_legacy_marker`'s legacy-
   marker map with the final 6 entries. After this sub-brief,
   the map is complete.
4. **Delete** ALL raw SQL files in `crates/argo-store/migrations/`
   once `migration_walk_smoke.rs` is GREEN against the full
   v1..v40 ladder.
5. **Remove** lazy `ensure_schema()` calls from these argo-store
   modules:
   - `crates/argo-store/src/anomaly.rs`
   - `crates/argo-store/src/classification.rs`
   - `crates/argo-store/src/feedback.rs`
   - `crates/argo-store/src/recovery_cache.rs`
   - `crates/argo-store/src/spend_anomaly.rs`
   - `crates/argo-store/src/subtask.rs`
   - `crates/argo-store/src/tenant_quota.rs`
   The migration ladder now guarantees the schema; these calls
   become dead code. Replace with an `assert_schema_at_least(vN)`
   debug-only check (or just delete; implementer's call).
6. **Remove** the `FS_ROOT_SCHEMA` + `IDENTITY_SCHEMA`
   `include_str!` blocks from
   `crates/argo-cli/src/handlers/worker_create.rs`. The DDL is
   now permanently owned by `v12.rs` (worker_identity) and
   `v20.rs` (worker_fs_root).
7. Update `migrate_down_to(v12)` to work end-to-end (smoke test).
8. Extend `migration_walk_smoke.rs` for v34..v40 + add a final
   "no raw SQL on disk" assertion.

## Files you may touch

- `crates/argo-store/src/migrations/mod.rs` — narrowly: register
  v34..v40 modules
- `crates/argo-store/src/migrations/v34.rs` (NEW) — `27__handoff_auctions.sql`
- `crates/argo-store/src/migrations/v35.rs` (NEW) — `27__spend_anomaly.sql`
- `crates/argo-store/src/migrations/v36.rs` (NEW) — `28__recovery_cache.sql`
- `crates/argo-store/src/migrations/v37.rs` (NEW) — `28__role_duties.sql`
- `crates/argo-store/src/migrations/v38.rs` (NEW) — `28__tenant_quota.sql`
- `crates/argo-store/src/migrations/v39.rs` (NEW) — `29__subtask_ledger.sql`
- `crates/argo-store/src/migrations/v40.rs` (NEW) — tombstone
- `crates/argo-store/src/migration_compat.rs` — narrowly: final 6
  legacy-marker entries
- `crates/argo-store/src/migration_compat_tests.rs` — narrowly:
  6 more tests + the "promotion-complete" test
- `crates/argo-store/migrations/*.sql` — DELETE ALL after walk smoke is GREEN
- `crates/argo-store/src/anomaly.rs` — narrowly: remove `ensure_schema()`
- `crates/argo-store/src/classification.rs` — narrowly: remove `ensure_schema()`
- `crates/argo-store/src/feedback.rs` — narrowly: remove `ensure_schema()`
- `crates/argo-store/src/recovery_cache.rs` — narrowly: remove `ensure_schema()`
- `crates/argo-store/src/spend_anomaly.rs` — narrowly: remove `ensure_schema()`
- `crates/argo-store/src/subtask.rs` — narrowly: remove `ensure_schema()`
- `crates/argo-store/src/tenant_quota.rs` — narrowly: remove `ensure_schema()`
- `crates/argo-cli/src/handlers/worker_create.rs` — narrowly:
  remove `FS_ROOT_SCHEMA` + `IDENTITY_SCHEMA` `include_str!` blocks
- `crates/argo-cli/tests/migration_walk_smoke.rs` — narrowly:
  extend to v40 + add "no raw SQL on disk" assertion

## Files you must NOT touch

- v1..v33 modules (stable from prior sub-briefs)
- `migration-consolidation-test` (#811) — extend if needed but
  do not delete

## Acceptance criteria

- [ ] `v34.rs` … `v40.rs` exist with up()/down()/tests
- [ ] ALL raw SQL files in `crates/argo-store/migrations/` deleted
- [ ] `ensure_schema()` calls removed from the 7 argo-store modules
      listed; `cargo check` clean
- [ ] `FS_ROOT_SCHEMA` + `IDENTITY_SCHEMA` removed from
      `worker_create.rs`; the DDL lives only in `v12.rs` + `v20.rs`
- [ ] `roll_back_to(v12)` works end-to-end on a database that ran
      the full v1..v40 ladder
- [ ] `migration_compat::promote_legacy_marker` map is complete;
      promotion is idempotent on re-run
- [ ] Fresh-DB walk via `migration_walk_smoke.rs` runs v1..v40
      cleanly; final schema matches the snapshot raw SQL oracle;
      "no raw SQL on disk" assertion passes
- [ ] Existing `migration-consolidation-test` (#811) extended for
      the full v12..v40 range
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-store -p argo-cli` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-mission-migration-reconciliation-v28-37`
- PR title: `store: migration reconciliation v34..v40 + remove lazy ensure_schema (mission-migration-reconciliation-v28-37)`

## Notes for the implementer

- This sub-brief is allowed to ship as 2 PRs if cleaner:
  PR-A: v34..v40 modules + raw SQL deletion + walk smoke green
  PR-B: ensure_schema removal across argo-store + worker_create
  Default: single PR. Operator preference welcome.
- The "no raw SQL on disk" assertion is a hard tripwire that
  prevents future regressions — keep it permanent.
- After this sub-brief lands, the v12..v40 ladder is the SOLE
  source of schema truth. The migration system invariant promised
  by the original brief is fully restored.
- Any failure in this sub-brief should NOT roll back the prior 3
  sub-briefs — the v12..v33 ladder is independently valuable
  even without `ensure_schema` removal.
