# hotfix-argo-tool-progress-event-import — Restore `ToolProgressEvent` import in `tools/terminal.rs`

**Owner**: claude. **Depends on**: none. **Priority**: HOTFIX — unblocks workspace clippy gate.
**Authored against**: origin/main `905902e4` on `2026-05-10`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh hotfix-argo-tool-progress-event-import`.

> **Tier P (production hardening) — workspace clippy gate hotfix.**
> Worker reports on PR #1255 confirmed that `cargo clippy --workspace --all-targets -- -D warnings` (and `bash scripts/workspace-clippy-fast-fail.sh`) fail on current main with:
>
> > 6 errors in `crates/argo-tool/src/tools/terminal_tests.rs` (`ToolProgressEvent` not in scope, etc.)
> > 1 error in `crates/argo-tool/tests/process_registry_integration.rs` (`Terminal.dispatch(...)` form)
>
> Conductor verified the root cause: `crates/argo-tool/src/tools/terminal.rs` line ~59 imports `error_text, success_text, Concurrency, Tool, ToolContext` from `crate::tool` but **omits `ToolProgressEvent`**. The sibling test file `terminal_tests.rs` uses `use super::*;` which only re-exports symbols already in scope in `terminal.rs` — so the test file's 5+ references to `ToolProgressEvent` are unresolved. Likely introduced during the rebase/split for #1239 (tool-terminal-background-processes) when `terminal_streaming.rs` was split out.

## Scope

Pure-additive surgical fix. Add `ToolProgressEvent` (and any related symbols the tests reference) to the import list in `crates/argo-tool/src/tools/terminal.rs` so `use super::*;` in `terminal_tests.rs` brings them into scope. If the integration test `process_registry_integration.rs` has a separate issue (`Terminal.dispatch(...)` form), fix that import or method-name reference too.

This is NOT in scope: refactoring streaming, changing behavior, adding new features, modifying test logic, fixing other crates.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported

## Files you may touch

- `crates/argo-tool/src/tools/terminal.rs` — narrowly scoped: extend the existing `use crate::tool::{...};` line on or near line 59 to include `ToolProgressEvent` (and any other re-export the tests need). NO other edits.
- `crates/argo-tool/tests/process_registry_integration.rs` — narrowly scoped: fix the cited `Terminal.dispatch(...)` form (likely a method-name or trait-invocation issue post-rebase). Verify against current `Terminal` impl. NO other edits.
- `crates/argo-tool/src/tools/terminal_tests.rs` — narrowly scoped: ONLY if `super::*` import isn't sufficient (e.g. if `ToolProgressEvent` isn't re-exportable through `terminal.rs` for some reason), add a direct `use crate::tool::ToolProgressEvent;` at the top.

## Files you must NOT touch

- `crates/argo-tool/src/tool.rs` — `ToolProgressEvent` definition is correct and stable; reuse, don't modify
- `crates/argo-tool/src/tools/terminal_streaming.rs` — already imports correctly
- Any other tool, any non-argo-tool crate
- `crates/argo-store/migrations/**` — no schema change

## Acceptance criteria

- [ ] `cargo clippy -p argo-tool --all-targets -- -D warnings` clean (was failing before this fix)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (was failing before this fix)
- [ ] `bash scripts/workspace-clippy-fast-fail.sh` exits 0
- [ ] `cargo test -p argo-tool` passes (skip pre-existing flakes per KNOWN_FLAKES.md)
- [ ] `cargo test -p argo-tool --test process_registry_integration` passes
- [ ] Diff is ≤ 10 LOC across at most 3 files
- [ ] No new public API
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-hotfix-argo-tool-progress-event-import`
- PR title: `argo-tool: restore ToolProgressEvent import in terminal.rs (hotfix-argo-tool-progress-event-import)`

## Notes for the implementer

- This is a regression hotfix. Run `cargo clippy -p argo-tool --all-targets -- -D warnings` first to reproduce the failure, then apply the fix, then re-run to confirm clean.
- The integration test's `Terminal.dispatch(...)` form may be unrelated to the import issue — investigate the actual error message (it may be a method-rename that needs `Terminal::dispatch(&self, ...)` or `Tool::dispatch(...)` instead). Fix the simplest invocation form that compiles.
- Prefer extending the import line in `terminal.rs` over modifying the test file — keeps the test idiomatic (`use super::*;` is the convention).
- Do NOT add any features. Do NOT touch any tool other than terminal. Do NOT modify the streaming logic. This is a 5-line typo fix.
- If you find the issue is more complex than expected (e.g. there's a real API mismatch from a deeper refactor), STOP and BLOCK with the specific finding rather than expanding scope.
