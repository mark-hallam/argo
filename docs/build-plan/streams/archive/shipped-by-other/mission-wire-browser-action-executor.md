# mission-wire-browser-action-executor — BrowserExecutor adapter into stateful browser MCP

**Owner**: claude. **Depends on**: nothing (extends existing executor trait + browser MCP). **Tier M — Mission MVP critical path.**

**Authored against**: origin/main 7785708b on 2026-05-10.

## Why

Mission claim: workers can autonomously post to Marketplace / eBay / internal SaaS without
a human at the keyboard. Today only `McpExecutor`, `ApiExecutor`, and `HumanExecutor`
implement the `Executor` trait — `executor.kind = "browser"` has no in-process adapter.
Without this primitive, every browser-driven action falls back to HumanExecutor and the
mission MVP is not autonomous. This brief adds the missing adapter on top of the existing
16-tool stateful browser MCP session manager.

## Approach (concrete)

1. **NEW** `crates/argo-cli/src/handlers/worker_actions_executor_browser.rs`
   (~320 LOC, mirrors `worker_actions_executor_mcp.rs` shape):
   - `BrowserPool` struct holding `Arc<tokio::Mutex<HashMap<SessionKey, BrowserSession>>>`
     where `SessionKey = (tenant_id, worker_id)` — sessions are sticky per worker so
     login/cookie state persists across actions.
   - `default_pool()` static via `OnceLock`.
   - `dispatch_action(action: &ActionDeclaration, payload: &Value, pool: &BrowserPool, timeout: Duration) -> ArgoResult<ActionOutcome>`
     async: opens or reuses a session against `binding.target_url`, runs either
     a CU loop (`binding.mode = "cu"`) or a scripted CDP sequence
     (`binding.mode = "script"` with `binding.steps`), captures screenshots
     into the action's evidence bundle directory, returns `ActionOutcome::Completed`.
   - `dispatch_blocking(...)` sync wrapper for the `Executor::dispatch` trait.
2. **Modify** `worker_actions_executor_stubs.rs`:
   - Replace `BrowserExecutor` stub (or add if missing) with real impl.
   - `BrowserExecutor::with_manifest(declaration)` constructor mirroring `McpExecutor::with_manifest`.
3. **Modify** `worker_actions_executor.rs`:
   - `resolve_executor` arm for `ExecutorKind::Browser` constructs `BrowserExecutor::with_manifest(...)`.
4. **Modify** `crates/argo-vertical/src/manifest_actions.rs`:
   - Ensure `ExecutorKind::Browser` exists in the enum (extend if needed) plus serde
     round-trip test. **EXHAUSTIVE-MATCH amendment** applies: add the new arm to any
     existing match across the workspace, but ONLY the new arm.
5. **Reuse** `substitute_args` from `worker_actions_executor_mcp` for `binding.target_url`
   + `binding.instruction_template` template substitution (same `{{ payload.X }}` /
   `{{ action_id }}` / `{{ secret.X }}` syntax as the MCP/API executors).
6. **Capability**: declare `Spawn::Browser` (or reuse the existing browser capability if
   already exported by `argo-capability`); the brief does NOT widen capability surface
   beyond what the browser MCP already required.
7. **Rustdoc**: every `pub` item carries doc + at least one example, per the hard rule.

## Manifest contract (uses existing `executor.binding` field — no schema change)

For browser kind, `binding` carries:
- `binding.target_url` — initial URL the session navigates to
- `binding.mode` — `"cu"` (CU loop) or `"script"` (CDP sequence)
- `binding.instruction_template` — natural-language goal for CU mode
- `binding.steps` — array of `{action, selector, value?}` for script mode
- `binding.session_scope` — `"per_worker"` (default) or `"per_action"` (ephemeral)

## Files you may touch

- `crates/argo-cli/src/handlers/worker_actions_executor_browser.rs` (NEW, ≤ 380 LOC)
- `crates/argo-cli/src/handlers/worker_actions_executor_browser_tests.rs` (NEW, ≤ 250 LOC) — 4+ unit tests:
  - success → Completed with screenshot paths in evidence bundle
  - missing `target_url` → provider error
  - timeout → provider error
  - args + secret substitution
- `crates/argo-cli/src/handlers/worker_actions_executor_stubs.rs` — narrowly: replace
  `BrowserExecutor` stub with real impl + `with_manifest` constructor.
- `crates/argo-cli/src/handlers/worker_actions_executor.rs` — narrowly: `resolve_executor`
  arm for `ExecutorKind::Browser`.
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 1 pub mod line.
- `crates/argo-vertical/src/manifest_actions.rs` — narrowly: ensure `Browser` enum variant
  + serde round-trip.
- `crates/argo-cli/tests/browser_action_executor_smoke.rs` (NEW, ≤ 200 LOC) —
  1 integration test with a mock browser MCP server (Python stdlib HTTP server that
  records the requested actions; mirrors the `api_echo_server.py` / `mcp_echo_server.py` pattern).
- `crates/argo-cli/tests/fixtures/browser_mock_mcp.py` (NEW) — stdlib HTTP server.

## Files you must NOT touch

- `argo-types::ActionOutcome` — do NOT add a `Failed` variant; use `Err(ArgoError::Provider(...))`.
- The existing 16-tool browser MCP session manager itself — this brief is an *adapter*,
  not a refactor of the underlying primitive.
- Any auto-approval / state-machine code — orthogonal.

## Acceptance criteria

- [ ] `BrowserExecutor` returns `Ok(ActionOutcome::Completed(...))` on success with
      screenshot paths recorded to the evidence bundle.
- [ ] Missing `binding.target_url` → `Err(ArgoError::Provider(...))` with clear message.
- [ ] Timeout (30s default, env-overridable via `ARGO_BROWSER_ACTION_TIMEOUT_MS`) →
      `Err(ArgoError::Provider(...))`.
- [ ] Per-worker session reuse: 2 sequential actions for the same worker reuse the
      same browser session (asserted via mock MCP request count).
- [ ] 4+ unit tests + 1 integration smoke test.
- [ ] Capability `Spawn::Browser` declared on the executor; rustdoc on every pub item.
- [ ] `cargo clippy -p argo-cli -p argo-vertical --all-targets -- -D warnings` clean.
- [ ] Every modified file < 500 LOC.

## Branch + PR

- Branch: `claude-mission-wire-browser-action-executor`
- PR title: `actions: real action-level BrowserExecutor (mission-wire-browser-action-executor)`
