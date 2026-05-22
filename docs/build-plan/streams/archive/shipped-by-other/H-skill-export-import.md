# Stream H — `argo skill export/import`

**Owner**: codex. **Depends on**: H-skill-review-cli (#823),
H-skill-library-routes (#817) merged.

## Scope

- `argo skill export <id> --output <path> [--format md|toml|json]`
- `argo skill import <path> [--lock content_locked]` (sets
  provenance to `imported:<sha256>`)
- Round-trip integration test

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-cli/src/cli_skill_review_cmds.rs` — narrowly:
  extend with Export/Import variants
- `crates/argo-cli/src/handlers/skill_export.rs` (NEW)
- `crates/argo-cli/src/handlers/skill_import.rs` (NEW)
- `crates/argo-cli/tests/skill_export_import.rs` (NEW)

## Acceptance criteria
- [ ] Round-trip preserves frontmatter + body + lock
- [ ] Format flag respected (md is default)
- [ ] Import refuses hash collisions with clear error
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean
- [ ] Every authored file < 500 LOC

## Branch + PR
- Branch: `codex-H-skill-export-import`
- PR title: `cli: argo skill export/import for offline distribution (H-skill-export-import)`
