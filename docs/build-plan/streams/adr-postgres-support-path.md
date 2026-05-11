# ADR — PostgreSQL support path for SKYCOT (long-tail SaaS scale)

**Status**: ACCEPTED 2026-05-11 (Tier 0 only).
**Authored against**: origin/main on `2026-05-11`.
**Source**: MISSION_BACKLOG.md Section 1 #29 — "Plan PostgreSQL support path (long-tail SaaS)."
**Decision-maker**: operator (Tier 0 locked-in 2026-05-11; Tier 1 approved-in-principle, unscheduled).
**Scope**: design ADR. No code change in this PR.

## 1. Status / context

SKYCOT v1 ships as a **single-binary SQLite** deployment. This is a
load-bearing architectural invariant — CLAUDE.md lists "single-binary
architecture" under MAINTAIN, and `argo-store` today uses SQLite via
`sqlx::Sqlite` end-to-end.

For the self-hosted use case (one tenant per binary, one operator per
deployment, low write QPS) this is the right answer. SQLite's own
documentation puts it bluntly: *"if you don't need >1 writer, SQLite
is the answer."* That is the operator's first decision point.

The case for a future PostgreSQL backend is **long-tail multi-tenant
SaaS** — specifically deployments where any of the following are true:

- **>100 active tenants** sharing one binary, with overlapping write
  windows. SQLite serializes writers at the file level (WAL mitigates
  but does not remove this); at high tenant counts the write lock
  becomes a soft ceiling on throughput.
- **Sustained write load** — audit log volume, action receipts,
  high-frequency approval traffic — pushing per-second write counts
  into the hundreds.
- **Cross-region replication need** — DR posture, read-replicas for
  geographic latency, or compliance-driven multi-AZ requirements.
  SQLite has Litestream/LiteFS, but the operational model is "ship
  log files," not "ACID across regions."
- **Operator preference** for managed databases (RDS / Cloud SQL /
  Supabase) — some buyers will not accept "your database is a file in
  the binary's data dir," even when it would be technically sufficient.

None of these apply at v1 launch. All of them are plausible by v2.

This ADR documents the migration path so we can defer the work
without painting ourselves into a corner. **No code changes today.**

## 2. Three-tier proposal

We propose three tiers, sequenced over the v1 → v2 lifetime.

### Tier 0 — v1, today

**Single-binary SQLite. No change.**

- `argo-store` continues to use `sqlx::SqlitePool` directly.
- Migrations live in `crates/argo-store/migrations/` and target SQLite
  syntax (PRAGMA, STRICT, JSON1).
- No driver-abstraction work, no feature flags, no test-matrix split.
- Operator ships one binary, one DSN-less config, one data dir.

This is the v1 architecture. CLAUDE.md "single-binary" invariant is
intact.

### Tier 1 — post-v1.x, opt-in PostgreSQL

**Optional PostgreSQL backend behind a `--features postgres` build flag.**

- `argo-store` exposes a `Pool<DB>`-style abstraction generic over
  `sqlx::Sqlite | sqlx::Postgres`. Every `sqlx::query!()` /
  `sqlx::query_as!()` call site is audited and routed through the
  abstraction.
- Default build remains SQLite. The `postgres` feature is additive;
  the SQLite binary continues to ship as the canonical self-hosted
  artifact.
- **No migration of existing operators.** Tier 1 is strictly opt-in.
  An existing SQLite deployment stays SQLite forever unless its
  operator explicitly runs the (future) `migrate-to-postgres` flow.
- Operators choosing Tier 1 build with `--features postgres`, supply
  a `DATABASE_URL=postgres://...` config value, and accept the
  operational overhead of running a Postgres instance.

This tier is **approved-in-principle but not scheduled.** It does not
land until a real operator hits the SQLite ceiling and we have a
concrete throughput number to design against.

### Tier 2 — post-v2.x, PostgreSQL default for SaaS

**PostgreSQL becomes the default for SKYCOT-managed SaaS; SQLite
remains the default for self-hosted single-tenant.**

- The SaaS deployment template ships with Postgres preconfigured.
- The self-hosted template still ships SQLite.
- Documentation distinguishes the two deployment models explicitly.
- New features are tested against both backends in CI.

Tier 2 requires a **follow-up ADR** before adoption. It implies a
forked default story (self-hosted vs SaaS) and the operator must
sign off on that split deliberately.

## 3. Engineering work catalog

Concrete work items that Tier 1 implies. Listed for cost-visibility,
not as a commitment. None of this is scheduled.

1. **`argo-store` driver-generic refactor.** Every call site that
   constructs an `sqlx::SqlitePool`, executes `sqlx::query!()` with
   SQLite-specific syntax, or relies on SQLite-only return types
   must be audited. Estimate: full sweep of
   `crates/argo-store/src/**/*.rs` (currently ~40 files at the time
   of writing). Methodology: introduce a `trait StoreBackend` with
   `Sqlite` and `Postgres` impls; switch on a `cfg(feature =
   "postgres")` boundary in `Cargo.toml`.

