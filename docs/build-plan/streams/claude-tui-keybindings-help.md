# claude-tui-keybindings-help — `?` shows context-aware help overlay

**Owner**: claude. **Depends on**: none.

## Scope

The TUI has accumulated many keybindings (j/k, Enter, q, A, $, Ctrl-K,
D, Tab, ?, etc.) across overlays. There is no in-app reference: the
user must check the README. This brief adds a `?` overlay that lists
the keybindings active _on the current overlay_, plus a global section.

## Approach (concrete)

1. **NEW** `crates/argo-tui/src/help_overlay.rs` (~260 LOC):
   - `pub struct HelpOverlay { sections: Vec<HelpSection>, parent_overlay: OverlayKind }`
   - `HelpSection { title: &'static str, bindings: Vec<KeyBinding> }`
   - `KeyBinding { keys: &'static str, description: &'static str }`
   - `pub fn for_overlay(kind: OverlayKind) -> Self` — static lookup table mapping each overlay to its bindings + always appends a "Global" section
   - `pub fn render(&self, frame: &mut Frame, area: Rect)` — centred modal, two-column key/description list, section headers in bold
   - `pub fn handle_key(&mut self, key: KeyEvent) -> HelpOutcome` — `?` toggles, q/Esc closes
2. **Hook** into `app.rs` global key dispatch: `?` opens help with the
   current overlay's bindings.
3. **Source of truth**: a single `KEYBINDINGS` const table in
   `help_overlay.rs` with all overlays' bindings declared. Each overlay
   author adds a row when they introduce a new binding.

## Files you may touch

- `crates/argo-tui/src/help_overlay.rs` (NEW, ≤ 280 LOC)
- `crates/argo-tui/src/help_overlay_tests.rs` (NEW, ≤ 180 LOC) — 5 unit tests:
  - render Global section only when no parent overlay
  - render Global + SessionsList sections when parent is sessions list
  - render correctly with 0 bindings (empty state)
  - q closes overlay
  - `?` toggles open/closed
- `crates/argo-tui/src/lib.rs` — narrowly: 1 `pub mod help_overlay;` line
- `crates/argo-tui/src/app.rs` — narrowly: 1 global key arm + 1 dispatch arm only

## Files you must NOT touch

- `crates/argo-tui/src/input.rs` (at 500 LOC ceiling)
- Any other overlay file beyond declaring its bindings in the table
- README — out of scope; help is in-app, not docs

## Acceptance criteria

- [ ] `?` opens help overlay from any TUI surface
- [ ] Help shows "Global" section + section for current overlay
- [ ] Each binding shows `key (or chord)` + description, two columns
- [ ] q or Esc closes overlay; `?` again also closes (toggle)
- [ ] All existing TUI keybindings (j/k/Enter/q/Tab/A/$/Ctrl-K/D etc.) appear in the table
- [ ] 5 unit tests pass
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-tui` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-tui-keybindings-help`
- PR title: `tui: context-aware ? help overlay with keybindings table (claude-tui-keybindings-help)`

## Notes

- The keybindings table is the source of truth — once this brief
  lands, follow-up briefs that add new keybindings should add rows here.
- Keep the table sorted by overlay then key for readability.
- Coordinate with whoever lands `app.rs` split — `?` is a global key
  routed from `app.rs`, so an arm there is unavoidable.
