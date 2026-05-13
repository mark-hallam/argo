# mission-skill-version-migration-publisher-check — gate `record_migration` on publisher tenant ownership

**Owner**: claude. **Depends on**: nothing.
**Authored against**: origin/main `bf8236e3` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-skill-version-migration-publisher-check`.

> 🔒 **SECURITY 2026-05-14 by conductor (CEO-authorized Phase 2).** Audit cluster 3 found `argo-skill/src/version_migration.rs:45` has a TODO: gate `record_migration` on `publisher_tenant_id == skill.publisher_tenant_id`. **Today any tenant can record a migration row for any skill family.** Multi-tenant skill marketplace isolation gap.

## Scope

Add a publisher-ownership check inside `SkillStore::record_migration` (or wherever the function lives — verify at claim time via `rg 'pub.*fn record_migration' crates/argo-skill/src/`). The check:

1. Look up the skill family's `publisher_tenant_id` (the original publisher of `version 0` or the first published version, depending on schema).
2. Compare against the caller's `tenant_id` (passed in via `Principal` or a typed parameter).
3. If mismatch → return `Err(ArgoError::CapabilityDenied(Capability::SkillMigrateRecord))` (or a new `ArgoError::PublisherMismatch` if a typed variant is preferred).
4. If match → proceed with the existing insert.

Audit emission: emit an audit-chain row with `AuditKind::SkillMigrateRecord` (or whichever existing variant is closest) on both Allow + Reject paths. Tamper-evidence for cross-tenant attempts.

**Not in scope**: backfilling `publisher_tenant_id` for legacy skill rows that don't have one (separate migration brief if needed); changing the migration-recording API signature; adding the `SkillMigrateRecord` capability to operator default roles (operator may want to keep this admin-only).

## Files you may touch

- `crates/argo-skill/src/version_migration.rs` (line 45 + surrounding) — narrowly: add the ownership check + audit emission. ≤50 modified lines.
- `crates/argo-skill/src/version_migration_tests.rs` (or sibling) — add 3 tests: same-tenant Allow, cross-tenant Reject, audit row emitted on both paths.
- `crates/argo-types/src/capability.rs` — narrowly: ONE new `Capability` variant if `SkillMigrateRecord` doesn't exist. ≤5 modified lines.

## Files you must NOT touch

- The migration-recording call sites — they pass in the principal/tenant_id; verify they already do, otherwise flag in PR body.
- `argo-store::skill_meta` (or wherever `publisher_tenant_id` lives) — read-only.
- Any audit-chain machinery — use the existing `AuditSink`.

## Acceptance criteria

- [ ] `record_migration` returns `Err(CapabilityDenied)` when caller's `tenant_id` mismatches the skill family's `publisher_tenant_id`.
- [ ] Audit row emitted on both Allow + Reject paths.
- [ ] 3 unit tests: same-tenant happy path; cross-tenant rejected; audit row appears in store.
- [ ] **No regression for legitimate publishers** — same-tenant `record_migration` still works exactly as before.
- [ ] `cargo clippy -p argo-skill --all-targets -- -D warnings` clean.
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] `cargo test -p argo-skill` passes.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-mission-skill-version-migration-publisher-check`
- PR title: `skill: gate record_migration on publisher tenant ownership (mission-skill-version-migration-publisher-check)`

## Notes for the implementer

- ~50 LOC core; smallest security brief in the queue. ~1 hour worker time.
- If `publisher_tenant_id` doesn't exist on the skill row, BLOCK and surface the precursor (need to add the column first).
- This is Phase 2 of the CEO-authorized cleanup. Multi-tenant invariant per CLAUDE.md.