2. **Migration system adapter.** SQLite introspects schema via
   `PRAGMA table_info(<table>)`; Postgres uses
   `information_schema.columns`. The migration runner's
   "does this column exist yet" check needs a backend-aware shim.
   Estimate: one new file in `crates/argo-store/src/migrations/`,
   ≤ 200 LOC, plus per-migration auditing of SQLite-only SQL
   (STRICT tables, `WITHOUT ROWID`, `ON CONFLICT` semantics).

3. **CI test matrix doubling.** `cargo test -p argo-store` must run
   against both backends. Two CI lanes:
   - `argo-store-sqlite` — current behavior, no DB service.
   - `argo-store-postgres` — spins up `postgres:16` in GitHub Actions
     services, sets `DATABASE_URL`, runs tests with `--features
     postgres`.

   This roughly doubles `argo-store` CI wall-clock. Caching helps
   but the runtime cost is real.

4. **Connection pool semantics.** Both backends use `sqlx::Pool`, so
   the API surface is shared. But timeout, idle-eviction, and
   max-connections defaults differ — SQLite is usually 1 writer +
   many readers; Postgres is many connections, all roughly equal.
   Tier 1 must expose backend-specific pool tunables in the config
   schema (`argo.toml` `[store.pool]` section).

5. **Operator config surface.** `argo.toml` gains a
   `[store.backend]` discriminated-union: `sqlite { path = "..." }`
   or `postgres { dsn = "..." }`. Today the field is implicit.
   Backwards-compat: missing field defaults to `sqlite` with the
   v1 path.

6. **Type-mapping audit.** SQLite is dynamically-typed; Postgres is
   strictly-typed. Every `BLOB`/`TEXT`/`INTEGER` column needs an
   explicit Postgres type. JSON columns are the worst offender —
   see §5 risk register.

## 4. Operator-facing migration (sketch, not implemented)

A future CLI surface, planned for the Tier 1 milestone:

```
argo store migrate-to-postgres --dsn postgres://user:pass@host/db
    [--dry-run]
    [--batch-size 1000]
    [--verify]
```

**Behavior** (intended; no implementation in this ADR):

1. Validate Postgres DSN reachable and target DB is empty.
2. Open the live SQLite store read-only; snapshot via
   `sqlite3 .backup`.
3. Apply Postgres schema (translated migrations) to target DB.
4. Stream each table from snapshot → Postgres via paged
   `INSERT ... ON CONFLICT DO NOTHING`.
5. With `--verify`: run row-count + checksum comparison per table.
6. Print a "switchover instructions" block: stop binary, swap
   config to `[store.backend.postgres]`, restart.

**Gotchas to enumerate (operator-facing docs):**

- **rowid behavior**. SQLite implicit `rowid` does not exist in
  Postgres. Tables that lean on `rowid` ordering need an explicit
  `id BIGSERIAL` column added in the schema-translation step.
- **`AUTOINCREMENT` vs `SERIAL`**. SQLite `INTEGER PRIMARY KEY
  AUTOINCREMENT` maps to `BIGSERIAL`. Sequence values must be set
  to `MAX(id) + 1` post-import or new inserts collide.
- **JSON1 vs JSONB**. SQLite stores JSON as `TEXT` and queries via
  `json_extract`; Postgres has native `JSONB` with `->`/`->>`
  operators. Migration must decide per-column: keep as `TEXT` (no
  query rewrite) or upgrade to `JSONB` (query rewrite required,
  faster reads).
- **Datetime semantics**. SQLite stores `TEXT` ISO-8601 by
  convention; Postgres has `TIMESTAMPTZ`. Migration validates that
  every datetime column is parseable, normalizes to UTC, casts to
  `TIMESTAMPTZ`.
- **Boolean encoding**. SQLite `INTEGER` 0/1; Postgres native
  `BOOLEAN`. Migration translates 0/1 → false/true.
- **Foreign-key enforcement**. SQLite enforces FKs only when
  `PRAGMA foreign_keys = ON`; some legacy data may violate FKs that
  Postgres will reject on import. Migration emits a dry-run report
  of FK violations before insert.
- **Case-sensitivity collation**. SQLite default collation is
  binary case-sensitive; Postgres default depends on cluster
  locale. Document a recommended `CREATE DATABASE ... LC_COLLATE
  'C'` for parity.

**Out of scope for this ADR**: actual CLI implementation; benchmark
data; production migration runbooks. Those land in the Tier 1
implementation brief, not here.

## 5. Risk register

Substantive risks of adopting Tier 1 / Tier 2. Honest about cost.

