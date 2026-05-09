# mvp-api-executor — harden + smoke-test action-level API executor

**Owner**: claude. **Depends on**: nothing. **Tier 1 MVP — closes claim 6 ("multi-channel approvals + executor matrix complete").**

**Authored against**: origin/main 7fdbbdb8 on 2026-05-10.

## Why

`crates/argo-cli/src/handlers/worker_actions_executor_api.rs` was originally specced as "implement from stub mirroring MCP executor". Audit on 2026-05-10 confirms the file already exists at ~10 KB with `ApiPool`, `ApiBinding`, dispatch, and timeout wiring (likely landed via `mission-wire-api-action-executor` #952). The remaining gap for v1.0:

1. **End-to-end smoke test** — drives a real `argo` binary against a local mock HTTP server, verifies a 2xx auto-completes the action and a 5xx records a Provider error.
2. **Re-verify** the implementation against the brief's spec: per-base-URL pool (✓ pool exists), timeout wrapper (✓), URL/header/body templating including secrets (verify), 2xx auto-complete (verify), non-2xx/timeout → Provider error (verify), 6 unit tests minimum (count current — `worker_actions_executor_api_tests.rs` is ~5 KB).

## Approach

1. **Audit pass**: read `worker_actions_executor_api.rs` + `_tests.rs` end-to-end. Document any gaps against the spec list above. If the impl already meets the bar, the brief is mostly a smoke test + verification.
2. **Add smoke test** at `crates/argo-cli/tests/api_action_executor_smoke.rs` (≤ 200 LOC):
   - Spin up a `tiny_http`-based local server in the test (already a workspace dep via test-utils, or use `wiremock` if not).
   - Manifest with one `executor.kind = "api"` action pointing at `http://127.0.0.1:<port>`.
   - Drive `argo worker actions execute …` end-to-end.
   - Assert: success → action completes; 500 → Provider error.
3. **Fill gaps** if audit finds any (e.g., missing secret-substitution test). Each gap must be tracked with a unit test in `_tests.rs`.

## Files you may touch

- `crates/argo-cli/src/handlers/worker_actions_executor_api.rs` — narrowly: gap fixes only
- `crates/argo-cli/src/handlers/worker_actions_executor_api_tests.rs` — narrowly: gap-coverage tests
- `crates/argo-cli/tests/api_action_executor_smoke.rs` (NEW, ≤ 200 LOC)
- `crates/argo-cli/Cargo.toml` — narrowly: add `tiny_http` or `wiremock` test-dep IF not present

## Files you must NOT touch

- Any other executor file (mcp, code, document, cu_macos, stubs)
- Any non-api source

## Acceptance criteria

- [ ] Audit doc in PR body lists each spec line ✓ or → fixed-by-this-PR
- [ ] At least 6 unit tests in `_tests.rs` (count current; if ≥ 6 already, fine)
- [ ] 1 integration smoke at `tests/api_action_executor_smoke.rs` passes
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-cli` clean (Boundary OK)

## Branch + PR

- Branch: `mvp-api-executor`
- PR title: `cli: harden + smoke-test API action executor (mvp-api-executor)`
