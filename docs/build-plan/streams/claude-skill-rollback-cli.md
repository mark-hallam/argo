# claude-skill-rollback-cli — `argo skill rollback <id> --to <version>`

**Owner**: claude. **Depends on**: existing `SkillVersion` type
(`crates/argo-skill/src/version.rs`); existing skill versioning A/B story
(claude-skill-versioning-ab-test).

## Why

Promotions go wrong. Without a documented rollback path the operator has to
hand-edit the database or re-publish an old export — both error-prone. A
single command that pins the active body to a known prior version closes the
gap and turns "we have versioning" into "we have versioning we can use under
fire."

## Scope

- `argo skill rollback <id> --to <version>` makes `<version>` the active body
  for `<id>`.
- Refuse rollback when target version doesn't exist OR is itself archived
  (`lifecycle_state = 'archived'` from migration v11). Clear error messages.
- Audit row: every rollback writes a record to `skill_runs` (or, if more
  appropriate, an existing audit/event table) with `kind=rollback`,
  from_version, to_version, actor, timestamp.
- `--dry-run` flag previews the change without persisting.
- `--reason <string>` optional, recorded in the audit row.

## Approach

1. Verify versioning is in place: `grep -rn "SkillVersion" crates/argo-skill/src/version.rs`
   and confirm a `skill_versions` (or equivalent) table exists. If it does not,
   add migration 38 `38__skill_versions.sql` with columns
   `(skill_id TEXT, version TEXT, body TEXT, frontmatter_json TEXT, created_at INTEGER, archived BOOLEAN)`
   plus an index on `(skill_id, version)`. **Use 38, NOT 30 — per build plan.**
2. **NEW** `crates/argo-cli/src/handlers/skill_rollback.rs` (~250 LOC).
3. Helper on `SkillStore`: `pub async fn rollback_to(&self, id: &str, to_version: &str, reason: Option<&str>) -> ArgoResult<RollbackOutcome>`.
4. Update `cli.rs` with the `SkillRollback` clap variant.

## Files you may touch

- BLANKET CLI ALLOWLIST applies.
- `crates/argo-cli/src/handlers/skill_rollback.rs` (NEW, ≤ 280 LOC).
- `crates/argo-cli/src/cli.rs` — narrowly: add `SkillRollback` variant.
- `crates/argo-skill/src/version.rs` — narrowly: add `rollback_to` method (or new sibling file `crates/argo-skill/src/rollback.rs` if it would push version.rs over 500 LOC).
- `crates/argo-store/migrations/38__skill_versions.sql` (ONLY IF migration is missing).
- `crates/argo-cli/tests/skill_rollback.rs` (NEW, ≤ 250 LOC).

## Files you must NOT touch

- Other migrations.
- `crates/argo-skill/src/store.rs` — keep rollback logic next to versioning.
- Marketplace surfaces — rollback is local-only in this brief.

## Acceptance criteria

- [ ] `argo skill rollback <id> --to <version>` succeeds when target version exists.
- [ ] Refuses gracefully when target is missing or archived.
- [ ] Writes an audit row with from/to/actor/reason/timestamp.
- [ ] `--dry-run` previews without writes.
- [ ] 6+ tests: happy path, missing version, archived target, dry-run, audit recorded, JSON output.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] Every authored file < 500 LOC.

## Branch + PR

- Branch: `claude-skill-rollback-cli`
- PR title: `cli: argo skill rollback to a prior version (claude-skill-rollback-cli)`

## Notes

- Run `grep -rn "skill_versions\|skill_v\b" crates/argo-store/migrations/`
  before deciding whether to add the migration.
- Rollback is NOT a delete; the previously-active version stays in the table
  and can be rolled forward to.
