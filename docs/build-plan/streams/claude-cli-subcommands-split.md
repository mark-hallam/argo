# V1.x — Architectural prep: split cli_subcommands.rs

**Owner**: claude. **Depends on**: nothing (pure refactor).

## Scope

`crates/argo-cli/src/cli_subcommands.rs` is exactly at 500
LOC, the project's per-file invariant ceiling. This blocks
new top-level subcommand variants (claude-skill-quality-judge
hit this exact ceiling at tick 107).

Pure refactor. Split the file into focused sibling-file
modules (mirror the `worker_actions_query.rs` split pattern
from claude-worker-actions-split #851). Public API stays
stable so existing 50+ call sites don't have to change.

## Approach

Inspect existing sibling files in `crates/argo-cli/src/`:
- `cli_worker_cmds.rs`
- `cli_session_cmds.rs`
- `cli_skill_review_cmds.rs`
- `cli_skill_conflicts_cmds.rs`
- ... etc.

Extract groups out of the 500-LOC `cli_subcommands.rs`. Most
of the file is one big `#[derive(Subcommand)] pub enum
TopLevelSubcommand { ... }`. Cleanest split:

- Keep the top-level enum + the 5-10 most-common variants
  in `cli_subcommands.rs` (target ≤300 LOC)
- Move the long-tail variant declarations to per-domain
  sibling files (e.g. `cli_subcommands_skill.rs`,
  `cli_subcommands_audit.rs`,
  `cli_subcommands_governance.rs`) — each ≤150 LOC
- Re-export through `pub use` so callers see a flat namespace

After:
- `cli_subcommands.rs` ≤ 300 LOC
- Each new sibling file ≤ 150 LOC
- Headroom for the next 5-10 subcommand additions before
  any sibling itself approaches the 500 ceiling

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-cli/src/cli_subcommands.rs` — reduce to
  ≤300 LOC; keep public re-exports
- `crates/argo-cli/src/cli_subcommands_skill.rs` (NEW) —
  or whatever names the split chooses
- `crates/argo-cli/src/cli_subcommands_audit.rs` (NEW)
- `crates/argo-cli/src/cli_subcommands_governance.rs` (NEW)
- Any cli_*_cmds.rs file that imports symbols from
  cli_subcommands::: narrowly: update import paths if a
  symbol moved (mechanical; should be zero edits if the
  re-exports are right)

## Acceptance criteria
- [ ] `cli_subcommands.rs` ≤ 300 LOC
- [ ] Each new sibling file ≤ 150 LOC
- [ ] No public-API change (every existing call site still
      compiles unchanged)
- [ ] `cargo build --workspace` clean
- [ ] `cargo test --workspace` — same set of tests pass as
      before (no test deletions; no skips)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean
- [ ] `argo --help` output unchanged (pin via test)
- [ ] No changes to existing tests

## Branch + PR
- Branch: `claude-cli-subcommands-split`
- PR title: `refactor: split cli_subcommands.rs under 500 LOC budget (claude-cli-subcommands-split)`
