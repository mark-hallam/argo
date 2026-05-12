# Schema migrations — forward-only

**Audience:** operators running SKYCOT in production. If you're
*writing* a migration, the implementer rules live alongside the runner
in `crates/argo-store/src/migrations/`; this doc answers the questions
an operator asks when a new migration lands on their box.

> **TL;DR:** Every SKYCOT release applies any new `vN` migrations once,
> in order, on startup. There is **no rollback path** — neither in the
> CLI nor in the supported operator workflow. The canonical fix for a
> bad migration is a follow-up `vN+1`; the canonical recovery for an
> in-progress disaster is restoring the backup taken **before** the
> upgrade.

## 1. The migration model

The runner in `crates/argo-store/src/migrations/mod.rs` is intentionally
boring:

- Each numbered `vN.rs` carries a single transactional `up` step.
- On startup, `run()` reads `schema_version` and applies every
  `vN > current` in ascending order, each inside its own SQLite
  transaction. A failure in `vN` rolls *that one migration's*
  transaction back and aborts startup; nothing past `vN-1` is left in
  a half-applied state.
- After a clean apply, `schema_version` advances to the highest `vN`
  that ran. Subsequent restarts on the same binary are no-ops.
- SKYCOT has shipped ~60 migrations to date (`v1` … `v60`). Numbers
  are dense and never reused.

The runner never invokes the `down` side of a migration during normal
operation. A `down` implementation exists on most migrations as a
test-support hook (the
`migration_reconciliation_tests` suite uses it to verify that schema
state round-trips correctly during development), but it is **not** an
operator-facing primitive. There is no `argo db migrate --down`,
`argo db rollback`, or equivalent.

> The CLI does ship `argo workflow rollback` — that command rolls back
> a *workflow run* to a saved checkpoint inside the workflow engine.
> It does not touch the schema. The names are similar; the
> capabilities are not.

## 2. Why forward-only

Three reasons, in descending order of weight:

1. **Operator reasoning stays simple.** "Prod is on `vN`" is one fact.
   "Prod is on `vN` but could be downgraded to `vM`" is a state space
   that has to be tested for every column, index, view, and trigger we
   ship — and the test matrix grows with every release. Forward-only
   collapses that matrix to a line.
2. **SQLite limits two-way migrations.** SQLite's online-schema-change
   support is limited compared to PostgreSQL or MySQL: dropping a
   column, changing a type, or renaming a column in place all require
   the full table-rebuild dance (create new table, copy rows, drop
   old, rename). A `down` that exactly reverses an `up` is achievable
   in code but expensive and easy to get wrong on a live DB; we don't
   pay that cost for a path we don't use.
3. **Dataset size doesn't justify Liquibase-shaped tooling.** SKYCOT's
   workspace DB is a single-binary SQLite file. The cost of a missed
   rollback path is "restore the backup and replay the user's
   actions" — not "page the DBA, freeze writes, replay the WAL
   into the snapshot." Forward-only is the right Idiot Index here.

## 3. If a migration is buggy: fix forward

You discover that `vN` shipped with a bug — wrong default, missing
index, column mistyped. The supported recipe is **fix-forward**:

1. Author `vN+1` (or `vN+2`, …) that performs the corrective work as
   an additive migration:
   - Wrong default? Insert a new column with the right default and
     migrate data in the same transaction; leave the old column for
     now.
   - Missing index? Add it (`CREATE INDEX IF NOT EXISTS …`).
   - Mistyped column? Add the correctly-typed sibling column,
     backfill, dual-write across the release, and remove the old
     column in a later migration once code no longer reads it (see
     §4 — the 3-step dance).
2. Land `vN+1` in `mod.rs` `all_migrations()` and ship it in the next
   release. Operators restart; the runner applies `vN+1` exactly once.
3. **Do not** edit `vN` after it has shipped to any production box.
   Editing applied migrations is the one path that genuinely breaks
   the model — the schema_version table records `vN` as applied, but
   the new bytes never run, so production and dev diverge silently.

This is the same fix-forward pattern you'd use for a bad PR after
merge: a new commit that corrects the problem, not a force-push that
rewrites history.

## 4. Compatibility rules — additive only

**`additive migrations`** is a CLAUDE.md invariant for exactly this
reason. Concretely, a migration is "additive" if it can be applied on
top of running prod **without** breaking any code path on the deployed
binary at the moment of apply:

