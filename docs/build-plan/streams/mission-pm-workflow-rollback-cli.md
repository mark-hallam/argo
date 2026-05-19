# Stream P1-7 — workflow versioning + rollback CLI

**Owner**: codex.
**Depends on**: mission-pm-workflow-persistence-schema MERGED, mission-pm-workflow-edit-api MERGED.
**Authored against**: origin/main `80ffa124` on `2026-05-19`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-pm-workflow-rollback-cli`.
**Source**: skycot-pm-worker REPORT.md §3.8 + §11 P1 #7.

## Scope

With the schema (P1 #5) and edit API (P1 #6) in place, expose a CLI for operators to inspect history and roll back a workflow to a prior version. Subcommands:

- `argo workflow list <vertical>` — list workflows + their head version.
- `argo workflow show <vertical> <name> [--version <id>]` — print the TOML body of head (or specified version).
- `argo workflow versions <vertical> <name>` — list every version with `(version, id, created_at, modified_by, lifecycle_state)`.
- `argo workflow rollback <vertical> <name> --to <version-id>` — set head to the named version, confirming with `--yes` or interactive prompt.
- `argo workflow diff <vertical> <name> <version-a> <version-b>` — print a unified TOML diff between two versions.
- `argo workflow archive <vertical> <name> --version <id>` — archive a version.

All subcommands honour `--json` flag (CLAUDE.md project rule).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-cli/src/cli_command_enum.rs` — narrowly scoped: add `Workflow(WorkflowCmd)` variant. No other edits to the enum.
- `crates/argo-cli/src/cli_workflow_cmds.rs` (NEW) — `WorkflowCmd` enum with the 6 subcommand variants.
- `crates/argo-cli/src/handlers/workflow.rs` — narrowly scoped: extend the existing file with new `list`, `show`, `versions`, `rollback`, `diff`, `archive` handler functions. If the file exceeds 500 LOC, split into siblings per the existing `claude-cli-rs-split-v2` pattern.
- `crates/argo-cli/src/handlers/workflow_tests.rs` (NEW or extend existing) — coverage.
- `crates/argo-cli/src/main.rs` — narrowly scoped: dispatch arm for `Workflow(cmd)`. No other edits.

## Files you must NOT touch

- `crates/argo-store/src/workflow_definition.rs` — locked from P1 #5.
- Any gateway routes — the CLI calls the store directly (not via HTTP); the gateway routes are for the web client.
- `crates/argo-vertical/` — workflow parsing is locked.

## Acceptance criteria

- [ ] All 6 subcommands implemented and dispatched.
- [ ] `--json` flag respected on every list/show/status surface (project rule).
- [ ] `rollback` requires `--yes` or interactive confirm (`y/N` prompt). Refusing aborts without mutation.
- [ ] `rollback` emits a `WorkflowHeadChanged` audit row (uses the kind from P1 #6).
- [ ] `diff` uses the `similar` crate (already a workspace dep — verify with `cargo metadata`) for unified diff output; falls back to a naive line-diff if `similar` is unavailable.
- [ ] Capability check: every subcommand checks `workflow.read` or `workflow.edit` (rollback + archive need edit; rest need read). Test the deny path.
- [ ] Tests: 8 cases — (a) list shows current head; (b) versions shows newest-first; (c) show without --version returns head; (d) show with --version returns that version; (e) rollback flips head; (f) rollback without --yes prompts; (g) diff between two versions prints meaningful unified diff; (h) archive flips lifecycle_state.
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean (crate-scoped).
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean — claim explicitly in PR body.
- [ ] `cargo test -p argo-cli` passes.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `codex-mission-pm-workflow-rollback-cli`
- PR title: `cli: argo workflow list/show/versions/rollback/diff/archive (mission-pm-workflow-rollback-cli)`

## Notes for the implementer

- The CLI talks to `argo-store::workflow_definition` directly. No gateway round-trip in process — `argo` is the single binary.
- For `argo workflow show`, default to head when `--version` is absent. Print TOML to stdout for piping (`argo workflow show ... > file.toml`).
- The `diff` subcommand prints to stdout. Add `--json` to emit a structured `{ added: [...], removed: [...], changed: [...] }` envelope for programmatic consumers.
- Audit rows: re-use the kinds from P1 #6 (`WorkflowHeadChanged`, `WorkflowVersionArchived`). Don't add new kinds in this brief.
- If `workflow.rs` is already near the 500-LOC ceiling (it currently has minimal content), the additions may push it over. Split into siblings `workflow_list.rs`, `workflow_show.rs`, etc. on the proven split pattern (see `claude-routes-post-split` precedent).

## Out of scope

- Watching the DB for live edits (no inotify / file-system bridge).
- Rollback that re-runs in-flight sessions against the new head — current sessions keep using the version they bound at start.
- Cross-tenant rollback (operator at tenant A rolling back tenant B's workflow) — capability denies it.
