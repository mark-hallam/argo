# codex-worker-kpi-cli-split-prep — free worker CLI headroom for KPI

**Owner**: codex. **Depends on**: `claude-worker-charter-storage` MERGED.

**Authored against**: origin/main `ab1fbc40` on 2026-05-08.

## Why

`claude-worker-kpi-tracker` is blocked because
`crates/argo-cli/src/cli_worker_cmds.rs` is exactly 500 LOC on current
main. Adding the `Kpi` subcommand there would violate the hard project
limit. This prep brief performs a no-behavior split so the KPI brief can
later add its CLI surface safely.

## Scope

Pure refactor. Move the existing `WorkerCharterCmd` enum out of
`cli_worker_cmds.rs` into a sibling module and re-export it, matching the
existing `WorkerArchiveCmd` pattern.

Do not add KPI functionality in this prep brief.

## Approach

1. **NEW** `crates/argo-cli/src/cli_worker_charter_cmds.rs`
   containing the existing `WorkerCharterCmd` enum.
2. `crates/argo-cli/src/cli_worker_cmds.rs`: remove the inline enum and
   add `pub use crate::cli::cli_worker_charter_cmds::WorkerCharterCmd;`.
3. `crates/argo-cli/src/cli_reexports.rs`: add the sibling module path.
4. `crates/argo-cli/src/handlers/worker_actions.rs`: adjust imports only
   if the re-export path requires it.
5. Pin `argo worker charter --help` in the existing help smoke coverage
   if it is not already covered.

## Files You May Touch

- `crates/argo-cli/src/cli_worker_cmds.rs`
- `crates/argo-cli/src/cli_worker_charter_cmds.rs` (NEW)
- `crates/argo-cli/src/cli_reexports.rs`
- `crates/argo-cli/src/handlers/worker_actions.rs` (import-only change if
  needed)
- `crates/argo-cli/tests/help_smoke.rs` (only if adding the help row)

## Files You Must NOT Touch

- `CLAUDE.md`
- Migrations
- `crates/argo-store/src/worker_kpi.rs`
- Any KPI implementation files
- Worker charter handlers or storage logic, except for imports required
  by the split

## Acceptance Criteria

- [ ] `cli_worker_cmds.rs` drops below 470 LOC.
- [ ] New `cli_worker_charter_cmds.rs` is below 140 LOC.
- [ ] `argo worker charter --help` output is unchanged in command shape.
- [ ] No KPI command or storage behavior is added.
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.
- [ ] `cargo test -p argo-cli help_smoke` passes.
- [ ] Every modified file is under 500 LOC.

## Branch + PR

- Branch: `codex-worker-kpi-cli-split-prep`
- PR title: `cli: split worker charter commands for KPI headroom (codex-worker-kpi-cli-split-prep)`

