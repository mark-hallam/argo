# Stream A — Executor trait + manifest enum

**Owner**: claude. **Critical path**: this is the most important
PR of the next 14 days. Streams B, C, G all depend on it.

## Scope

Extend the `Executor` trait so the OS can dispatch to one of
four executor families per action: `human` (current behavior),
`mcp`, `computer_use`, `api`. Make the manifest's `executor`
field a tagged enum and route action execution accordingly.
Back-compat: existing `executor = "human"` strings continue to
parse.

This is a CONTRACT PR. Other streams will implement against the
trait + tests this PR defines. Get the shape right.

## Files you may touch
- `crates/argo-cli/src/handlers/worker_actions_executor.rs`
- `crates/argo-cli/src/handlers/worker_actions_executor_tests.rs`
- `crates/argo-vertical/src/manifest_actions.rs` (executor
  field)
- `crates/argo-cli/src/handlers/worker_actions_dispatch.rs`
  (dispatch site)
- New file (if helpful): `crates/argo-core/src/executor.rs`
- `Cargo.toml` (workspace member if a new crate is added)

## Files you must NOT touch
- Any vertical's `vertical.toml` (manifest content) — only the
  schema changes here, not the manifests
- `crates/argo-mcp/**` (Stream C wires MCP into dispatch later)
- Anything under `crates/argo-channels/**` (Stream D)

## Acceptance criteria
- [ ] `ExecutorKind` enum: `Human | Mcp | ComputerUse | Api`
      (new types may add later)
- [ ] `Executor` trait: `kind() -> ExecutorKind`,
      `preflight(action) -> Result<Preflight>`,
      `dispatch(action, ctx) -> Result<ActionOutcome>`
- [ ] `ActionOutcome` enum: `HumanHandoff` (renders artifact
      bundle, ledger sits at `executing`),
      `Completed { result_json }` (auto-completes the action)
- [ ] `HumanExecutor` refactored to return `HumanHandoff`
- [ ] Manifest `executor` field accepts BOTH the legacy string
      `"human"` AND a tagged-enum form:
      `executor = { kind = "computer_use", profile = "...", budget_usd = 0.50 }`
      (legacy strings parse as `{ kind = "human" }`)
- [ ] Dispatch path: `worker_actions_dispatch::execute()` reads
      the action's executor declaration, looks up the right
      impl, dispatches; auto-completes on `Completed` outcome
- [ ] Three impls registered as no-ops that return
      `Err("not yet implemented")`: `McpExecutor`,
      `ComputerUseExecutor`, `ApiExecutor`. Stream B / C / G
      replace these.
- [ ] Unit tests in `worker_actions_executor_tests.rs`:
      back-compat parses, tagged-enum parses, HumanExecutor
      still returns `HumanHandoff`, the three no-op impls fail
      cleanly with a useful error
- [ ] `cargo build -p argo-cli && cargo test -p argo-cli &&
      cargo clippy --workspace --all-targets -- -D warnings`
      all pass
- [ ] Every modified file < 500 LOC

## Branch + PR
- Branch: `claude-A-executor-trait`
- PR title: `executor: trait + manifest enum for human/mcp/cu/api dispatch (A-executor-trait)`

## Notes for the implementer

- The existing `HumanExecutor` lives at
  `worker_actions_executor.rs:HumanExecutor`. Keep it working
  exactly the same — only the trait shape changes.
- Tagged-enum parsing in serde is straightforward; use
  `#[serde(untagged)]` or a custom Deserialize for the legacy
  string form.
- The "no-op impls fail cleanly" path is so Stream C / B / G
  don't have to wire dispatch separately — they only have to
  swap the no-op for a real impl.
