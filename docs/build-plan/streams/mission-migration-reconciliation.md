# Mission production blocker #4 — reconcile two migration systems

**Owner**: claude. **Depends on**: none.

> SOURCE: deep-research findings tick 198, Section 2, items #24–28.
> Two parallel migration systems exist:
>
> - Versioned `v1.rs`–`v11.rs` modules in `argo-store/src/migrations/`
>   with proper transactional up()/down() semantics
> - Raw SQL files numbered 12–28 with COLLISIONS on the same numeric
>   prefix:
>   - `19__anomaly.sql` + `19__worker_fs_root.sql`
>   - `22__roster.sql` + `22__tenant_archived.sql`
>   - `23__handoff_routing.sql` + `23__workflow_checkpoints.sql`
>   - `24__operator_feedback.sql` + `24__skill_quality_score.sql` +
>     `24__vertical_archived.sql`
>   - `26__data_classification.sql` + `26__spend_baseline.sql` +
>     `26__worker_budget_alert.sql`
>   - `27__handoff_auctions.sql` + `27__spend_anomaly.sql`
>   - `28__recovery_cache.sql` + `28__role_duties.sql`
>
> Lazy `ensure_schema()` per module + no transactional guarantee +
> no rollback for v12+. **Predictable schema drift on first
> multi-process startup in the field.**

## Scope

Move all 26 raw SQL files (numbered 12–28) into versioned
`vN.rs` modules in `crates/argo-store/src/migrations/` with the
same transactional + rollback semantics as v1–v11.

Each colliding numeric prefix gets a UNIQUE successor version:

- `19__anomaly.sql` + `19__worker_fs_root.sql` → v12 + v13
- `22__roster.sql` + `22__tenant_archived.sql` → v14 + v15 (or
  collapse if logically inseparable; document the choice)
- ...and so on

The renumbering must preserve the order in which migrations
were applied historically (so on-main databases continue to
work). Worker-applied migrations record the OLD prefix in the
`schema_version` table; the new modules MUST recognize the old
version markers and skip re-running. (Deterministic compatibility
shim — required.)

After this lands, `migrate_down_to(v12)` works, lazy
`ensure_schema()` is removed, and every migration runs inside
the existing `pool.begin()` transaction.

## Approach

1. Audit each raw SQL file: extract its DDL into a `vN.rs` module
   following the existing v1–v11 template (up + down + tests).
2. Choose a stable renumbering. Recommendation: order by
   git-log-of-creation (oldest first), assign monotonic versions
   v12, v13, ..., v37.
3. Add a backwards-compat shim: when `current_version` is read
   from `schema_version` and shows an old "raw SQL" version (like
   the lazy ensure pattern that effectively logged "26"),
   rewrite/promote that to the new monotonic version atomically.
   Document this carefully.
4. Delete the raw SQL files only AFTER the shim is verified.
5. Add an integration test that walks every migration in numeric
   order on a fresh DB and asserts the resulting schema matches
   what raw SQL would have produced. Compare with sqlite's
   `.schema` output as the oracle.

## Files you may touch

- `crates/argo-store/src/migrations/mod.rs` — narrowly: register
  v12..v37 modules; add the back-compat shim
- `crates/argo-store/src/migrations/v12.rs` … `v37.rs` (NEW; up to
  ~26 modules, one per raw SQL file or merged where appropriate)
- `crates/argo-store/migrations/*.sql` — DELETE after migration
  modules verified
- `crates/argo-store/src/migration_compat.rs` (NEW) — shim
  recognizing old version markers + promotion logic + tests
- `crates/argo-store/src/migrations/v12_to_v37_tests.rs` (NEW
  per-module tests, OR a single shared test file — implementer's
  choice as long as < 500 LOC each)
- `crates/argo-cli/tests/migration_walk_smoke.rs` (NEW) —
  fresh-DB walk + schema oracle comparison

## Files you must NOT touch

- v1–v11 modules (they are stable; do not renumber retroactively)
- Any application code that reads from these tables (the schema
  is unchanged; only the migration system reorganizes)
- The `migration-consolidation-test` (#811) test file — extend if
  needed but do not delete

## Acceptance criteria

- [ ] All 26 raw SQL files (12–28) replaced by vN.rs modules with
      monotonic, unique version numbers
- [ ] No two modules share a version number
- [ ] Back-compat shim recognizes old version markers from
      databases that ran the lazy ensure pattern
- [ ] `roll_back_to(v12)` works (rollback paths defined for every
      new module; if an op is genuinely irreversible, document and
      mark `down` as a no-op with an audit-log warning)
- [ ] Lazy `ensure_schema()` calls removed from application code
      that depended on the raw SQL pattern
- [ ] Fresh-DB walk integration test passes
- [ ] Existing migration-consolidation-test (#811) extended to
      cover the new v12+ modules
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-store -p argo-cli` clean
- [ ] Every modified file < 500 LOC
- [ ] BLOCKED-acceptable scenario: if a single existing raw SQL
      file is genuinely too coupled to split cleanly, mark BLOCKED
      with a one-line reason naming the file; conductor can split
      into sub-briefs per cluster

## Branch + PR

- Branch: `claude-mission-migration-reconciliation`
- PR title: `store: reconcile raw SQL files into versioned migration modules (mission-migration-reconciliation)`

## Notes for the implementer

- This is a SCHEMA-SENSITIVE refactor. Extra care: every step
  validated against the migration-consolidation-test BEFORE
  pushing the next.
- The colliding numeric prefixes are NOT just naming clutter —
  they document a real race window where two SQL files both
  thought they were "version 26" and a parallel migration could
  apply them in either order. The renumbering closes that.
- The back-compat shim is the load-bearing piece. Without it,
  customers running on-main get a broken first-startup. Test it
  extensively against checkpointed schema_version states.
- This brief MAY exceed 500 LOC across new files. The 500-LOC
  invariant is per-file; the brief is sized large because the
  refactor is genuinely architectural. If a single file in the
  may-touch list approaches 500 LOC, split into sub-modules.
- Acceptable to ship as a multi-PR series if the operator+conductor
  prefer; default is single PR.
