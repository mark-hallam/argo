# claude-action-dry-run-mode — `dry_run: true` manifest mode that builds the action artifact without executing

**Owner**: claude. **Depends on**: `mission-wire-mcp-action-executor` (#933), `mission-wire-api-action-executor` merged — both are reference dispatch paths this brief intercepts.

## Why

Operators authoring action manifests need a fast feedback loop: "given this manifest + this payload, what would the dispatch layer actually send?" Today they have to run a real action against a real server (or set up an echo fixture). That's slow and pollutes audit logs.

This brief adds a **dry-run mode** to action declarations: when `dry_run: true`, the dispatch layer performs all the substitution, validation, and artifact assembly steps but stops short of the side-effecting call (no HTTP, no MCP `tool/call`, no CU subprocess). The full would-have-been request body is returned as the action outcome so the operator can inspect it.

Useful for: manifest authoring, CI-time syntax checks, redteam dry-runs.

## Scope

1. **NEW** `crates/argo-cli/src/handlers/worker_actions_dry_run.rs` (~280 LOC):
   - `DryRunInterceptor` struct wrapping any inner `Executor`
   - `dispatch` impl: builds the artifact via the inner executor's preparation phase (substitution, schema validation), then short-circuits — returns `ActionOutcome::Completed(json!({ "dry_run": true, "would_have_sent": <artifact>, "executor_kind": "<kind>" }))`
   - Per-executor preparation hooks: extract `prepare_artifact(action, payload) -> Value` from each existing executor (no new ExecutorKind variants needed) and call them from the interceptor
2. **Manifest contract** — extend the existing `executor` table parsing in the action loader to recognize `dry_run = true` and wrap the resolved executor with `DryRunInterceptor`. Same TOML schema; just one new optional field. **No `argo-vertical` schema change** (this lives in the same loose-typed action declaration table that already accepts ad-hoc fields, like `binding`).
3. **CLI surface** — `argo action invoke <action_id> --payload <json> --dry-run` flag forces dry-run regardless of manifest setting (operator escape hatch). Wires through the existing `argo action invoke` path.
4. **Telemetry** — emit `action.dry_run` event with `{action_id, executor_kind, payload_hash}`

## Files you may touch

- `crates/argo-cli/src/handlers/worker_actions_dry_run.rs` (NEW, ≤ 320 LOC)
- `crates/argo-cli/src/handlers/worker_actions_dry_run_tests.rs` (NEW, ≤ 280 LOC sibling) — 8 unit tests:
  - dry-run on api executor returns `would_have_sent` shape
  - dry-run on mcp executor returns `would_have_sent` shape
  - dry-run on cu executor returns `would_have_sent` shape (with goal summary, not screenshots)
  - manifest `dry_run = true` triggers wrapping at resolve_executor
  - CLI `--dry-run` flag overrides manifest (manifest off, flag on → still dry-run)
  - real run when manifest off + flag absent
  - substitution still applies in dry-run (verify args resolved)
  - schema validation still applies (invalid payload errors out before dry-run completes)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 1 `pub mod` line
- `crates/argo-cli/src/handlers/worker_actions_executor.rs` — narrowly: `resolve_executor` wraps with `DryRunInterceptor` if `dry_run` flag present, ~15 LOC delta
- `crates/argo-cli/src/handlers/worker_actions_executor_api.rs` — narrowly: extract `prepare_artifact` helper (no behaviour change for real path)
- `crates/argo-cli/src/handlers/worker_actions_executor_mcp.rs` — narrowly: extract `prepare_artifact` helper (no behaviour change for real path)
- `crates/argo-cli/src/handlers/worker_actions_executor_stubs.rs` — narrowly: same extract for any other executor that has a buildable artifact
- `crates/argo-cli/src/commands.rs` — narrowly: add `--dry-run` flag to `argo action invoke`
- `crates/argo-cli/tests/action_dry_run_smoke.rs` (NEW, ≤ 220 LOC) — 1 integration test exercising both manifest-driven and flag-driven dry-runs
- `crates/argo-cli/tests/help_smoke.rs` — narrowly: 1 row pinning `argo action invoke --help` showing the new flag

## Files you must NOT touch

- `argo-vertical/src/manifest.rs` — the existing TOML accepts `dry_run` as a free field; no schema change
- `argo-types::ActionOutcome` — reuse `Completed(Value)`; do not add a new variant
- The `Executor` trait — `DryRunInterceptor` is just an `Executor` impl that wraps another
- `argo-mcp` / `argo-channels`

## Acceptance criteria

- [ ] `dry_run = true` in action declaration short-circuits dispatch and returns `would_have_sent` artifact
- [ ] `argo action invoke --dry-run` CLI flag forces dry-run regardless of manifest
- [ ] Substitution + schema validation still run before short-circuit
- [ ] Schema validation failure in dry-run still surfaces as `Err`, not as a fake-success
- [ ] All three executor kinds (api, mcp, cu) supported via `prepare_artifact` extracts
- [ ] Telemetry event `action.dry_run` fires
- [ ] 8 unit tests + 1 integration smoke + 1 help-smoke row
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli` clean
- [ ] Every file < 500 LOC

## Branch + PR

- Branch: `claude-action-dry-run-mode`
- PR title: `actions: dry-run mode (manifest + CLI flag) returns would-have-sent artifact (claude-action-dry-run-mode)`

## Notes for the implementer

- The "extract `prepare_artifact`" sub-changes in api / mcp / stubs are pure refactors — the real-path output should be byte-identical pre/post. Keep those changes in tiny commits and run the existing executor tests to verify.
- For CU, "would_have_sent" is the goal summary + the system prompt that would be sent to the model — not screenshots. Document this in the dry-run output explicitly so users know dry-run-CU is informational, not a full preview.
- The interceptor wrapping happens in `resolve_executor` (where ExecutorKind dispatch lives); this lets the rest of the dispatch path stay oblivious to dry-run mode.
- If a future executor (e.g. `ComputerUseMacOS` per `mission-cu-native-macos-executor-dispatch`) ships without a `prepare_artifact` hook, dry-run for that kind returns `Err(ArgoError::Provider("dry-run not supported for executor kind <kind>"))` — fail loud, not silently.
- Per Tick 45 EXHAUSTIVE-MATCH: this brief does NOT add a new ExecutorKind variant, so workspace-wide match-arm changes are out of scope.
