# Claude — Skill conflict resolution

**Owner**: claude. **Depends on**: H-4b (#803),
claude-marketplace-vertical-publish (#821), H-skill-review-cli
(#823) merged.

## Scope

H-4b's two-tier lookup picks workspace > library on name
collision (silently). Add UI for operators to resolve:

- `argo skill conflicts list` → table of name-collisions
- `argo skill conflicts show <name>` → side-by-side diff
- `argo skill conflicts resolve <name> --keep workspace|library`
  → flips lifecycle so only one remains active

`--json` everywhere.

## Files you may touch
- `crates/argo-skill/src/conflicts.rs` (NEW)
- `crates/argo-skill/src/lib.rs` — narrowly: re-export
- `crates/argo-cli/src/cli_skill_review_cmds.rs` — narrowly:
  Conflicts subcommand
- `crates/argo-cli/src/handlers/skill_conflicts.rs` (NEW)
- BLANKET CLI ALLOWLIST applies
- `crates/argo-cli/tests/skill_conflicts.rs` (NEW)

## Acceptance criteria
- [ ] `list` finds collisions
- [ ] `resolve` flips lifecycle (loser → superseded)
- [ ] `show` produces unified-diff output
- [ ] `--json` everywhere
- [ ] Integration test with 2 conflicting skills
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean
- [ ] Every authored file < 500 LOC

## Branch + PR
- Branch: `claude-skill-conflict-resolution`
- PR title: `skill: conflicts list/show/resolve for layered-lookup collisions (claude-skill-conflict-resolution)`
