# V1.3 — Workflow DAG executor

**Owner**: claude. **Depends on**: E-identity (#812),
H-handoff merged.

## Scope

Declarative multi-step workflows across multiple workers.
A workflow is a YAML describing a DAG of tasks; each task
specifies a worker (or persona-match) + inputs + outputs
referencing upstream tasks. The DAG executor schedules tasks
when their deps are satisfied.

This is the v1.3 multi-worker orchestration foundation —
v1 ships Session/Handoff/EscalationChain primitives but no
declarative workflow.

- New crate or module `argo-workflow` with:
  - `Workflow { tasks: Vec<TaskNode>, edges: Vec<Edge> }`
  - `TaskNode { id, worker_match, inputs: Map<String,
    Expr>, outputs: Map<String, Schema> }`
  - `Edge { from_task, to_task, on: Trigger }`
- YAML schema + parser
- DAG executor using existing Session primitive (each task →
  one Session)
- Output binding: downstream `inputs.<key>: ${tasks.foo.out.bar}`
- New CLI: `argo workflow validate <yaml>`,
  `argo workflow run <yaml> [--dry-run]`,
  `argo workflow status <run-id>`,
  `argo workflow list-runs`

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- EXHAUSTIVE-MATCH amendment applies
- `crates/argo-workflow/` (NEW crate, ~1500 LOC cap)
  - `Cargo.toml`, `lib.rs`, `dag.rs`, `executor.rs`,
    `yaml.rs`, `expr.rs`
- `crates/argo-store/migrations/<next>__workflow_runs.sql`
  (NEW)
- `crates/argo-cli/src/handlers/workflow_*.rs` (4 NEW)
- `crates/argo-cli/src/cli_workflow_cmds.rs` (NEW)
- `crates/argo-cli/src/cli.rs` — narrowly: register
  `Workflow` command
- `Cargo.toml` workspace — narrowly: add `argo-workflow`
  member

## Acceptance criteria
- [ ] YAML parses + validates topology (no cycles)
- [ ] DAG runs schedule tasks with deps satisfied
- [ ] Output binding evaluates from upstream task results
- [ ] Failed task fails downstream; --dry-run lists plan
- [ ] `--json` universal output
- [ ] 12 unit tests (parse, cycle detect, executor) + 2
      integration tests (3-task linear, 4-task diamond)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR
- Branch: `claude-workflow-dag`
- PR title: `workflow: declarative DAG executor for multi-worker tasks (claude-workflow-dag)`
