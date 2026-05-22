# Stream B — ComputerUseExecutor + E2B sandbox

**Owner**: claude. **Depends on**: A-executor-trait merged.

## Scope

Implement the `ComputerUseExecutor` impl of the `Executor`
trait from Stream A. Drives a Firecracker microVM (via E2B
managed service) running a desktop image. Calls the Anthropic
computer-use API in a loop: send screenshots + previous
actions, receive next actions (click / type / scroll / key),
execute in the sandbox, capture screenshot, repeat until the
model emits a `done` signal or the budget is exhausted.

Auto-completes the action on success with the captured result
(URL, confirmation text, etc.). On failure, falls back to
emitting a `HumanHandoff` so the operator can finish manually.

## Files you may touch
- `crates/argo-executor-computer-use/**` (NEW crate)
- `Cargo.toml` (workspace member)
- `crates/argo-cli/src/handlers/worker_actions_executor.rs`
  (replace the no-op `ComputerUseExecutor` from Stream A with
  the real impl — single function call to the new crate)
- `manifests/executors/computer_use.toml` (NEW — declares
  available profiles: `marketplace_browser`, `desktop_full`,
  etc.)

## Files you must NOT touch
- `Executor` trait or its sibling impls (Stream A's contract)
- Anything in `argo-channels` or `argo-mcp`

## Acceptance criteria
- [ ] New crate `argo-executor-computer-use` builds clean
- [ ] `ComputerUseExecutor::dispatch` opens an E2B session,
      runs the CU loop with a token+wallclock budget, returns
      `Completed { result }` on success
- [ ] `result.json` includes: final screenshot path, action
      trajectory (one JSON line per CU action), captured URL
      (if the action profile extracts one), tokens used
- [ ] Per-action `evidence` array gains a row per CU action
      taken (`cu://action/<n>`)
- [ ] `browser.jsonl` audit log written under
      `.skycot/outputs/<job_id>/actions/<action_id>/browser.jsonl`
- [ ] Screenshot per CU step saved alongside `browser.jsonl`
- [ ] Token budget per action enforced; over-budget returns
      `HumanHandoff` with a clear message
- [ ] E2B VM pool keyed by `(tenant, persona, profile)` with
      10-min idle TTL; reuse if available
- [ ] Per-tenant quota check via existing `argo-types::cost_ceilings`
- [ ] One profile shipped in v1: `marketplace_browser`
      (Chromium, no desktop apps, FB Marketplace + eBay
      whitelist for navigation)
- [ ] Integration test (gated behind `E2B_API_KEY` env var):
      runs a synthetic "navigate to example.com, click the
      first link, capture URL" task end-to-end
- [ ] `cargo clippy -p argo-executor-computer-use -- -D warnings`
      clean
- [ ] Every file < 500 LOC

## Branch + PR
- Branch: `claude-B-computer-use-executor`
- PR title: `executor(cu): ComputerUseExecutor + E2B sandbox + marketplace_browser profile (B-computer-use-executor)`

## Notes for the implementer

- E2B SDK has a Rust crate; if it's stale, vendor the few HTTP
  calls we need.
- Anthropic computer-use API: see Anthropic docs. Use the
  computer-use-2024-10-22 tool variant. Screenshot resolution
  1280×800 baseline.
- The CU loop terminates when the model emits a tool call
  named `stop_reason: success | failure` OR the budget cap
  hits.
- When the action's `payload` contains `photo_paths`, mount
  those into the sandbox and tell the model where they are
  (pass paths in the system prompt of the CU loop).
