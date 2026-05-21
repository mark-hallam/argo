# codex-tenant-fk-ambiguous-followups

**Authored against**: `origin/main` `b566866d4` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-tenant-fk-ambiguous-followups`.

**Fast-mode eligible**: yes — mechanical follow-up sweep.

## Summary

#1988 (`codex-migration-tenant-fk-audit`) audited 97+ migrations and SAFETY-commented 12 GLOBAL tables. The PR body declared **17 AMBIGUOUS findings** — tables that are domain-tenant-scoped but had unclear scoping in the SQL. Those 17 were deferred to follow-ups (correctly under the ≤3-in-brief budget). This brief addresses them.

Pre-condition: **#1988 MERGED ✅**.

## Acceptance

- [ ] Read #1988 PR body to extract the 17 AMBIGUOUS table names.
- [ ] For each, inspect the current migration + the table's callsites (`rg -l '<table_name>' crates/argo-store/`).
- [ ] Classify each:
  - **BUG** — table has tenant-scoped data but no `tenant_id` column / no index. Add via additive migration (next free slot via `bash scripts/check-migration-slots.sh`).
  - **MISSING-INDEX** — has `tenant_id` but no index → add index via additive migration.
  - **GLOBAL-ACCEPTED** — actually intentionally global; downgrade to GLOBAL with `// SAFETY:` comment (continuation of #1988's pattern).
- [ ] Each migration follows [docs/skycot/operator/migrations.md](docs/skycot/operator/migrations.md) — additive only, next free slot, registered in migration registry.
- [ ] Each BUG fix adds at least one regression test (mirror #1989's pattern: cross-tenant `select` returns 0 rows / 404).
- [ ] PR body restates the 17 AMBIGUOUS table list with the chosen classification per row.
- [ ] All existing tests pass.

## May touch

- New migration files under `crates/argo-store/migrations/<slot>__<fix>.sql`
- Test files for the regression coverage (additive)
- Migration registry entry per new migration
- Tenant-filter SQL in handlers where a BUG is fixed at the query layer rather than the schema layer (case-by-case judgment)

## Must NOT touch

- The 12 GLOBAL tables #1988 already settled.
- Tables in the 3 production handlers fixed by #1992 (audit_detail / audit_replay / retros).
- Migration files that #1988 already commented (settled).
- Production code outside the tenant-filter fixes (no scope creep).

## Notes

- Pre-flight: `gh pr view 1988 --json body --jq '.body' | rg -A 1 AMBIGUOUS` to extract the 17 names.
- Sizing: 200-500 LOC depending on how many BUGs require additive migrations. If all 17 are GLOBAL-ACCEPTED, the brief is just comment additions.
- v1.0 procurement leverage — close the tenant-isolation audit chain end-to-end.
