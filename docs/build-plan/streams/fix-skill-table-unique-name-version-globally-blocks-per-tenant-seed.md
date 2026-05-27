# fix-skill-table-unique-name-version-globally-blocks-per-tenant-seed

**Owner**: claude or codex.
**Depends on**: none.
**Authored against**: origin/main `a9512cd22` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-skill-table-unique-name-version-globally-blocks-per-tenant-seed`.

## Why

Evaluator round 97 (2026-05-27T03:55Z) cold-reran the load-bearing
regression test that PR #2437 claimed to close. It still **fails**:

```
$ cargo test -p argo-demo-seed --lib seeds_independently_per_tenant

test marketplace::tests::seeds_independently_per_tenant ... FAILED

failures:
---- marketplace::tests::seeds_independently_per_tenant stdout ----
thread 'marketplace::tests::seeds_independently_per_tenant' panicked at
crates/argo-demo-seed/src/marketplace.rs:205:14:
called `Result::unwrap()` on an `Err` value:
  Argo(Provider("skill install: error returned from database:
  (code: 2067) UNIQUE constraint failed: skill.name, skill.version"))
```

Three independent confirmations that this is a substrate defect, not a
flaky test (full report at
`docs/skycot/evals/scheduled/2026-05-27/round-97-pr-2437-demo-seed-post-merge-FAIL.md`):

1. **Schema** — `sqlite3 ~/.argo/sessions.db ".schema skill"` shows
   `UNIQUE(name, version)` with NO `tenant_id` column in the
   constraint. The v80 migration (which added `tenant_id` to the
   `skill` table) did not amend the uniqueness invariant.

2. **Production code** — `crates/argo-skill/src/store_vertical.rs:110-127`:
   `install_for_tenant_if_absent` does a per-tenant pre-check
   (`is_active_name_present_for_tenant` at line 134), then on a NEW
   tenant falls through to `insert_row_for_tenant` (line 124), which
   issues a raw `INSERT INTO skill(...)`. The global UNIQUE rejects
   it — exactly as observed in the test failure.

3. **Live DB state** — gateway-side probe shows 7 skill rows / 1
   distinct tenant. Only the first tenant ever successfully seeded;
   every subsequent tenant going through SPA `principal_resolve` will
   now hit a noisy DB error instead of the prior silent skip.

PR #2437 moved the failure mode from "silently skipped, returns
`Ok(None)`" to "noisy `UNIQUE` constraint Err" — but did NOT enable
per-tenant catalog seeding. Demo MP4 Scene 4 (skills marketplace) is
still gated on this: the moment a second tenant boots against the
running gateway, their `/api/v1/skills` returns the seed error
instead of the demo skill list.

## What

Add **migration v84** (SQL slot **135**) that relaxes the skill
table's uniqueness invariant from
`UNIQUE(name, version)` → `UNIQUE(name, version, tenant_id)` so
per-tenant catalog seeding works.

### Why option (a), not (b)

R97 listed two options:
- **(a)** relax `UNIQUE(name, version)` → `UNIQUE(name, version, tenant_id)`
- **(b)** keep global uniqueness as a marketplace-publishing
  invariant; rework seeding to use a per-tenant install-mapping table
  (or `INSERT-OR-IGNORE` with same-id-different-tenant-id rows
  treated as views).

This brief chooses **(a)** for these reasons (decision-rationale, not
preference):

1. **The "global marketplace publishing" invariant is already
   architecturally broken.** v80 added `tenant_id` to the row but
   left the UNIQUE constraint global — that's the source of the
   bug. There is no consistent invariant to preserve; we are
   choosing which of the two halves to align with the other.
2. **Demo MP4 Scene 4 is v1.0 ship-gate.** The smallest change that
   unblocks the load-bearing test is preferable to a larger refactor
   while the swarm is in pre-record window.
3. **Reversibility.** A true global marketplace surface (cross-tenant
   skill catalog) can be reintroduced later as a **separate**
   `skill_catalog` table that the per-tenant `skill` rows reference
   — this is the natural design once the v1 demo ships. Tightening
   the UNIQUE later is a one-line follow-up migration; nothing about
   this brief precludes that path.

### Migration mechanics

SQLite cannot drop a UNIQUE constraint with `ALTER TABLE`. The
canonical SQLite pattern is the **table-swap**:

1. `CREATE TABLE skill_new (... UNIQUE(name, version, tenant_id))`
   with the same column set + types as the existing `skill` table at
   the point of v83 (all 14 columns: `id`, `name`, `version`, `body`,
   `frontmatter`, `source`, `hash`, `learned_in`, `superseded_by`,
   `created_at`, plus v11's `lifecycle_state`, v41's `shared` +
   `shared_with_cluster`, v50's `min_trust_grade`, v80's `tenant_id`).
   Verify the column set by running
   `sqlite3 ~/.argo/sessions.db ".schema skill"` at claim time — the
   list above is authoritative at SHA `a9512cd22` but a worker
   should re-verify before authoring the SQL.
2. `INSERT INTO skill_new SELECT ... FROM skill` (preserve all rows
   including the `__migration_placeholder__` tenant from v80).
3. `DROP TABLE skill`.
4. `ALTER TABLE skill_new RENAME TO skill`.
5. Recreate the v80 `idx_skill_tenant_id` index on the new table.

The down-migration reverses the swap (recreate a `skill_old` with the
v83 schema, copy rows, drop, rename, restore the original index set).
Down-migration MUST preserve every row including the
`__migration_placeholder__` sentinel.

### Surface contract

No new `pub struct` / `pub enum` / `pub fn` is introduced — this is
pure schema. The existing `install_for_tenant_if_absent` consumer
(`crates/argo-skill/src/store_vertical.rs:110`) already expects this
constraint shape; nothing in Rust changes.

### Test pin (regression-locking)

The existing test `marketplace::tests::seeds_independently_per_tenant`
in `crates/argo-demo-seed/src/marketplace.rs` currently **fails**
and MUST turn 🟢 GREEN under this fix. Do not modify the test —
modifying it would erase the regression evidence.

Add ONE new unit test in `crates/argo-store/src/migrations/v84.rs`
(or as a sibling pattern to v83's `up_quarantines_legacy_rows_*`
test) named `up_allows_same_name_version_across_tenants`:

- Apply migrations up to v84 against `sqlite::memory:`.
- `INSERT INTO skill (..., tenant_id) VALUES (..., 'tenant_a')`.
- `INSERT INTO skill (..., tenant_id) VALUES (..., 'tenant_b')`
  with same `name` + `version` — assert it **succeeds** (this is
  the post-fix invariant).
- `INSERT INTO skill (..., tenant_id) VALUES (..., 'tenant_a')`
  with same `name` + `version` + same tenant — assert it **fails**
  with `UNIQUE constraint failed` (the per-tenant uniqueness must
  still bite).

## May-touch

- `crates/argo-store/src/migrations/v84.rs` (NEW)
- `crates/argo-store/migrations/135__skill_unique_per_tenant.sql` (NEW)

### Slot-numbering note (don't get confused)

There are TWO independent namespaces here that both happen to share
digit space:

- **Migration VERSION** numbers run 1..83 on origin/main. The next
  free version is **84**. Registered in
  `crates/argo-store/src/migrations/registry_compliance.rs`.
- **SQL FILENAME slot** numbers run 1..134 (some 2-digit, some
  3-digit). The next free filename slot is **135**. Validated by
  `scripts/check-migration-slots.sh`.

Existing file `crates/argo-store/migrations/84__operator_feedback.sql`
uses filename slot 84 — that does NOT make migration version 84
taken. The lint script `scripts/check-brief-freshness.sh` reports a
warning that conflates the two; it is a false-positive and does not
fail the freshness check (script exits 0 with "✅ No drift signals").
- `crates/argo-store/src/migrations/registry_compliance.rs`
  (register v84 in the slice; add `v84` to the `use super::{}` line)
- `crates/argo-store/src/migrations/mod.rs` if it declares migration
  submodules (re-verify by inspection — v83 wiring is the pattern)
- `docs/build-plan/BUILD_STATUS.md` (mandatory ledger flip on PR open)

Do NOT touch:
- `crates/argo-skill/src/store_vertical.rs` — the Rust call-site
  (`install_for_tenant_if_absent`) is already correct; the fix is
  purely at the SQL layer.
- `crates/argo-demo-seed/src/marketplace.rs` — the test there is the
  regression pin; do not modify it.

## Acceptance criteria

- [ ] `cargo test -p argo-demo-seed --lib seeds_independently_per_tenant`
      passes (cold; from a freshly-built target dir). Worker reports
      the exact pass line + wall-clock in PR body.
- [ ] `cargo test -p argo-store --lib up_allows_same_name_version_across_tenants`
      passes (the new test added in v84's module).
- [ ] `cargo test -p argo-store --lib` overall is GREEN (no other
      test regresses; v82/v83 down-migration tests in particular must
      still pass).
- [ ] `sqlite3 ~/.argo/sessions.db ".schema skill"` post-migration
      shows `UNIQUE(name, version, tenant_id)` and NO bare
      `UNIQUE(name, version)`. (Worker may probe a temp DB instead
      of `~/.argo/sessions.db` if running headless.)
- [ ] `scripts/check-migration-slots.sh` passes (no duplicate v84,
      no duplicate filename slot 135).
- [ ] Schema constant `EXPECTED_SCHEMA_VERSION` updated to 84 if the
      project uses one (check `crates/argo-store/src/lib.rs` /
      `migrations/mod.rs` at claim time).
- [ ] `bash scripts/check-gateway-freshness.sh` post-merge reports
      `gateway_schema=v83 codebase_latest=v84 drift=1`
      (this is the EXPECTED state immediately after merge; clears
      once `make restart-live-gateway` runs — flag in PR body so
      the conductor's post-merge tick can surface the restart).

## Out of scope (do NOT widen)

- **Reintroducing a global marketplace surface.** That belongs in a
  follow-up brief (`mission-skill-catalog-cross-tenant-publishing` or
  equivalent), not in this fix.
- **The workflow_definition_index parallel write-gap.** R97 names
  this as a separate brief candidate
  (`fix-gateway-verticals-boot-seed-workflow-definition-index`); a
  worker may author it after this brief lands but it is NOT part of
  this brief's may-touch list.
- **The R97 P1 process brief**
  (`ops-conductor-pre-merge-gate-block-admin-merge-when-pr-body-acknowledges-unrun-test`)
  is a separate authorship; do not bundle.

## Parallel-surface check

Per `docs/skycot/DRIFT_PREVENTION.md`'s "grep before write" rule:

- `rg 'CREATE TABLE skill_new'` against `crates/` returned 0 hits at
  authoring time (SHA `a9512cd22`) — no parallel migration is in
  flight.
- `rg 'UNIQUE.*name.*version.*tenant_id'` against
  `crates/argo-store/` returned only `v72.rs` (verticals table, NOT
  skill) — no parallel skill-constraint migration.
- `rg 'install_for_tenant_if_absent'` showed exactly ONE call site
  (`crates/argo-demo-seed/src/marketplace.rs:94`) and the canonical
  definition (`crates/argo-skill/src/store_vertical.rs:110`). The
  surface is wired; no dead-code drift risk.

## Source

- Evaluator round 97: `docs/skycot/evals/scheduled/2026-05-27/round-97-pr-2437-demo-seed-post-merge-FAIL.md`
- Closes anti-theater hole left by `72c005978` (PR #2437) — the
  fix that "moved the failure mode from silent skip to noisy
  constraint error" but did not enable per-tenant catalog seeding.
- 104th anti-theater verification this week (per R97 footer).
