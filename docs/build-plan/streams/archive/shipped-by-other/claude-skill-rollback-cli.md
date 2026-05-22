# claude-skill-rollback-cli — `argo skill rollback <name> --to <version>`

**Owner**: codex. **Depends on**: `claude-skill-versioning-ab-test`
MERGED.

**Authored against**: origin/main `ab1fbc40` on 2026-05-08.

## Conductor Rewrite

Rewritten on 2026-05-08 after worker validation found stale storage
premises. Current main stores versions in the existing `skill` table
(`name`, `version`, `superseded_by`, `lifecycle_state`) and already has
`SkillStore::promote(name, version)`. There is no `skill_versions` or
`skill_runs` table to target. Do not add a migration.

## Why

Promotions can go wrong. Operators need a documented rollback path that
flips the active skill pointer back to a known prior version without
hand-editing SQLite or re-importing an old artifact.

## Scope

- Add `argo skill rollback <name> --to <version>`.
- Resolve the current active row with `SkillStore::find_active_by_name`.
- Resolve candidate versions with `SkillStore::versions_for_name`.
- Refuse rollback when:
  - the skill name is unknown,
  - the target version is missing,
  - the target row has `lifecycle_state = archived` or `rejected`.
- `--dry-run` prints the from/to transition without changing rows.
- `--reason <text>` is optional and recorded in the audit payload.
- On write, call existing `SkillStore::promote(name, version)`.
- Add typed audit event `skill.rollback` through `AuditSink`.

## Approach

1. `crates/argo-cli/src/cli_subcommands_skill.rs`: add a `Rollback`
   variant under `SkillCmd`.
2. `crates/argo-cli/src/handlers/skill.rs`: dispatch to a new handler.
3. **NEW** `crates/argo-cli/src/handlers/skill_rollback.rs`
   (<= 260 LOC): validation, dry-run output, JSON output, call to
   `SkillStore::promote`, audit emission.
4. `crates/argo-audit/src/event.rs`: add `AuditKind::SkillRolledBack`
   with wire label `skill.rollback`; update parser/listing tests as
   required by exhaustive matches.
5. Add CLI smoke tests.

## Files You May Touch

- `crates/argo-cli/src/cli_subcommands_skill.rs`
- `crates/argo-cli/src/handlers/skill.rs`
- `crates/argo-cli/src/handlers/mod.rs`
- `crates/argo-cli/src/handlers/skill_rollback.rs` (NEW)
- `crates/argo-cli/tests/skill_rollback.rs` (NEW)
- `crates/argo-audit/src/event.rs`
- `crates/argo-audit/src/tests.rs` (only for the new event label, if
  needed)

## Files You Must NOT Touch

- `CLAUDE.md`
- Migration files
- `crates/argo-skill/src/store.rs` (use existing public methods)
- `crates/argo-skill/src/version.rs` (read-only for reference)
- Marketplace / OCI surfaces

## Acceptance Criteria

- [ ] `argo skill rollback <name> --to <version>` makes that version
      active when it exists and is rollback-eligible.
- [ ] Unknown name, missing version, archived target, and rejected target
      fail with clear errors.
- [ ] `--dry-run` previews from_version/to_version without writes.
- [ ] `--reason` appears in the `skill.rollback` audit payload.
- [ ] JSON output includes `{ok, dry_run, name, from_version, to_version}`.
- [ ] 6+ tests cover happy path, missing version, archived/rejected
      target, dry-run, audit payload, and JSON output.
- [ ] `cargo clippy -p argo-audit --all-targets -- -D warnings` clean.
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.
- [ ] Every modified file is under 500 LOC.

## Branch + PR

- Branch: `codex-claude-skill-rollback-cli`
- PR title: `cli: rollback active skill to a prior version (claude-skill-rollback-cli)`
