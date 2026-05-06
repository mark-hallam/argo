# V1.3 — Workflow checkpoint + resume

**Owner**: claude. **Depends on**: claude-workflow-dag (#857)
merged.

## Scope

Long-running workflows (the v1.3 DAGs from #857) need to
survive process restarts, crashes, machine reboots. Without
checkpointing, a 4-hour workflow that hits a transient error
at the 3.5-hour mark loses everything.

Add checkpoint/resume to argo-workflow:

- After each task completes, persist its output + the DAG's
  current frontier (which tasks are queued, running, done) to
  `workflow_checkpoint` table
- On startup, scan for non-terminal `workflow_run` rows and
  resume from their last checkpoint
- New CLI: `argo workflow resume <run-id>`,
  `argo workflow checkpoints <run-id>` (list snapshots),
  `argo workflow rollback <run-id> --to-checkpoint <id>`

- Per-task idempotency key: tasks declare a `cache_key`; if
  resume sees the cache_key already present, skip re-execution
  and use cached output (saves money on expensive LLM calls)

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-workflow/src/checkpoint.rs` (NEW)
- `crates/argo-workflow/src/executor.rs` — narrowly: hook
  per-task checkpoint write + on-startup resume scan
- `crates/argo-store/migrations/<next>__workflow_checkpoints.sql`
  (NEW)
- `crates/argo-store/src/workflow_checkpoint.rs` (NEW)
- `crates/argo-cli/src/handlers/workflow_resume.rs` (NEW)
- `crates/argo-cli/src/handlers/workflow_checkpoints.rs` (NEW)
- `crates/argo-cli/src/handlers/workflow_rollback.rs` (NEW)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 3 new
  pub mods
- `crates/argo-cli/src/cli_workflow_cmds.rs` — narrowly:
  3 new variants

## Acceptance criteria
- [ ] Per-task checkpoint persisted on completion
- [ ] On startup, resumes pending workflows from last checkpoint
- [ ] `argo workflow resume <run-id>` works
- [ ] `cache_key` idempotency skips re-execution
- [ ] Rollback to a specific checkpoint works
- [ ] `--json` envelope across all 3 new subcommands
- [ ] 10 unit tests + 2 integration tests (crash-mid-run + resume)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `claude-checkpoint-resume`
- PR title: `workflow: checkpoint + resume + idempotency (claude-checkpoint-resume)`
