# Mission load-bearing wire — skill version migration path (v1 → v2 with downstream notify)

**Owner**: claude. **Depends on**: `argo-skill::version::{SkillVersion, find_version, list_versions}` SHIPPED (verified at `crates/argo-skill/src/version.rs:20,50`); `argo-skill::SkillStore::install` SHIPPED; existing skill audit primitives.
**Authored against**: origin/main `939e3bec` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-skill-version-migration-path`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 4 #79 — "Build skill version migration path (v1 → v2 → downstream notify)." Today multiple skill versions can coexist via `SkillVersion` rows but there is NO supported flow for a publisher (or a Forge-generated improvement) to mark v2 as the successor of v1 + notify tenants that have v1 installed. Mission promise: `argo skill migrate <id> --from v1 --to v2 [--auto-upgrade]` writes a migration row + each tenant-installed instance learns about it on next session boot.

## Scope

1. **NEW migration**: `<next-slot>__skill_version_migration.sql`:
   ```sql
   CREATE TABLE skill_version_migration (
       skill_id BLOB NOT NULL,
       from_version TEXT NOT NULL,
       to_version TEXT NOT NULL,
       publisher_tenant_id BLOB NOT NULL,
       reason TEXT,
       auto_upgrade INTEGER NOT NULL DEFAULT 0,  -- bool
       created_at INTEGER NOT NULL,
       PRIMARY KEY (skill_id, from_version, to_version)
   );
   CREATE INDEX idx_skill_version_migration_to ON skill_version_migration(skill_id, to_version);
   ```
   Versioned counterpart `v<next>.rs`; bump `EXPECTED_SCHEMA_VERSION`.

2. **`SkillStore` API additions** (sibling module `version_migration.rs`):
   - `record_migration(skill_id, from, to, publisher, reason, auto_upgrade) -> ArgoResult<()>` — idempotent on PK.
   - `lookup_migration(skill_id, installed_version) -> Option<MigrationRow>` — used at session-boot to spot v1→v2 paths.
   - `list_for_skill(skill_id) -> Vec<MigrationRow>` — operator visibility.

3. **NEW CLI**: `argo skill migrate <skill_id> --from <v> --to <v> [--auto-upgrade] [--reason "..."]` — publisher-side command. Records the migration row. Future call from Forge promote pipeline is a separate wire (out of scope here).

4. **NEW CLI**: `argo skill upgrade <skill_id> [--dry-run] [--json]` — tenant-side command. Looks up `lookup_migration(skill_id, current_version)`. If found, prints summary; if `auto_upgrade=true` OR `--yes` flag, calls `SkillStore::install` with the new version + audit row "upgraded from v X to v Y per migration record". `--dry-run` only prints what would happen.

5. **Session-boot hint**: at agent session boot (find the boot path; likely `argo-agent-runner::turn`), for each installed skill the worker uses, call `lookup_migration(skill_id, installed_version)`. If any migrations are pending, emit ONE `tracing::info!` event `"skill_migration_available skill={} from={} to={} auto={}"` — NO automatic side effects in this brief. The event is for operator dashboards / `argo doctor`.

6. **`argo skill list-migrations` `[--json]`** — list ALL pending migrations across the tenant's installed skills. Operator dashboard input.

This is NOT in scope: actual auto-upgrade execution at session boot (the hint is emitted, but upgrade still requires `argo skill upgrade`; separate brief can wire automation); cross-tenant migration propagation (separate brief — `mission-cross-tenant-trust-graph`); deprecation warnings on old versions; rollback flow.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-store/migrations/<next-slot>__skill_version_migration.sql` (NEW) — verify slot.
- `crates/argo-store/src/migrations/v<next>.rs` (NEW) — versioned migration.
- `crates/argo-store/src/lib.rs` — narrowly scoped: bump `EXPECTED_SCHEMA_VERSION` (currently 47).
- `crates/argo-skill/src/version_migration.rs` (NEW, ≤ 250 LOC) — the 3 new methods + `MigrationRow` struct.
- `crates/argo-skill/src/version_migration_tests.rs` (NEW, sibling, ≤ 250 LOC) — 7 unit tests: record-roundtrip; idempotent-on-pk; lookup-by-installed-version; lookup-no-match-returns-none; list-for-skill-multi-rows; auto-upgrade-flag-roundtrip; reason-text-length-cap.
- `crates/argo-skill/src/lib.rs` — narrowly scoped: re-export `version_migration::*`.
- `crates/argo-cli/src/handlers/skill_migrate.rs` (NEW, ≤ 200 LOC) — publisher CLI.
- `crates/argo-cli/src/handlers/skill_upgrade.rs` (NEW, ≤ 250 LOC) — tenant CLI w/ dry-run.
- `crates/argo-cli/src/handlers/skill_list_migrations.rs` (NEW, ≤ 150 LOC) — list CLI.
- `crates/argo-cli/src/cli_subcommands_skill.rs` (verify path) — narrowly scoped: ADD `Migrate`, `Upgrade`, `ListMigrations` subcommand variants.
- `crates/argo-agent-runner/src/turn_skills.rs` OR similar (verify in first 5min) — narrowly scoped: ADD one tracing emission per installed skill where `lookup_migration` returns Some. ≤ 25 LOC delta. If this would breach 500 LOC, spill to `turn_skills_migration.rs` (NEW).

## Files you must NOT touch

- `crates/argo-skill/src/version.rs` — `find_version`/`list_versions` stable; reuse only
- `crates/argo-skill/src/store.rs` `install` — reuse from `argo skill upgrade` handler only
- Any non-store/non-skill/non-cli/non-agent-runner crate

## Acceptance criteria

- [ ] Migration adds `skill_version_migration` table with composite PK
- [ ] EXPECTED_SCHEMA_VERSION bumped
- [ ] 3 new `argo-skill` methods + `MigrationRow` struct exist
- [ ] `argo skill migrate <id> --from v1 --to v2` records the row
- [ ] `argo skill upgrade <id> --dry-run` prints summary without writing
- [ ] `argo skill upgrade <id> --yes` (or `auto_upgrade=true`) calls `install` + writes audit row
- [ ] `argo skill list-migrations --json` JSON envelope shape stable
- [ ] Session boot emits `tracing::info!` for each pending migration (no side effect)
- [ ] 7 unit tests + 3 CLI unit tests + 1 integration smoke (migrate + upgrade --dry-run)
- [ ] No new public API outside the listed exports
- [ ] `cargo build -p argo-store -p argo-skill -p argo-cli -p argo-agent-runner` clean
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-skill-version-migration-path`
- PR title: `skill: skill_version_migration storage + argo skill migrate/upgrade/list-migrations CLI (mission-skill-version-migration-path)`

## Notes for the implementer

- Migration slot: verify in-flight briefs aren't taking the next slot. Multiple `mission-*` briefs stage migrations.
- The session-boot hint is intentionally low-impact: just a `tracing::info!`. Don't try to inject the available-upgrade into the agent prompt or trigger auto-install — that's the next brief.
- `lookup_migration` query should use the indexed `(skill_id, from_version)` pattern, not table-scan. Verify with EXPLAIN QUERY PLAN.
- For the publisher-tenant: only the publisher should be able to record a migration for their own skill. Add a publisher-tenant-matches check OR document that as a TODO in the PR body (skill ownership enforcement is a separate concern).
- BLOCK and request revision rather than touch `version.rs` internals.
