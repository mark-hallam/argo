# claude-cli-subcommands-skill-split — split cli_subcommands_skill.rs (already OVER 500 LOC)

**Owner**: claude. **Depends on**: nothing (pure refactor).
**Authored against**: origin/main `d57d3a31` on `2026-05-13`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-cli-subcommands-skill-split`.

## Scope

Pure refactor. `crates/argo-cli/src/cli_subcommands_skill.rs` is at **596 LOC** — **already over** the 500-LOC hard ceiling. This is a pre-existing HARD RULE 6 violation that slipped past CI (CI is currently disabled per BUILD_STATUS.md header; will re-fire when re-enabled). Splitting it now also unblocks:

- mission-skill-publish-verified-ingest (and any future skill-CLI brief)

The file's bulk is the SkillCmd enum + 3 trailing `*Cmd` struct definitions (`SkillPublishCmd` at line 551, `SkillUnpublishCmd` at line 572, `SkillTagsCmd` at line 584). These three structs together are ~75 LOC and a clean extract target.

## Approach

Extract `SkillPublishCmd` + `SkillUnpublishCmd` + `SkillTagsCmd` (with their `#[derive(Args, Debug)]` attrs and the trailing `}` of each struct body) into a new sibling `crates/argo-cli/src/cli_subcommands_skill_publish.rs`. Re-export from `cli_subcommands_skill.rs` via `pub use cli_subcommands_skill_publish::{SkillPublishCmd, SkillUnpublishCmd, SkillTagsCmd};`.

Move only the `use` lines those three structs need. The `clap::Args` import and any path-imports (`PathBuf` etc.) will need to be present in both files; duplicate the `use` lines.

After split:
- `cli_subcommands_skill.rs` ≤ 525 LOC (target: ~480, comfortable headroom)
- `cli_subcommands_skill_publish.rs` ≤ 100 LOC

If 525 LOC still feels tight (only 25 lines of headroom under 500), the implementer should additionally extract the next-largest variant cluster (the dependency-graph / quality / explanation subcommand structs higher up in the file — inspect line-range and pick the cohesive group). Stop when `cli_subcommands_skill.rs` ≤ 480 LOC.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- BLANKET CLI ALLOWLIST applies (per existing `claude-cli-subcommands-split` precedent).
- `crates/argo-cli/src/cli_subcommands_skill.rs` — remove the 3 trailing structs + (optionally) more if needed to hit ≤480 LOC.
- `<NEW: crates/argo-cli/src/cli_subcommands_skill_publish.rs>` — receives the extracted structs verbatim.
- `crates/argo-cli/src/lib.rs` — narrowly scoped: add ONE `pub mod cli_subcommands_skill_publish;` line (next to existing `cli_subcommands_skill`). Only this one line.

## Files you must NOT touch

- The `SkillCmd` enum itself — variants stay declared in `cli_subcommands_skill.rs`. Only the `*Cmd` *struct* definitions referenced by those variants move.
- Any `crates/argo-cli/src/handlers/skill_*.rs` handler file — they import via the path that the re-export preserves.
- Tests.

## Acceptance criteria

- [ ] `cli_subcommands_skill.rs` ≤ 480 LOC after the split (real headroom, not borderline).
- [ ] `cli_subcommands_skill_publish.rs` ≤ 200 LOC.
- [ ] `pub use cli_subcommands_skill_publish::{SkillPublishCmd, SkillUnpublishCmd, SkillTagsCmd};` re-export in `cli_subcommands_skill.rs` so existing call sites keep working unchanged.
- [ ] Zero behavior change — `argo skill publish` / `argo skill unpublish` / `argo skill tags` work identically.
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean — claim explicitly in PR body.
- [ ] `cargo test -p argo-cli` passes.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-cli-subcommands-skill-split`
- PR title: `cli: extract cli_subcommands_skill_publish sibling — file was 596 LOC (claude-cli-subcommands-skill-split)`

## Notes for the implementer

- Mechanical. Move the structs verbatim, including their full `#[derive(...)]` + doc comments. Don't rename, don't reorder fields.
- If the final `cli_subcommands_skill.rs` ends up above 480 LOC after extracting the 3 trailing structs (the math says ~525 LOC, which is still over the ceiling), do a SECOND pass extracting another cohesive cluster — pick the cluster that's currently the most isolated (no inter-struct dependencies). Document the cluster you picked in the PR body.
- The split-prep is `*-split`-suffixed so the pre-claim ceiling check auto-skips.
- This brief is unusual — the file is already 596/500 OVER the ceiling. The split is mandatory not just preventive.
