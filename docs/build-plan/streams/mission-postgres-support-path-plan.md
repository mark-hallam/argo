# Mission load-bearing wire — PostgreSQL support path planning ADR (doc-only)

**Owner**: claude. **Depends on**: nothing — pure design ADR.
**Authored against**: origin/main `b908e563` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-postgres-support-path-plan`.

> SOURCE: MISSION_BACKLOG.md Section 1 #29 — "Plan PostgreSQL support path (long-tail SaaS)." SQLite-single-binary is the v1 architecture and CLAUDE.md "MAINTAIN: single-binary architecture" rule stays. But for multi-tenant SaaS at >100 tenants the SQLite write-lock contention becomes a soft ceiling. Mission goal: an ADR documenting the migration path WITHOUT requiring any code change today.

## Scope

This is a **doc-only ADR**. No Rust crate touches, no schema changes, no tests beyond the ADR itself.

Author `docs/build-plan/streams/adr-postgres-support-path.md` (NEW, ≤ 400 LOC) with these sections:

1. **Status / context**: SQLite is v1; PostgreSQL is for long-tail SaaS scale (>100 active tenants, sustained write load, or cross-region replication need).

2. **Three-tier proposal**:
   - **Tier 0 (v1, today)**: single-binary SQLite. No change.
   - **Tier 1 (post-v1.x)**: optional PostgreSQL backend behind a `--features postgres` build flag. `argo-store` exposes a `Pool<DB>` generic over `sqlx::Sqlite | sqlx::Postgres`. NO migration of existing operators — opt-in.
   - **Tier 2 (post-v2.x)**: PostgreSQL becomes the default for SaaS deployment; SQLite remains the default for self-hosted single-tenant.

3. **Engineering work catalog**:
   - `argo-store` generic-over-driver refactor (every `sqlx::query()` audit).
   - Migration system: SQLite has `PRAGMA table_info`; Postgres needs `information_schema.columns`. Adapter layer.
   - Test matrix: every `cargo test -p argo-store` runs against both backends in CI.
   - Connection pooling: sqlx handles both, but timeout/idle semantics differ.

4. **Operator-facing migration**: `argo store migrate-to-postgres --dsn <url>` (future CLI). Exports SQLite db → schema-equivalent Postgres dump → re-imports. ADR enumerates the gotchas: rowid behavior, AUTOINCREMENT vs SERIAL, JSON1 vs JSONB, datetime semantics.

5. **Risk register**:
   - **Test coverage doubling**: every test runs against both engines.
   - **Migration churn**: every new migration must work on both.
   - **Surface drift**: SQLite-specific tricks (e.g. STRICT tables, PRAGMA hacks) leak into the abstraction.
   - **Operator complexity**: "which engine should I use?" support burden.

6. **Decision**: ACCEPT Tier 0 status for v1. Tier 1 work is approved-in-principle but not scheduled. Tier 2 is post-v2 and requires a follow-up ADR.

7. **Open questions** (operator decides later):
   - Will SQLite stay the canonical self-hosted default forever, or is Tier 2 the eventual world?
   - What's the threshold to flip Tier 1 from "available" to "recommended for SaaS"?
   - Cross-region replication: AWS RDS multi-AZ vs Postgres logical replication?

This is NOT in scope: actual driver-abstraction implementation; any code change; benchmark data (referenced but not collected here).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `docs/build-plan/streams/adr-postgres-support-path.md` (NEW, ≤ 400 LOC) — the ADR itself.

## Files you must NOT touch

- Any `crates/**` source
- Any other doc beyond the listed ADR file
- `crates/argo-store/migrations/**`

## Acceptance criteria

- [ ] `docs/build-plan/streams/adr-postgres-support-path.md` exists with all 7 sections
- [ ] ADR lists 3 tiers + concrete decision (ACCEPT Tier 0; Tier 1 approved-in-principle)
- [ ] Risk register has ≥ 4 items
- [ ] Operator-facing migration `argo store migrate-to-postgres` is sketched but NOT implemented
- [ ] No code changes
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-postgres-support-path-plan`
- PR title: `docs: ADR for PostgreSQL support path (mission-postgres-support-path-plan)`

## Notes for the implementer

- This is a documentation brief, not an architectural commitment. The ACCEPT decision applies to Tier 0 only.
- For the operator-facing migration: just sketch the CLI shape + enumerate edge cases. NO implementation.
- Be honest about cost: Tier 1 doubles the test matrix. The ADR should not pretend this is free.
- Reference: SQLite docs say "if you don't need >1 writer, SQLite is the answer." Use this as the operator's first decision point.
- The "post-v1.x" / "post-v2.x" phrasing is intentional — no specific dates.
