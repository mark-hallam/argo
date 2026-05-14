# claude-tui-input-globals-split — Extract global-key dispatch from input.rs (unblocks 4 TUI briefs)

**Owner**: claude. **Depends on**: nothing.
**Authored against**: origin/main `03573bda` on `2026-05-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-tui-input-globals-split`.

## Scope

Phase D BLOCKED triage (`docs/skycot/audit/2026-05-15-phase-D-blocked-triage.md` §3) found 4 TUI briefs scope-mismatched: their may-touch lists forbid `crates/argo-tui/src/input.rs`, but that file owns the global-key dispatch (e.g., `?` → help overlay, `$` → spend overlay). Each blocked brief needs to add a new global keybinding but can't because the brief authors didn't include input.rs in scope.

This brief extracts the global keybinding dispatch from input.rs into a new sibling `input_globals.rs`, exposing a `register_global_key()` API. Once landed, the 4 blocked briefs can be rewritten to extend `input_globals.rs` (which is in scope for them).

Currently `crates/argo-tui/src/input.rs` is 334 LOC — under the ceiling. The split is for **scope-clarity**, not LOC-ceiling enforcement; the goal is to give downstream briefs a narrow extension point.

**Unblocks 4 BLOCKED briefs**:
- `claude-tui-approval-tray`
- `claude-tui-spend-monitor`
- `claude-tui-search-everywhere`
- `claude-tui-keybindings-help`

LOC: ~200.

## Files you may touch

- `crates/argo-tui/src/input.rs` — narrowly: extract global-key match arms into a delegate call to `input_globals::dispatch_global_key()`. ≤80 modified lines.
- NEW `crates/argo-tui/src/input_globals.rs` — global-key dispatcher with a `pub fn register_global_key(key, action)` registry-builder. ≤150 LOC.
- NEW `crates/argo-tui/src/input_globals_tests.rs` (sibling) — 3 unit tests covering: register + lookup + duplicate-key error.
- `crates/argo-tui/src/lib.rs` — narrowly: `pub mod input_globals;`. ≤5 modified lines.

## Files you must NOT touch

- `crates/argo-tui/src/render.rs` — render-side concerns (separate from input).
- `crates/argo-tui/src/overlays.rs` — overlay rendering (not input).
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] `input_globals.rs` exists with `register_global_key()` + `dispatch_global_key()` pub functions.
- [ ] `input.rs` delegates global-key matches via the new dispatcher.
- [ ] Behavior preservation: `?` still opens help overlay, `$` still opens spend overlay (no regression).
- [ ] 3+ unit tests passing.
- [ ] `cargo test -p argo-tui` passes.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] No file > 500 LOC.
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `claude-tui-input-globals-split`
- PR title: `refactor(tui): extract global-key dispatch to input_globals.rs (claude-tui-input-globals-split)`

## Notes for the implementer

- Mission-claim alignment: production hardening (unblocks 4 downstream TUI briefs in BLOCKED state).
- High-leverage: this single brief unlocks 4 BLOCKED rows that have been waiting for a scope-fix.
- Drift-prevention: adds ONE new pub module (`input_globals`); no parallel surface — extends existing TUI input subsystem.