| Change                                          | Additive?                                          |
| ----------------------------------------------- | -------------------------------------------------- |
| `CREATE TABLE …`                                | ✅ yes                                             |
| `CREATE INDEX …`                                | ✅ yes                                             |
| `ALTER TABLE … ADD COLUMN <col> <type> NULL`    | ✅ yes                                             |
| `ALTER TABLE … ADD COLUMN <col> <type> DEFAULT` | ✅ yes (constant default; SQLite back-fills NULLs) |
| `ALTER TABLE … ADD COLUMN <col> <type> NOT NULL WITHOUT DEFAULT` | ❌ requires the 3-step dance       |
| `DROP TABLE`, `DROP COLUMN`, `DROP INDEX`       | ❌ requires the 3-step dance                       |
| Column type change                              | ❌ requires the 3-step dance                       |
| Column rename                                   | ❌ requires the 3-step dance                       |
| Trigger / view replacement that changes shape   | ❌ requires the 3-step dance                       |

### The 3-step dance for non-additive change

Spread over **at least two releases**, never one:

1. **Release N — add.** Migration `vK` introduces the new column /
   table / shape. Code on release N starts **dual-writing** to old
   and new representations; reads still come from the old.
2. **Release N+1 — switch reads.** Code on release N+1 reads from
   the new representation. Old representation is still dual-written
   but no longer authoritative. (If your release cadence is slow
   enough that operators may run N+1 against a DB that was migrated
   by N-1 and never touched N, do the read switch inside its own
   `vK+1` migration so the relationship is forced.)
3. **Release N+2 — remove.** Once N+1 has been live long enough that
   you trust no operator will downgrade their *binary* to N (which
   would still try to dual-write to the now-removed old column), ship
   `vK+2` that drops the old column / table / shape.

Skipping a step — e.g. removing the old column in the same release
that adds the new one — guarantees a busted upgrade for any operator
who restarts mid-deploy.

## 5. Disaster recovery — backup before, restore on disaster

Forward-only doesn't mean "no recovery." It means recovery goes
through the backup, not through the migration runner. The supported
shape:

1. **Before every upgrade, take a backup.** For the single-tenant
   workspace DB (`~/.argo/argo.db`), a cold `cp` or `sqlite3 …
   .backup` against the file is sufficient because `argo` is stopped
   for the upgrade anyway. Production deployments should already have
   a periodic backup job; the upgrade hook is the moment to confirm
   the latest backup is fresh and restorable.
2. **If a migration fails to apply,** the runner aborts startup and
   leaves the DB at `vN-1` (the failed transaction rolls back). The
   binary refuses to come up. Resolve by:
   - Reading the startup log for the migration error.
   - If the bug is in the migration itself, ship the corrected
     migration (or a follow-up `vN+1` that no-ops the broken `vN`'s
     intent) and try again.
   - If the bug is environmental (disk full, file permissions, locked
     DB), fix the environment and restart.
3. **If a migration succeeded but the new schema breaks the running
   tenant** (e.g. data corruption from a buggy backfill, missing
   constraint), the only "rollback" is **stop the binary, restore the
   pre-upgrade backup, replay any user actions taken since the
   backup**. SKYCOT does not support downgrading the schema in place.

This is the operator commitment forward-only buys you: failed
migrations are recoverable by restart-and-retry, *succeeded* migrations
are recoverable by restore-from-backup, and there is no third option
to forget about or get wrong.

## 6. Where the code lives

- `crates/argo-store/src/migrations/mod.rs` — the runner, the
  `schema_version` table, and the registry of all migrations.
- `crates/argo-store/src/migrations/vN.rs` — one per shipped
  migration. The `up` body is what runs in prod; the `down` body, if
  present, is test-support only.
- `crates/argo-store/src/migration_reconciliation_tests.rs` — the
  test harness that exercises `up`/`down` symmetry for development
  confidence. Operators do not interact with this layer.

If you're tempted to call `argo_store::Store::roll_back_to(…)` from an
operator script: don't. That entry point exists for the in-process
test harness; it is not a stable operator API, has no audit
integration, and will not be supported across releases.

---

**Cross-references**

- `docs/skycot/RUNBOOK.md` — operator runbook; this page is linked
  from the Migrations subsection.
- `CLAUDE.md` — "Maintain" → `additive migrations`.
