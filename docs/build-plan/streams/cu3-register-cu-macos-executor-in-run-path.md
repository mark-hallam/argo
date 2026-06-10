# cu3-register-cu-macos-executor-in-run-path

**Owner**: claude or codex.
**Depends on**: cu2 (the dispatch wire it plugs into).
**Authored against**: origin/main `5f81ae776` on `2026-06-11`.
**Validate before claim**: bash scripts/check-brief-freshness.sh cu3-register-cu-macos-executor-in-run-path

## Goal (DEC-050 G3)

Route a teammate's `computer_use` action to the real macOS executor
(`argo-executor-computer-use-macos`) from the run path, so a hired teammate can
drive Notes/Finder/Safari on screen (the literal "computer use" demo).

## Why now

The cu-macos executor is built + tested (screenshot/click/type via
screencapture+osascript, 58 tests) but reachable only from CLI replay — it is
NOT in the run-path executor resolver. Once cu2 dispatches actions, the
resolver needs a `computer_use`/`computer_use_native_macos` arm that constructs
`MacosCuExecutor` with the manifest's `target_app` (e.g. `com.apple.Notes`).

## Fix

1. Add the `ComputerUseNativeMacos` arm to `resolve_executor()` (mirror the
   existing ComputerUse/MCP arms); construct `MacosCuExecutor` with a real
   `TenantWorkerLookup` (SessionStore-backed, not the test noop).
2. Manifest schema: accept `executor = { kind = "computer_use_native_macos",
   target_app = "com.apple.Notes" }`.
3. Preflight: surface the 4-gate TCC result to the operator (deep-link to
   System Settings) when ungranted, instead of a silent skip.

## Acceptance / verifier

1. A `computer_use_native_macos` action on a granted Mac drives a real Notes
   action (screenshot→type→readback), recorded as a CU bundle (browser.jsonl +
   step PNGs). Ungranted → a clear HumanHandoff with the settings deep-link.
2. `cargo test -p argo-cli -p argo-executor-computer-use-macos` green.

## Size / owner / priority

~80-150 LOC. **🟠 P2 — DEC-050 G3 (desktop-drive).** Needs cu2 + a perms
re-grant (cu4). The flashiest demo; furthest dependency chain.
