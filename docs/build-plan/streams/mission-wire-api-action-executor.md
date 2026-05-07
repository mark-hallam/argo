# Mission load-bearing wire #3 — action-level ApiExecutor (replace stub)

**Owner**: codex. **Depends on**: mission-wire-mcp-action-executor merged (mirrors its shape).

> SOURCE: deep-research findings tick 198, Section 1, item #3.
> Mechanical mirror of mission-wire-mcp-action-executor for direct
> REST calls declared in manifests.

## Scope

[`worker_actions_executor_stubs.rs`](crates/argo-cli/src/handlers/worker_actions_executor_stubs.rs)
ships an `ApiExecutor` stub deliberately, per A-executor-trait spec.
Some manifests prefer a direct REST call over an MCP connector
(e.g., a one-off webhook with no full connector justifying its
own crate). This brief implements that path: read the action's
manifest declaration, build a `reqwest` request from
`endpoint`/`method`/`headers`/`body_template`, fire it, translate
the response into `ActionOutcome`.

This is a smaller, mechanical mirror of the McpExecutor brief — it
exists to round out the executor families without forcing every
trivial REST call into a full MCP connector crate.

## Files you may touch

- `crates/argo-cli/src/handlers/worker_actions_executor_stubs.rs` —
  swap the `ApiExecutor` stub for the real impl
- `crates/argo-cli/src/handlers/worker_actions_executor_api.rs`
  (NEW, sibling) — request building + response translation helpers.
  Keep file under 200 LOC
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: ONE
  `pub(crate) mod worker_actions_executor_api;` line
- `crates/argo-cli/tests/api_action_executor_smoke.rs` (NEW) —
  integration smoke against a mock httpbin-style server (use
  `wiremock` if already a workspace dep; otherwise `tokio::net`
  loopback)
- `crates/argo-cli/Cargo.toml` — narrowly: add `wiremock = { workspace = true }`
  to dev-dependencies if not already present (verify first; do
  NOT add a new workspace member without flagging)

## Files you must NOT touch

- `crates/argo-cli/src/handlers/worker_actions_executor_mcp.rs`
  (sibling to this; owned by mission-wire-mcp-action-executor)
- Any connector crate
- The agent-level dispatch path

## Acceptance criteria

- [ ] `ApiExecutor::dispatch` resolves `endpoint`/`method`/`headers`/
      `body_template` from `ActionDeclaration`
- [ ] reqwest call uses workspace-pinned client; bearer/basic auth
      from manifest is honored
- [ ] 2xx response → `ActionOutcome::Completed { result_json }`
      (body parsed as JSON if `Content-Type: application/json`,
      else string-wrapped)
- [ ] 4xx/5xx → `ActionOutcome::Failed { error }`
- [ ] Transport error → `ArgoError::Provider`
- [ ] `[actions.X.budget].max_secs` enforced via tokio::time::timeout
      (default 30s if unset)
- [ ] 5 unit tests (200 / 401 / 500 / timeout / body-template) + 1
      integration smoke
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `codex-mission-wire-api-action-executor`
- PR title: `actions: real action-level ApiExecutor (mission-wire-api-action-executor)`

## Notes for the implementer

- Mirror the structural choices made in mission-wire-mcp-action-executor.
  If that brief landed `worker_actions_executor_mcp.rs` with a
  particular helper layout, follow the same pattern.
- Body template substitution: identical helper to the MCP brief
  (refactor the substitution helper into a shared sibling if both
  briefs converge on the same code; keep file count low).
- Do NOT introduce a new HTTP client. reqwest is already a workspace
  dep used by every connector crate.
