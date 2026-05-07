# V1.x — Architectural prep: split handlers/skill.rs

**Owner**: claude. **Depends on**: nothing (pure refactor).

## Scope

`crates/argo-cli/src/handlers/skill.rs` is at exactly 500 LOC,
the project invariant ceiling. Two queued briefs hit this
ceiling already (codex-skill-quality-judge-cli for adding
`quality-trend`, codex-skill-marketplace-publish-cli for adding
`publish/unpublish/versions`). Future skill subcommands will
keep stalling here.

Pure refactor mirroring proven patterns:
- `claude-worker-actions-split` (#851)
- `claude-cli-subcommands-split` (#882)
- `claude-routes-channels-split` (queued)

## Approach

Inspect existing handler structure: many siblings already exist
(skill_import.rs, skill_export.rs, skill_review.rs, skill_search.rs,
skill_quality_*.rs, skill_conflicts.rs). The remaining `skill.rs`
likely contains the central `SkillCmd` dispatch + a few legacy
arms. Cleanest split:

- Keep dispatch + small arms in `skill.rs` (target ≤300 LOC)
- Move long-tail arms to per-domain sibling files
  (e.g. `skill_versions.rs`, `skill_install.rs`, etc.)
- Re-export through `pub use` so callers see flat namespace

After:
- `skill.rs` ≤ 300 LOC
- New sibling files ≤ 150 LOC each
- Headroom for next 5+ skill subcommand additions

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-cli/src/handlers/skill.rs` — reduce to ≤300 LOC;
  keep public re-exports
- `crates/argo-cli/src/handlers/skill_<domain>.rs` (NEW, 2-4
  files depending on natural grouping)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: pub mod
  declarations
- Any handler that imports symbols from `handlers::skill::` —
  narrowly: update import paths if a symbol moved (mechanical;
  should be zero edits if re-exports are right)

## Acceptance criteria
- [ ] `skill.rs` ≤ 300 LOC
- [ ] Each new sibling file ≤ 150 LOC
- [ ] No public-API change (every existing call site still
      compiles unchanged)
- [ ] `cargo build --workspace` clean
- [ ] `cargo test --workspace` — same set of tests pass as
      before (no test deletions; no skips)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean
- [ ] Headroom for the next 5+ skill subcommand additions

## Branch + PR
- Branch: `claude-skill-handlers-split`
- PR title: `refactor: split handlers/skill.rs under 500 LOC budget (claude-skill-handlers-split)`
