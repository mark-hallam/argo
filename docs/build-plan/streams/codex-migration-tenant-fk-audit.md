# codex-migration-tenant-fk-audit

**Authored against**: `origin/main` `dff06f840` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-migration-tenant-fk-audit`.

**Fast-mode eligible**: yes — pure mechanical audit.

## Summary

Project invariant: multi-tenant by construction (`tenant_id` plumbed through every primitive per CLAUDE.md). Every table that holds tenant-scoped data should have a `tenant_id` column with appropriate index/foreign-key. Codex audits the 97+ migration files for compliance.

## Acceptance

- [ ] Enumerate every `CREATE TABLE` statement across `crates/argo-store/migrations/*.sql`.
- [ ] For each table, classify:
  - **TENANT-SCOPED-OK** — has `tenant_id` column + INDEX or FOREIGN KEY constraint.
  - **TENANT-SCOPED-MISSING-INDEX** — has tenant_id but no index → slow queries at scale, flag.
  - **GLOBAL** — intentionally not tenant-scoped (e.g. system-level config, migration registry itself) → OK with `// SAFETY: global table because …` comment in the SQL.
  - **AMBIGUOUS** — tenant-scoped by domain but no tenant_id column → BUG, flag for fix.
- [ ] PR body table: migration_file | table_name | classification | note.
- [ ] For each MISSING-INDEX or AMBIGUOUS finding: if ≤3 instances, fix in-brief via additive migration; else spawn follow-up briefs.
- [ ] Confirm `scripts/check-migration-slots.sh` exits 0 after any additive fix.
- [ ] No semantic change to existing tenant-scoping (extend, don't refactor).

## May touch

- `crates/argo-store/migrations/<next-slot>__<fix>.sql` — additive migration if a missing-index fix is ≤3 instances.
- New comment lines in existing migrations to document "// SAFETY: global table …" rationale.

## Must NOT touch

- Existing migration SQL semantics (additive only).
- Production code outside migrations.
- The `migration_registry` infrastructure.

## Notes

- Pre-flight: `ls crates/argo-store/migrations/*.sql | wc -l` to confirm count.
- Pattern reference: tenant_rate_limit (#60), worker_freeze (#119) for canonical tenant-scoped schema.
- Sizing: 100-300 LOC. Audit report bulk + ≤2 small additive migrations.
- v1.0 procurement leverage — "every table has a tenant_id" is what enterprise buyers verify.
