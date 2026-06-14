# win-cu-windows-types-cross-platform-for-mapper-coverage

**Authored against:** origin/main `de775470c` on 2026-06-14
**Validate before claim:** `bash scripts/check-brief-freshness.sh win-cu-windows-types-cross-platform-for-mapper-coverage`
**Priority:** P3 (test-coverage improvement; NOT a build/ship blocker — the mapper is
already covered on real Windows hosts by the fleet).

## Background (macOS-side fix-forward already landed)

`map_windows_outcome` (`crates/argo-cli/src/handlers/worker_actions_executor_cu_windows.rs`)
maps `argo_executor_computer_use_windows::WindowsCuOutcome` → `ActionOutcome`. Because
`argo-executor-computer-use-windows` is a **windows-only dependency**
(`[target.'cfg(target_os = "windows")'.dependencies]` in `argo-cli/Cargo.toml`), both the
function and its unit test `maps_completed_and_handoff_outcomes` can only compile on
Windows. The test was originally written **non-cfg** ("covered on the macOS/Linux swarm
too"), which broke `cargo test -p argo-cli` on macOS. The macOS conductor fix-forward
(2026-06-14) gated both the fn (#2937) and the test (`#[cfg(target_os = "windows")]`) so
the macOS gate compiles again — but that **lost the swarm-side coverage** of the pure
outcome-mapper logic, which is exactly what the test author wanted.

## The root fix (the version we don't have to revisit)

Make the windows-CU crate's **pure data types** (`WindowsCuOutcome`, config/step enums —
anything with no OS calls) compile on **every** platform, gating ONLY the real
OS-interaction code (screen capture, keyboard/mouse, the Anthropic live loop) behind
`#[cfg(target_os = "windows")]` *inside* the crate. Then:
- make `argo-executor-computer-use-windows` a normal cross-platform dependency of
  `argo-cli` (move it out of the `[target.'cfg(windows)']` table), and
- un-gate `map_windows_outcome` + `maps_completed_and_handoff_outcomes` so the mapper is
  covered on the macOS/Linux swarm again (revert the two `#[cfg(target_os = "windows")]`
  gates the fix-forward added — see the doc comment on that test for the exact spots).

## Verifier (checkable, NO THEATER — must be RUN on macOS, not just Windows)
- `cargo build -p argo-executor-computer-use-windows` succeeds on macOS (types compile;
  OS-call modules cfg-out cleanly, no dead-code/unused warnings under `-D warnings`).
- `cargo test -p argo-cli --bin argo maps_completed_and_handoff_outcomes` RUNS and PASSES
  on macOS (the mapper test is no longer gated out).
- Windows build + the existing `dispatch_on_windows_*` tests still green (no regression).

## Out of scope
- The macOS-side fix-forward gates are already merged and correct as an interim — this
  brief supersedes them by making the cross-platform path real, then removing them.