1. **Test coverage doubles for `argo-store`.** Every test must run
   against both engines. CI wall-clock for `argo-store` roughly
   doubles. Some tests will pass on SQLite and fail on Postgres
   (or vice versa) due to engine-specific behavior — JSON
   comparison, datetime precision, NULL ordering — and each one is
   a per-test investigation. **Mitigation**: gate Tier 1 work
   behind a per-test backend-tag macro, run SQLite by default in
   PR CI, run the full matrix nightly.

2. **Migration churn.** Every new schema migration must work on
   both engines. The migration author's mental model expands from
   "write the SQL I know" to "write the SQL that works on both."
   Cost is per-migration and never goes away. **Mitigation**:
   migration-template macros that generate parallel SQLite/Postgres
   variants from a single declarative spec; or a strict "common
   subset SQL" style guide (no STRICT, no `WITHOUT ROWID`, no
   `PRAGMA` outside the runner shim).

3. **Surface drift / abstraction leaks.** SQLite-specific tricks —
   STRICT tables, PRAGMA hacks, `INSERT OR REPLACE`, `LIMIT` in
   `UPDATE`/`DELETE` — leak into the abstraction unless we actively
   police them. Postgres has analogous-but-not-identical features
   (`ON CONFLICT`, `RETURNING`, etc.). Without discipline the
   `argo-store` API ends up with a least-common-denominator surface
   that helps neither engine. **Mitigation**: lint-level enforcement
   in `argo-store` (clippy custom lint or grep gate in CI) for
   SQLite-isms; code review checklist for engine-specific tokens.

4. **Operator complexity.** "Which engine should I use?" becomes a
   support FAQ. Self-hosted operators must understand they're on
   SQLite by default; SaaS operators must understand the Postgres
   dependency. Misconfigured deployments (running SQLite at SaaS
   scale, or running Postgres with no replication) create support
   load. **Mitigation**: opinionated deployment templates per use
   case; `argo doctor` checks that flag SQLite use above a tenant-
   count threshold with a recommendation to migrate.

5. **`sqlx` macro surface.** `sqlx::query!()` and `query_as!()`
   compile-time-verify SQL against a *specific* live DB. A
   multi-backend codebase either drops the macros (loses
   compile-time SQL checking — large regression) or wires the
   `SQLX_OFFLINE_DIR` to two sets of metadata, one per backend.
   **Mitigation**: keep `query!()` macros, generate offline
   metadata for both backends in CI, treat metadata drift as a
   PR-blocking check.

6. **Build-time and binary-size cost.** Compiling `sqlx-postgres`
   into the default binary even when unused inflates compile time
   and binary size. **Mitigation**: gate Postgres support behind
   `--features postgres` so the default SQLite binary is unaffected.
   Build matrix grows but each variant stays lean.

## 6. Decision

**ACCEPT Tier 0** for v1. SKYCOT ships SQLite single-binary as the
canonical deployment. CLAUDE.md "single-binary architecture"
invariant stands.

**Tier 1 is approved-in-principle but NOT scheduled.** No code
work today. The engineering catalog in §3 is recorded so the cost
is visible when it becomes time to schedule. A separate brief opens
Tier 1 when an operator hits the SQLite ceiling with measurable
data — not before.

**Tier 2 is post-v2 and requires a follow-up ADR.** Splitting the
default story between self-hosted and SaaS is a deliberate product
decision that this ADR does not pre-commit to.

## 7. Open questions

Defer to the operator. None blocks Tier 0.

1. **Will SQLite stay the canonical self-hosted default forever, or
   is Tier 2 the eventual world?** I.e. does the SaaS line of
   business ever become the dominant deployment shape, such that
   self-hosted SQLite becomes the niche path? Operator decision,
   driven by v2 revenue mix.

2. **What is the threshold for flipping Tier 1 from "available" to
   "recommended for SaaS"?** Candidates: tenant count, sustained
   write QPS, p99 write latency on SQLite, multi-region requirement.
   The threshold should be a concrete number an operator can check,
   not a vibe. Needs benchmark data to set responsibly.

3. **Cross-region replication strategy.** When/if Tier 2 lands, do
   we lean on managed-DB primitives (AWS RDS multi-AZ, GCP Cloud
   SQL HA) or Postgres logical replication directly? The former is
   simpler operationally; the latter is portable across clouds.
   Operator decision driven by deployment-platform commitments.

4. **Migration tooling ownership.** Is `argo store migrate-to-
   postgres` a first-class shipped CLI, or a one-time consulting
   engagement we run for early SaaS customers? Affects the
   priority of the Tier 1 work catalog item #6 (operator-facing
   migration polish).

---

This ADR is documentation-only. No code paths, schemas, dependencies,
or CI lanes change as a result of merging it. The decisions are:
keep SQLite for v1, record the migration plan, defer the work.
