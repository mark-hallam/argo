# mvp-split-cli-command-enum — split cli_command_enum.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Tier 1 MVP — proven split pattern.**

**Authored against**: origin/main 7fdbbdb8 on 2026-05-10.

## Why

`crates/argo-cli/src/cli_command_enum.rs` is at **512 LOC** — 12 over the 500-LOC ceiling. The top-level clap `Command` enum has grown with each new CLI surface (recent: `argo cu …`, `argo eval validate …` planned). Pure refactor.

## Approach

1. Read `cli_command_enum.rs` end-to-end. The file is mostly one giant enum + serde derives. Group variants by subsystem (e.g., worker vs. session vs. memory vs. gateway/admin vs. audit).
2. Strategy: extract one cohesive group of variants into a sibling enum, then have the top-level `Command` enum carry a wrapper variant `Worker(WorkerCmds)` that dispatches into the sibling. This keeps the per-file LOC tight and matches the existing `cli_worker_cmds.rs` pattern.
3. Re-export sibling enum from `cli_reexports.rs` if needed.

Target: `cli_command_enum.rs` ≤ 350 LOC.

## Files you may touch

- `crates/argo-cli/src/cli_command_enum.rs`
- `crates/argo-cli/src/cli_<topic>_cmds.rs` (NEW, ≤ 250 LOC) — pick one cohesive subsystem
- `crates/argo-cli/src/cli_reexports.rs` — narrowly: re-export new enum
- `crates/argo-cli/src/main.rs` — narrowly: dispatch routing through wrapper variant

## Files you must NOT touch

- `crates/argo-cli/src/handlers/*.rs` — handler logic stays put

## Acceptance criteria

- [ ] `cli_command_enum.rs` ≤ 350 LOC
- [ ] Sibling enum file ≤ 250 LOC
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] `argo --help` output is identical pre/post split (subcommand list unchanged)
- [ ] `cargo test -p argo-cli` clean

## Branch + PR

- Branch: `mvp-split-cli-command-enum`
- PR title: `cli: split cli_command_enum.rs below 500-LOC ceiling (mvp-split-cli-command-enum)`
