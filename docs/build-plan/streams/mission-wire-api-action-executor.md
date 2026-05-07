# Mission Phase A wire #3 — action-level ApiExecutor (rewrite v2, mirrors #933)

**Owner**: codex. **Depends on**: mission-wire-mcp-action-executor (#933) MERGED — this brief mirrors its shape.

> **REWRITE NOTE (tick 269 by conductor)**: original brief required
> `ActionOutcome::Failed` enum variant + manifest `endpoint` fields, but
> may-touch excluded the resolver/dispatch/schema files. One tab BLOCKED.
> This rewrite mirrors the proven #933 mcp-action-executor pattern and
> stays narrowly within executor + manifest scope.

## Scope

`ApiExecutor` (in `worker_actions_executor_stubs.rs`) currently returns
`Err(ArgoError::Provider("executor 'api' is not yet implemented"))`. Every
manifest declaring `executor.kind = "api"` bounces at dispatch. This brief
replaces the stub with a real impl that hits a configured HTTP endpoint
+ translates the response into an `ActionOutcome::Completed` (or a clear
provider error on transport/status failure).

Mirrors #933 (mcp-action-executor) exactly:
- Per-server connection pool (here: per-base-URL `reqwest::Client` reuse)
- `tokio::time::timeout` wrapping the HTTP call
- `substitute_args` template engine (already exists in `worker_actions_executor_mcp.rs` — reuse)
- Sync `dispatch_blocking` shim for the `Executor::dispatch` trait

## Approach (concrete)

1. **NEW** `crates/argo-cli/src/handlers/worker_actions_executor_api.rs`
   (~280 LOC, similar to `worker_actions_executor_mcp.rs` at 322):
   - `ApiPool` struct holding `Arc<tokio::Mutex<HashMap<String, reqwest::Client>>>`
   - `default_pool()` static via `OnceLock`
   - `dispatch_action(action: &ActionDeclaration, payload: &Value, pool: &ApiPool, timeout: Duration) -> ArgoResult<ActionOutcome>` async
   - `dispatch_blocking(...)` sync wrapper for the `Executor::dispatch` trait
2. **Modify** `worker_actions_executor_stubs.rs`:
   - `ApiExecutor` carries `Option<ActionDeclaration>` via `with_manifest` constructor (mirrors `McpExecutor::with_manifest`)
   - `dispatch` reads `executor.binding` field from the carried declaration (binding is the existing manifest field — DO NOT add new schema fields)
   - 30s default timeout via constant `DEFAULT_API_ACTION_TIMEOUT`
3. **Modify** `worker_actions_executor.rs`:
   - `resolve_executor` arm for `ExecutorKind::Api` constructs `ApiExecutor::with_manifest(...)` passing the action declaration through
4. **Reuse** `substitute_args` from `worker_actions_executor_mcp` —
   import via `use super::worker_actions_executor_mcp::substitute_args;`
   (or move to a shared `worker_actions_executor_args.rs` if cleaner)

## Manifest contract (no schema change)

`executor.binding` is the existing field. For api kind, it carries:
- `binding.url` — full HTTP URL (e.g. `https://api.example.com/{{action_id}}/run`)
- `binding.method` — HTTP method (default `POST`)
- `binding.headers` — optional headers map (e.g. `Authorization: Bearer {{secret.MY_TOKEN}}`)
- `binding.body_template` — optional body template (default uses payload as-is)

Template substitution uses the same `{{ payload.X }}` / `{{ action_id }}` /
`{{ secret.X }}` syntax as #933.

## Files you may touch

- `crates/argo-cli/src/handlers/worker_actions_executor_api.rs` (NEW, ≤ 320 LOC)
- `crates/argo-cli/src/handlers/worker_actions_executor_api_tests.rs` (NEW, ≤ 250 LOC) — 6 unit tests:
  - success → Completed
  - non-2xx HTTP status → provider error
  - timeout → provider error
  - missing url binding → provider error
  - args substitution
  - secret substitution
- `crates/argo-cli/src/handlers/worker_actions_executor_stubs.rs` —
  narrowly: replace ApiExecutor stub with real impl + `with_manifest` constructor
- `crates/argo-cli/src/handlers/worker_actions_executor.rs` —
  narrowly: `resolve_executor` arm for `ExecutorKind::Api`
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 1 pub mod line
- `crates/argo-cli/tests/api_action_executor_smoke.rs` (NEW) —
  1 integration test with a Python-stdlib HTTP server fixture
  (mirror of `mcp_action_executor_smoke.rs` + `tests/fixtures/mcp_echo_server.py`)
- `crates/argo-cli/tests/fixtures/api_echo_server.py` (NEW) —
  stdlib HTTPServer that echoes back the JSON body; same pattern as the
  mcp echo fixture

## Files you must NOT touch

- Any schema file (`crates/argo-vertical/src/manifest.rs` etc.) — use
  existing `executor.binding` field; no new fields
- `argo-types::ActionOutcome` — do NOT add a `Failed` variant; use
  `Err(ArgoError::Provider(...))` for transport failures (matches #933)
- argo-mcp — this brief touches HTTP only, not MCP

## Acceptance criteria

- [ ] `ApiExecutor` returns `Ok(ActionOutcome::Completed(...))` on 2xx HTTP
- [ ] Non-2xx HTTP status → `Err(ArgoError::Provider(...))` with status + body excerpt
- [ ] Timeout (30s default, env-overridable via `ARGO_API_ACTION_TIMEOUT_MS`) → `Err(ArgoError::Provider(...))`
- [ ] Args substitution works for `{{ payload.X }}` / `{{ action_id }}` / `{{ secret.X }}` in url + headers + body
- [ ] 6 unit tests + 1 integration smoke test
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `codex-mission-wire-api-action-executor`
- PR title: `actions: real action-level ApiExecutor (mission-wire-api-action-executor)`

## Notes for the implementer

- Look at `worker_actions_executor_mcp.rs` (322 LOC) end-to-end before starting.
  Your file should look ~95% similar in structure.
- `reqwest` is already a workspace dep — no new deps needed.
- For the integration smoke, the Python fixture should be ~30 LOC:
  `python3 -m http.server`-style + JSON echo on POST. Skip cleanly if
  `python3` not in PATH (mirrors `mcp_echo_server.py` pattern).
- 30s default timeout matches #933's `DEFAULT_ACTION_TIMEOUT`; keep them
  in sync (or refactor both to a shared constant later).
