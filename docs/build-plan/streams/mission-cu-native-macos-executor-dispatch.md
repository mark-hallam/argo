# mission-cu-native-macos-executor-dispatch — wire ExecutorKind dispatch arm for argo-executor-computer-use-macos (Phase B follow-up to #941)

**Owner**: claude. **Depends on**: nothing — `argo-executor-computer-use-macos` crate already exists from #941 (claude-computer-use-native-macos-poc). **Phase B claim-5 closure brief.**

## Why

#941 shipped a complete Rust crate for native macOS computer-use execution (subprocess approach via `screencapture` + `osascript`, 10 src files, ~1725 LOC). The PR's merge note explicitly deferred dispatch:

> "Boundary: ExecutorKind dispatch arm deferred to follow-up."

So today: a vertical declaring `executor.kind = "computer-use-macos"` cannot reach the new crate. The runtime returns `unknown executor kind`. This brief closes that gap.

## Scope

1. **Add `ComputerUseMacOS` variant to `ExecutorKind`** at `crates/argo-cli/src/handlers/worker_actions_executor.rs:41`. Doc-comment mirroring the existing `ComputerUse` variant comment style (`Wired by #941 (claude-computer-use-native-macos-poc).`).

2. **Parse arm** at line ~73 — accept `"computer-use-macos"` (and an alias `"computer-use-native-macos"` for clarity in vertical TOML).

3. **Resolve_executor arm** at line ~169-185 — invoke a thin adapter that converts the action declaration into a call into `argo-executor-computer-use-macos` and returns an `Executor` trait object.

4. **NEW thin adapter file**: `crates/argo-cli/src/handlers/worker_actions_executor_cu_macos.rs` (~100 LOC) — `ComputerUseMacOSExecutor` struct implementing `Executor` trait + `dispatch` shim that calls into the native crate's API surface. Pattern matches `worker_actions_executor_mcp.rs` (#933) and `worker_actions_executor_api.rs` (#952).

5. **Update 11 EXHAUSTIVE-MATCH sites** for the new enum variant (per Tick 45 amendment, this is permitted across may-touch boundaries). Most sites will need only an `_` arm or a routing-tag string update; a `grep -rn "match.*ExecutorKind" crates/` enumerates them.

6. **Cargo.toml** — add `argo-executor-computer-use-macos` to `argo-cli` workspace deps (today, the crate exists but no consumer imports it).

7. **Platform gating** — the underlying crate uses macOS-only subprocess primitives. Gate the dispatch arm behind `#[cfg(target_os = "macos")]`. On non-macOS, `ExecutorKind::ComputerUseMacOS` returns `Err(ArgoError::Provider("computer-use-macos requires macOS"))`. This matches how the upstream crate compiles cleanly on Linux CI today (#941's design).

## Files you may touch

- `crates/argo-cli/src/handlers/worker_actions_executor.rs` — narrowly: enum variant + parse arm + resolve_executor arm (~10 LOC delta)
- `crates/argo-cli/src/handlers/worker_actions_executor_cu_macos.rs` (NEW, ≤ 150 LOC)
- `crates/argo-cli/src/handlers/worker_actions_executor_cu_macos_tests.rs` (NEW, ≤ 200 LOC sibling) — 5 unit tests:
  - parse `"computer-use-macos"` → ExecutorKind::ComputerUseMacOS
  - parse alias `"computer-use-native-macos"` → ExecutorKind::ComputerUseMacOS
  - resolve_executor returns the right struct on macOS
  - On non-macOS (cfg-gated), dispatch returns a clean Provider error
  - End-to-end: action declaration with `executor.kind = "computer-use-macos"` reaches the adapter (mock the crate API)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 1 `pub mod` line
- `crates/argo-cli/Cargo.toml` — narrowly: add `argo-executor-computer-use-macos` workspace dep
- `crates/argo-cli/tests/cu_macos_dispatch_smoke.rs` (NEW, ≤ 100 LOC) — 1 integration test on macOS only (cfg-gated)
- The 11 ExecutorKind match sites across the workspace — narrowly: add the new variant arm (per Tick 45 EXHAUSTIVE-MATCH)
- `verticals/<demo>/vertical.toml` (one example, optional) — illustrate `executor.kind = "computer-use-macos"` for documentation; only if the chosen vertical isn't already busy with other features

## Files you must NOT touch

- `argo-executor-computer-use-macos` crate internals — its API surface is stable from #941. This brief consumes it; doesn't extend it.
- `argo-executor-computer-use` (the existing E2B/Firecracker variant) — separate executor; this brief adds a sibling, doesn't replace.
- `argo-vertical/src/manifest.rs` — `executor.kind` is already a string; no schema change needed (matches #952 pattern).
- Any vertical that currently runs in production — no behaviour change for existing `executor.kind` values.

## Acceptance criteria

- [ ] `ExecutorKind::ComputerUseMacOS` variant exists with parse from `"computer-use-macos"` and `"computer-use-native-macos"`
- [ ] On macOS, a manifest declaring `executor.kind = "computer-use-macos"` resolves to a working executor that reaches `argo-executor-computer-use-macos` crate
- [ ] On non-macOS, the same manifest declaration returns `Err(ArgoError::Provider("computer-use-macos requires macOS"))` cleanly
- [ ] All 11 existing match sites compile cleanly with the new variant
- [ ] 5 unit tests + 1 integration smoke (macOS-gated)
- [ ] `cargo build --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli` clean on Linux CI (smoke skips)
- [ ] On macOS dev: integration smoke passes
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-mission-cu-native-macos-executor-dispatch`
- PR title: `actions: ExecutorKind dispatch for native macOS computer-use (mission-cu-native-macos-executor-dispatch)`

## Notes for the implementer

- The adapter file should be ~95% similar in shape to `worker_actions_executor_mcp.rs` and `worker_actions_executor_api.rs` — read both end-to-end before starting.
- Per Tick 45 EXHAUSTIVE-MATCH amendment: enum variant additions are allowed to touch match sites across the workspace even when those files aren't in the explicit may-touch list. List which files you touched in the PR body so reviewers can verify nothing slipped.
- The native crate from #941 may have an API surface that doesn't directly fit the `Executor` trait shape — that's fine; the adapter layer handles the impedance mismatch. Keep the trait contract clean.
- Mission claim 5 ("real-world execution = any task a human can do on a PC") goes from ~75% (E2B-CU + macOS-PoC-not-wired) to ~95% (E2B-CU + macOS-CU dispatch live). The remaining 5% is iOS / Linux-desktop CU which is out of scope for this brief.
- If during implementation you discover the native crate's API needs a small extension to fit the dispatch shape — STOP, mark BLOCKED, and call it out in BUILD_STATUS.md. The crate is on may-touch-NOT list; better to queue a sub-brief than violate boundary.
