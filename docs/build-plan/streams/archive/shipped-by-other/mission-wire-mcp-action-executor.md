# Mission load-bearing wire #2 — action-level McpExecutor (replace stub)

**Owner**: claude. **Depends on**: A-executor-trait (#784), C-mcp-wire (#788), all C-* connector briefs merged.

> SOURCE: deep-research findings tick 198, Section 1, item #2. The
> action-level dispatch path was left as a stub when C-mcp-wire
> shipped the agent-level path. Today this brick the marketplace-
> seller demo (and any vertical declaring `executor = { kind = "mcp",
> server = "X", tool = "Y" }`).

## Scope

[`worker_actions_executor_stubs.rs`](crates/argo-cli/src/handlers/worker_actions_executor_stubs.rs)
ships an `McpExecutor` whose `dispatch` returns

```
Err(ArgoError::Provider("executor `mcp` is not yet implemented; ..."))
```

at line 46. The agent-level dispatch shim landed by C-mcp-wire
already implements the routing+pool semantics for the LLM-driven
turn loop; this brief implements the action-level equivalent so
that manifests can declare `[actions.X] executor = { kind = "mcp",
server = "gmail", tool = "gmail.send" }` and have the gate-approved
action actually fire through the connector.

Resolve the action's manifest declaration → look up the `[mcp_servers]`
entry → acquire a connection from the per-server pool C-mcp-wire
already maintains → call the tool → translate the JSON-RPC response
into `ActionOutcome`. Audit the result through the existing
`ActionRecord` chain.

## Approach

1. Read the action's `ActionDeclaration` (already passed via
   `dispatch(action, cwd)`).
2. The declaration carries the executor block. Parse out
   `server` + `tool` + optional `args_template`.
3. Build the args JSON from the action's `payload_json` (template
   substitution; reuse whatever C-mcp-wire uses for its substitution).
4. Acquire a connection from `argo-mcp::connection_pool` keyed by
   `server`; call `tools/call` with the resolved tool name.
5. Translate response: success → `ActionOutcome::Completed { result_json }`;
   tool error → `ActionOutcome::Failed { error }`; transport error
   → propagate as `ArgoError::Provider`.
6. Wallclock budget: enforce `[actions.X.budget].max_secs` if set.

## Files you may touch

- `crates/argo-cli/src/handlers/worker_actions_executor_stubs.rs` —
  swap the stub `dispatch` for the real impl; the file is well under
  500 LOC and the `ApiExecutor` stub stays intact (separate brief)
- `crates/argo-cli/src/handlers/worker_actions_executor_mcp.rs`
  (NEW, sibling) — pool acquisition + tool call + outcome translation
  helpers. Keep file under 250 LOC
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: ONE
  `pub(crate) mod worker_actions_executor_mcp;` line
- `crates/argo-cli/tests/mcp_action_executor_smoke.rs` (NEW) —
  integration smoke: register a mock MCP server (use the same
  `MockMcpServer` test harness C-mcp-wire uses), declare an action
  pointing at it, dispatch end-to-end

## Files you must NOT touch

- `crates/argo-mcp/**` (the runtime is locked from C-mcp-wire; reuse
  its public API only)
- `crates/argo-mcp-connector-*/**` (each connector crate is its own
  brief)
- The agent-level dispatch path (`turn.rs` etc.) — separate concern
- The `ApiExecutor` stub (separate brief: `mission-wire-api-action-executor`)

## Acceptance criteria

- [ ] `McpExecutor::dispatch` resolves `server` + `tool` from
      `ActionDeclaration` and calls the connector
- [ ] Connection pool reused (no per-call connect)
- [ ] Args JSON built via template substitution from `payload_json`
- [ ] Success path returns `ActionOutcome::Completed` with the
      tool's structured result
- [ ] Tool error returns `ActionOutcome::Failed` (action goes
      `Failed` state, not `Errored`)
- [ ] Transport error propagates as `ArgoError::Provider`
- [ ] `[actions.X.budget].max_secs` enforced via tokio::time::timeout
- [ ] 6 unit tests (success / tool-error / timeout / missing-server /
      missing-tool / args-substitution) + 1 integration smoke
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-mission-wire-mcp-action-executor`
- PR title: `actions: real action-level McpExecutor (mission-wire-mcp-action-executor)`

## Notes for the implementer

- C-mcp-wire's agent-level `dispatch_one` is the template — read it
  first; reuse the connection-pool API verbatim.
- Argument template substitution: pick the simplest form that works
  (e.g., `{{payload.field_name}}`); do not invent a DSL.
- The `MockMcpServer` test harness from C-mcp-wire integration tests
  is the model for the smoke test fixture.
- After this lands, the marketplace-seller's `send_buyer_followup_email`
  action stops returning the stub error and actually fires through Gmail.
