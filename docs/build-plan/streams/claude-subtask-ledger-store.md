# V1.3 — Subtask ledger store

**Owner**: claude. **Depends on**: claude-runtime-planner-trait
(in flight) merged. From dreams + runtime-swarm investigation
(`DREAMS_AND_RUNTIME_SWARM.md` Section 4).

## Scope

The Planner emits subtask trees. Persist them so:
- Decompositions are auditable (which goal → which subtasks?)
- Restarts can resume mid-decomposition (durability)
- Operators can inspect a worker's plan via CLI
- Future Dreams integration (v2.0) can analyze planner cohorts

New `subtask_ledger` table tracks each subtask + its parent
goal + completion state.

## Schema

```sql
CREATE TABLE subtask_ledger (
    id TEXT PRIMARY KEY,
    parent_goal TEXT NOT NULL,
    parent_session_id TEXT,
    worker_id TEXT NOT NULL,
    plan_node_path TEXT NOT NULL,  -- DAG path like "/draft/copy"
    description TEXT NOT NULL,
    state TEXT NOT NULL,            -- pending/running/done/failed/skipped
    output JSONB,
    started_at TIMESTAMP,
    completed_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

## Files you may touch
- `crates/argo-store/migrations/<next>__subtask_ledger.sql` (NEW)
- `crates/argo-store/src/subtask.rs` (NEW)
- `crates/argo-store/src/lib.rs` — narrowly: re-export
- `crates/argo-runtime/src/planner.rs` — narrowly: write through
  to subtask_ledger on decompose/transition
- `crates/argo-cli/src/handlers/subtask_*.rs` (3 NEW — list/show/tree)
- `crates/argo-cli/src/cli_subtask_cmds.rs` (NEW)
- `crates/argo-cli/src/cli_subcommands.rs` — narrowly: register

## Acceptance criteria
- [ ] subtask_ledger table created via migration
- [ ] Planner writes subtask rows on decompose
- [ ] State transitions persist (pending → running → done/failed)
- [ ] `argo subtask list/show <id>/tree <session>` work
- [ ] `--json` envelope universal
- [ ] 6 unit tests + 1 integration test
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `claude-subtask-ledger-store`
- PR title: `runtime: subtask ledger persistence (claude-subtask-ledger-store)`
