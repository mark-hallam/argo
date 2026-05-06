# V1.3 — Runtime planner trait + subtask decomposition

**Owner**: claude. **Depends on**: claude-workflow-dag (#857) merged.
From the dreams + runtime-swarm investigation
(`docs/skycot/DREAMS_AND_RUNTIME_SWARM.md` Section 4).

## Scope

Mission claim 2 says delegation is the unit — operators delegate
ROLES, not tasks. But coarse delegations like "promote this
product launch" need to decompose into sub-tasks (research →
draft → design → schedule → monitor). Today workers don't do
this; they get a Session and treat it as one task.

Add a `Planner` trait that takes a coarse goal + worker context
and emits a subtask DAG (composes with `argo-workflow` from
#857). Subtasks can be:
- Single actions (existing action vocabulary)
- Sub-sessions (delegated to other workers via Handoff)
- Approval gates (operator decides one branch vs another)

## Architecture

```rust
trait Planner {
    fn decompose(&self, goal: &str, ctx: &PlannerContext)
        -> impl Future<Output = ArgoResult<SubtaskTree>>;
}

struct LlmPlanner;     // calls Anthropic with a planning prompt
struct TemplatePlanner; // matches goal to a saved plan template
                       // (precursor to the dreams-plan-templates
                       // brief in v2.0)
```

`SubtaskTree` is a DAG (reuses `argo-workflow` types). A
worker runtime calls `planner.decompose(...)` then hands the
DAG to `argo-workflow::Executor` to run.

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- EXHAUSTIVE-MATCH amendment applies
- `crates/argo-runtime/` (NEW crate — finally — for runtime
  worker pieces that don't fit in argo-orchestrator)
- `crates/argo-runtime/Cargo.toml` (NEW)
- `crates/argo-runtime/src/lib.rs` (NEW)
- `crates/argo-runtime/src/planner.rs` (NEW — trait + 2 impls)
- `crates/argo-runtime/src/planner_tests.rs` (NEW)
- `Cargo.toml` workspace — narrowly: add new member
- `crates/argo-cli/src/handlers/plan_*.rs` (3 NEW —
  `plan generate`, `plan show`, `plan execute`)
- `crates/argo-cli/src/cli_plan_cmds.rs` (NEW)
- `crates/argo-cli/src/cli_subcommands.rs` — narrowly: register
  `Plan` subcommand

## Acceptance criteria
- [ ] Planner trait + LlmPlanner + TemplatePlanner ship
- [ ] Decomposed subtasks become a valid argo-workflow DAG
- [ ] `argo plan generate <goal>` outputs YAML; `--apply` runs it
- [ ] `argo plan show <run-id>` displays planner reasoning
- [ ] LlmPlanner uses Anthropic; TemplatePlanner is offline-only
- [ ] 12 unit tests (DAG validity, prompt assembly) + 2 integration
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `claude-runtime-planner-trait`
- PR title: `runtime: Planner trait + subtask decomposition (claude-runtime-planner-trait)`
