# Mission production blocker #4c — migration reconciliation v22..v33

**Owner**: claude. **Depends on**: mission-migration-reconciliation-v16-20 MERGED.

> SOURCE: deep-research findings tick 198, Section 2, items #24–28.
> Sub-brief 3/4. Handles the bulk of the colliding numeric prefixes
> (22, 23, 24, 26, 27).

## Scope (this sub-brief only)

Convert the THIRD file cluster — five collision groups +
single-prefix entries between them — into versioned modules.
This is the densest collision cluster of the four sub-briefs.

Files in this cluster:

| Raw file                                  | New version  |
| ----------------------------------------- | ------------ |
| `21__org_chart.sql`                       | `v22.rs`     |
| `22__roster.sql` (collision)              | `v23.rs`     |
| `22__tenant_archived.sql` (collision)     | `v24.rs`     |
| `23__handoff_routing.sql` (collision)     | `v25.rs`     |
| `23__workflow_checkpoints.sql` (collision)| `v26.rs`     |
| `24__operator_feedback.sql` (collision)   | `v27.rs`     |
| `24__skill_quality_score.sql` (collision) | `v28.rs`     |
| `24__vertical_archived.sql` (collision)   | `v29.rs`     |
| `25__policy_files.sql`                    | `v30.rs`     |
| `26__data_classification.sql` (collision) | `v31.rs`     |
| `26__spend_baseline.sql` (collision)      | `v32.rs`     |
| `26__worker_budget_alert.sql` (collision) | `v33.rs`     |

The remaining 27/28/29-prefix files (handoff_auctions,
spend_anomaly, recovery_cache, role_duties, tenant_quota,
subtask_ledger) land in the v28-37 sub-brief.

## Approach

1. Read each raw SQL file's DDL.
2. Write `v22.rs` … `v33.rs` modules with up()/down()/tests
   following `v11.rs` shape.
3. For each collision group (22 / 23 / 24 / 26), order the
   children by git-log creation timestamp (oldest first) — see
   table above for the recommended assignment.
4. Extend `migration_compat::promote_legacy_marker`'s legacy-
   marker map with all 12 new entries; ensure idempotency on
   re-run.
5. Register the new modules in
   `crates/argo-store/src/migrations/mod.rs` (additive).
6. Extend `migration_walk_smoke.rs` to cover v22..v33.
7. **DO NOT** delete the raw SQL files yet.

## Files you may touch

- `crates/argo-store/src/migrations/mod.rs` — narrowly: register
  v22..v33 modules
- `crates/argo-store/src/migrations/v22.rs` (NEW) — `21__org_chart.sql`
- `crates/argo-store/src/migrations/v23.rs` (NEW) — `22__roster.sql`
- `crates/argo-store/src/migrations/v24.rs` (NEW) — `22__tenant_archived.sql`
- `crates/argo-store/src/migrations/v25.rs` (NEW) — `23__handoff_routing.sql`
- `crates/argo-store/src/migrations/v26.rs` (NEW) — `23__workflow_checkpoints.sql`
- `crates/argo-store/src/migrations/v27.rs` (NEW) — `24__operator_feedback.sql`
- `crates/argo-store/src/migrations/v28.rs` (NEW) — `24__skill_quality_score.sql`
- `crates/argo-store/src/migrations/v29.rs` (NEW) — `24__vertical_archived.sql`
- `crates/argo-store/src/migrations/v30.rs` (NEW) — `25__policy_files.sql`
- `crates/argo-store/src/migrations/v31.rs` (NEW) — `26__data_classification.sql`
- `crates/argo-store/src/migrations/v32.rs` (NEW) — `26__spend_baseline.sql`
- `crates/argo-store/src/migrations/v33.rs` (NEW) — `26__worker_budget_alert.sql`
- `crates/argo-store/src/migration_compat.rs` — narrowly: extend
  legacy-marker map with the 12 new entries
- `crates/argo-store/src/migration_compat_tests.rs` — narrowly:
  add tests covering each collision group's promotion path
- `crates/argo-cli/tests/migration_walk_smoke.rs` — narrowly:
  extend the walk to v22..v33 schema correctness

## Files you must NOT touch

- v1..v21 modules (stable from prior sub-briefs)
- Any raw SQL file in this range — keep them on disk
- argo-store modules calling `ensure_schema()` (those land in
  the v28-37 sub-brief — they remain functional during this
  sub-brief because the raw SQL files still exist)
- `27*`/`28*`/`29*` raw SQL files (those land in next sub-brief)

## Acceptance criteria

- [ ] `v22.rs` … `v33.rs` exist with up()/down()/tests
- [ ] All 4 collision groups (22, 23, 24, 26) split with documented
      ordering rationale in module doc comments
- [ ] `migration_compat::promote_legacy_marker` recognises legacy
      markers for all 12 tables; promotion is idempotent on re-run
- [ ] Fresh-DB walk via `migration_walk_smoke.rs` runs v1..v33
      cleanly; final schema matches raw SQL oracle for the bundle
- [ ] `roll_back_to(v22)` works
- [ ] `migration-consolidation-test` (#811) extended for v22..v33
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-store -p argo-cli` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-mission-migration-reconciliation-v21-27`
- PR title: `store: migration reconciliation v22..v33 (mission-migration-reconciliation-v21-27)`

## Notes for the implementer

- This is the densest sub-brief — 12 new modules + 4 collision
  resolutions. Take one collision group at a time, run the walk
  smoke after each, and only proceed when GREEN.
- The git-log-creation-order rule is the tiebreaker for collisions.
  If git history is ambiguous (e.g. two files committed in the
  same commit), prefer alphabetical order and document the choice.
- Raw SQL files in this range remain on disk through this sub-brief.
  ensure_schema() callers continue to work — the v28-37 sub-brief
  is what severs that dependency.
