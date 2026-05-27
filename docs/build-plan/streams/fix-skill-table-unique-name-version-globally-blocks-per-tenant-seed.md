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

SQLite cannot drop a UNIQUE constraint with `ALTER TABLE`. The canonical SQLite pattern is the **table-swap**, but a 2026-05-27 audit pass surfaced two hazards the naive 5-step swap would silently mishandle:

#### ⚠️ HAZARD 1 — Foreign key from `session_skill` (currently INERT, but future-fragile)

`session_skill.skill_id` declares `REFERENCES skill(id) ON DELETE CASCADE` (verbatim from `v4.rs:33`).

**Empirical finding from the 2026-05-27 audit pass**: this codebase NEVER enables `PRAGMA foreign_keys = ON`. The default SQLite behavior is foreign-key enforcement OFF (per https://sqlite.org/pragma.html#pragma_foreign_keys). A grep across `crates/argo-store/` finds zero `PRAGMA foreign_keys` hits; `wal_tuning.rs:105-115` sets 7 pragmas, foreign_keys is not one of them. So **in the current runtime config, `DROP TABLE skill` does NOT cascade-delete `session_skill` rows** — the hazard is INERT today.

**BUT** — any future change that enables FKs (postgres target, security hardening pass, etc.) would silently break this migration. This brief specifies the forward-safe approach:

1. **Issue `PRAGMA defer_foreign_keys = ON`** at the top of `up()`. No-op today (FKs off) but armed if/when FKs flip on later. Works inside the per-migration transaction at `registry.rs:28-46` (the more familiar `PRAGMA foreign_keys = OFF` would NOT work inside a transaction).
2. **Reset `PRAGMA defer_foreign_keys = OFF`** at the end of `up()` for hygiene (connection-scoped; persists for that session otherwise).
3. **The regression test `up_preserves_session_skill_fk_after_table_swap` MUST explicitly enable `PRAGMA foreign_keys = ON` on its in-memory pool BEFORE running migrations** — otherwise the test passes regardless of whether the worker remembered the defer pragma, giving false confidence. Test recipe:
   ```rust
   sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await?;
   // insert one skill row + one session_skill row referencing it
   // run migrations up to v84
   // assert SELECT count(*) FROM session_skill is unchanged
   // (if the worker forgot defer_foreign_keys, the cascade fires HERE
   //  because FKs ARE on for THIS test pool)
   ```

#### ⚠️ HAZARD 2 — All four auxiliary indexes must be recreated

A 2026-05-27 audit grep against the `skill` table found 4 indexes the migration MUST recreate post-swap. Forgetting any one is a silent perf regression (full table scans, no test fails):

| Index name | Defined in | DDL |
|---|---|---|
| `idx_skill_vertical_id` | `v9.rs:50` | `CREATE INDEX idx_skill_vertical_id ON skill(vertical_id)` |
| `idx_skill_lifecycle` | `v11.rs:54` | `CREATE INDEX idx_skill_lifecycle ON skill(lifecycle_state)` |
| `idx_skill_shared_cluster` | `v41.rs:37-38` + `migrations/40__skill_shared.sql:22` | `CREATE INDEX idx_skill_shared_cluster ON skill(shared_with_cluster) WHERE shared = 1` (literal name confirmed by audit; no probe needed) |
| `idx_skill_tenant_id` | `v80.rs:53` (down ref) | `CREATE INDEX IF NOT EXISTS idx_skill_tenant_id ON skill(tenant_id)` |

Audit also confirmed: zero triggers on `skill` table, zero views referencing `skill` (only `skill_provenance` snapshot row from v69 which is denormalized and independent).

#### Canonical migration sequence

```sql
-- Top of up(): defer FK checks for the duration of this transaction (forward-safe)
PRAGMA defer_foreign_keys = ON;

-- 1. CREATE skill_new with same columns + tightened UNIQUE
CREATE TABLE skill_new (
  id              BLOB PRIMARY KEY,
  name            TEXT NOT NULL,
  version         TEXT NOT NULL,
  body            TEXT NOT NULL,
  frontmatter     TEXT NOT NULL,
  source          TEXT NOT NULL,
  hash            BLOB NOT NULL,
  learned_in      BLOB,
  superseded_by   BLOB REFERENCES skill(id),  -- self-FK re-resolves to renamed table post-swap
  created_at      INTEGER NOT NULL,
  -- v9, v11, v41, v50, v80 columns — re-verify at claim time via .schema skill:
  vertical_id     BLOB,                                              -- v9
  lifecycle_state TEXT NOT NULL DEFAULT 'active',                    -- v11
  shared          INTEGER NOT NULL DEFAULT 0,                        -- v41
  shared_with_cluster TEXT,                                          -- v41
  min_trust_grade INTEGER,                                           -- v50
  tenant_id       TEXT NOT NULL DEFAULT '__migration_placeholder__', -- v80
  UNIQUE(name, version, tenant_id)                                   -- THE FIX
);

-- 2. Copy ALL rows (preserves __migration_placeholder__ sentinel)
INSERT INTO skill_new SELECT id, name, version, body, frontmatter, source, hash,
  learned_in, superseded_by, created_at, vertical_id, lifecycle_state, shared,
  shared_with_cluster, min_trust_grade, tenant_id
FROM skill;

-- 3. Drop old. With defer_foreign_keys = ON, no cascade fires now.
DROP TABLE skill;

-- 4. Rename. session_skill.skill_id resolves to new skill at commit time.
ALTER TABLE skill_new RENAME TO skill;

-- 5. Recreate ALL FOUR indexes (omitting any one is silent perf regression)
CREATE INDEX idx_skill_vertical_id ON skill(vertical_id);
CREATE INDEX idx_skill_lifecycle ON skill(lifecycle_state);
CREATE INDEX idx_skill_shared_cluster ON skill(shared_with_cluster) WHERE shared = 1;
CREATE INDEX IF NOT EXISTS idx_skill_tenant_id ON skill(tenant_id);

-- 6. Reset hygiene
PRAGMA defer_foreign_keys = OFF;
```

**Self-FK note**: `superseded_by BLOB REFERENCES skill(id)` is a self-referential FK. When `ALTER TABLE skill_new RENAME TO skill` runs, SQLite re-resolves the reference to the renamed table (confirmed by SQLite docs — rename rewrites unqualified FK references).

#### Down-migration

Reverses the swap (recreate `skill_old` with the v83 schema, copy rows, drop, rename, restore the original v1 + v9 + v11 + v41 + v50 + v80 index set). Same `defer_foreign_keys = ON` requirement. MUST preserve every row including the `__migration_placeholder__` sentinel.

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
- [ ] `cargo test -p argo-store --lib up_preserves_session_skill_fk_after_table_swap`
      passes (NEW test added 2026-05-27 audit — proves the
      `session_skill.skill_id` FK rows survive the table-swap
      without cascade-delete. Recipe MUST `PRAGMA foreign_keys = ON`
      on the in-memory pool before running migrations, then insert
      one `skill` row + one `session_skill` row referencing it, run
      up to v84, assert `SELECT count(*) FROM session_skill` is
      unchanged. Without enabling FKs on the test pool, this test
      passes regardless of whether the worker remembered
      `defer_foreign_keys` — giving false confidence. This is the
      regression pin for HAZARD 1.).
- [ ] `cargo test -p argo-store --lib up_preserves_all_skill_indexes_after_table_swap`
      passes (NEW test added 2026-05-27 audit — proves the 4
      auxiliary indexes survive the swap. Recipe: run migrations
      up to v84 on `:memory:`, then query `sqlite_master WHERE type='index' AND tbl_name='skill'`
      and assert the set includes
      `idx_skill_vertical_id`, `idx_skill_lifecycle`,
      `idx_skill_shared_cluster`, `idx_skill_tenant_id`. This is
      the regression pin for HAZARD 2.).
- [ ] `cargo test -p argo-store --lib` overall is GREEN (no other
      test regresses; v82/v83 down-migration tests in particular must
      still pass).
- [ ] `sqlite3 ~/.argo/sessions.db ".schema skill"` post-migration
      shows `UNIQUE(name, version, tenant_id)` and NO bare
      `UNIQUE(name, version)`. (Worker may probe a temp DB instead
      of `~/.argo/sessions.db` if running headless.)
- [ ] `scripts/check-migration-slots.sh` passes (no duplicate v84,
      no duplicate filename slot 135).
- [ ] **MANDATORY** — `EXPECTED_SCHEMA_VERSION` constant in
      `crates/argo-store/src/lib.rs:364` (or wherever the audit
      confirms it lives at claim time — `rg 'EXPECTED_SCHEMA_VERSION'
      crates/`) bumped from 83 → 84. The pre-merge gate's
      `scripts/check-expected-schema-version-matches-tip.sh`
      preflight (called by `conductor-pre-merge-gate.sh:79`) catches
      omission, but if the worker bypasses the gate (the failure
      mode `ops-conductor-pre-merge-gate-block-admin-merge-when-pr-body-acknowledges-unrun-test`
      is designed to prevent), the omission silently lands and the
      gateway boot panics. Worker MUST report the bumped line in PR
      body.
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
