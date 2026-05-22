# claude-tui-input-split — split argo-tui/src/input.rs (at exactly 500 LOC) to free key-binding headroom

**Owner**: claude. **Depends on**: nothing. **Phase A architectural prep — unblocks 2 BLOCKED TUI briefs.**

**Authored against**: origin/main 6b22a1ac on 2026-05-09 (conductor-authored after multiple TUI briefs hit the input.rs ceiling).

## Why

`crates/argo-tui/src/input.rs` is at exactly **500 LOC** on current main — zero headroom. Two TUI briefs are already BLOCKED on this:
- `claude-tui-search-everywhere` (BLOCKED 2026-05-09; needs Ctrl-K binding)
- `claude-tui-spend-monitor` (BLOCKED 2026-05-09; needs `$` binding repurpose)

Plus future TUI key-binding additions all hit the same ceiling. This brief unblocks both BLOCKED rows AND prevents future ones.

Pure refactor; no behaviour change.

## Approach (concrete)

`input.rs` is the central key-dispatch hub — likely contains a giant `match key { ... }` block with arms for each binding (overlay toggles, navigation, etc.). Split by binding category:

**Option A (preferred)**: extract per-overlay key handlers to siblings.
- `input.rs` keeps the top-level `dispatch_key()` + global-mode arms
- `input_overlay_dispatch.rs` (NEW) — per-overlay key handlers (budget, dashboard, search, etc.)
- Keep navigation/mode-switch arms in `input.rs`

**Option B**: extract the modifier-key combos (Ctrl-/Alt-/Shift-) which tend to cluster.

Target after split: `input.rs` ≤ 350 LOC (≥ 150 LOC headroom for future bindings).

## Files you may touch

- `crates/argo-tui/src/input.rs` — narrowly: extract sibling, re-export
- `crates/argo-tui/src/input_<topic>.rs` (NEW, ≤ 250 LOC)
- `crates/argo-tui/src/lib.rs` — narrowly: 1 `pub mod` line if needed

## Files you must NOT touch

- `input_helpers.rs` (already a sibling for shared utilities)
- `input_h_stack.rs` (already a sibling for the H-stack overlay)
- Any overlay implementation file (`overlays_*.rs`) — refactor only the input dispatch
- Any test file other than to verify no behaviour change

## Acceptance criteria

- [ ] `input.rs` ≤ 350 LOC after the split (≥ 150 LOC headroom)
- [ ] Extracted sibling ≤ 250 LOC
- [ ] `cargo clippy -p argo-tui --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-tui` clean (no behaviour change)
- [ ] No public API changes; the `dispatch_key()` entry point unchanged

## Branch + PR

- Branch: `claude-tui-input-split`
- PR title: `tui: split input.rs at 500-LOC ceiling (claude-tui-input-split)`

## Notes for the implementer

- Pattern-match on prior splits — `claude-routes-post-split` (#932), `claude-doctor-channels-split` (#1088), `claude-agent-runner-turn-split` (#1097). All preserve public API via `pub use` re-exports.
- Existing `input_helpers.rs` and `input_h_stack.rs` show the project's accepted sibling-naming convention for input.
- After this lands, lift the BLOCKED status on `claude-tui-search-everywhere` and `claude-tui-spend-monitor` — they become claimable again.

## Unblocks

- **claude-tui-search-everywhere** — currently BLOCKED on input.rs ceiling
- **claude-tui-spend-monitor** — currently BLOCKED on input.rs ceiling
- Any future TUI key binding addition
