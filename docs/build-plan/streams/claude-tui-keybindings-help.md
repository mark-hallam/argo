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


## Amendment 2026-05-15 (input_globals extension point — supersedes old may-touch)

Worker self-BLOCKED 2026-05-08 noting the original may-touch forbade `crates/argo-tui/src/input.rs` (where global-key dispatch lived). PR #1670 (`claude-tui-input-globals-split`) MERGED 2026-05-15 extracts the global-key dispatcher into a sibling `input_globals.rs` with a registry-builder API: `default_global_keys() -> Vec<GlobalKeyBinding>` and `lookup_global_key(registry, key) -> Option<GlobalKeyAction>`.

This brief is now claimable using the new extension point. Updated may-touch:

- `crates/argo-tui/src/keybindings_help.rs` — NEW. The feature module itself (≤300 LOC).
- `crates/argo-tui/src/keybindings_help_tests.rs` — NEW. Unit tests (≤200 LOC, 5+ cases).
- `crates/argo-tui/src/input_globals.rs` — narrowly: add ONE new variant `GlobalKeyAction::KeybindingsHelpOverlay` + ONE new entry in `default_global_keys()` mapping key `?` to that variant. **≤15 modified lines total.**
- `crates/argo-tui/src/input.rs` — narrowly: add the dispatch arm that matches `GlobalKeyAction::KeybindingsHelpOverlay` and invokes the new feature module. **≤15 modified lines.** (Previously forbidden — now in scope because the file is no longer the centralized dispatcher; it delegates to `input_globals` per #1670.)
- `crates/argo-tui/src/lib.rs` — narrowly: `pub mod keybindings_help;` declaration. **≤3 modified lines.**

Files still forbidden:
- `crates/argo-tui/src/render.rs` — overlay rendering is separate from input dispatch.
- `crates/argo-tui/src/overlays.rs` — same.
- Any other crate.

Acceptance criteria (in addition to the brief's original list):
- New variant added to `GlobalKeyAction` enum.
- New key binding registered in `default_global_keys()`.
- `cargo test -p argo-tui input_globals` includes a coverage case for the new key→variant mapping.
- No regression: existing global keys still dispatch correctly.

**Drift-prevention:** brief adds ONE new pub variant + ONE new pub module — no parallel-name surfaces.


## Amendment 2026-05-15 #2 (rendering scope widening — codex worker self-BLOCK feedback)

Codex worker correctly self-BLOCKED noting that input-side scope alone is insufficient: the overlay renders through `state.<overlay_field>` + `render.rs`/`overlays.rs::draw_<overlay>`. The previous Amendment only opened input dispatch; visible overlay content also needs scope.

Further widening:

- `crates/argo-tui/src/render.rs` — narrowly: add ONE call site in the render loop that invokes the new overlay's render function when the overlay is active. **≤15 modified lines.**
- `crates/argo-tui/src/overlays.rs` — narrowly: add ONE `pub fn draw_<feature>(state, frame, area)` function for the new overlay's visible content. **≤200 LOC for the new function** (may include sub-state struct + helpers in the same file if cohesive).
- `crates/argo-tui/src/state.rs` (or wherever app state lives) — narrowly: add ONE field for the overlay's open/closed state + any cursor/selection sub-state. **≤30 modified lines.**

These are the same load-bearing edits the original brief implied. With both amendments in place, the brief is fully claimable:
- Input layer: `input_globals.rs` (variant + key) + `input.rs` (dispatch arm)
- Rendering layer: `state.rs` (overlay field) + `overlays.rs` (`draw_*` fn) + `render.rs` (call site)
- Module: NEW feature module + tests + `lib.rs` declaration

No file > 500 LOC after; behavior of existing overlays preserved.


## Files you may touch — AUTHORITATIVE 2026-05-15 evening (supersedes original list)

This list is the canonical may-touch — codex pre-claim parsers should treat this as the enforceable boundary. Original `## Files you may touch` and `## Files you must NOT touch` sections above are advisory only and may be stale.

**Permitted edits:**
- `crates/argo-tui/src/keybindings_help.rs` — NEW. Feature module. ≤300 LOC.
- `crates/argo-tui/src/keybindings_help_tests.rs` — NEW. Unit tests, ≥5 cases. ≤200 LOC.
- `crates/argo-tui/src/lib.rs` — narrowly: `pub mod keybindings_help;` declaration. ≤3 modified lines.
- `crates/argo-tui/src/input_globals.rs` — narrowly: add ONE new `GlobalKeyAction` variant + ONE entry in `default_global_keys()`. ≤15 modified lines.
- `crates/argo-tui/src/input.rs` — narrowly: dispatch arm matching the new GlobalKeyAction variant. ≤15 modified lines. (Input.rs is post-boot-helpers-split; no longer at LOC ceiling.)
- `crates/argo-tui/src/app.rs` — narrowly: state field on ViewerState (or equivalent) for the overlay's open/closed + cursor state. ≤30 modified lines.
- `crates/argo-tui/src/render.rs` — narrowly: call site in render loop when this overlay is the active overlay. ≤15 modified lines.
- `crates/argo-tui/src/overlays.rs` — narrowly: `pub fn draw_keybindings_help(state, frame, area)` function. ≤200 LOC for the new function + helpers if cohesive.

**Forbidden edits:**
- Any other crate outside `argo-tui`.
- Anything not in the permitted list above.

**Rationale:** The previously-claimed `at 500 LOC ceiling` constraint on input.rs is stale — boot-helpers split (#1670) freed it to 122 LOC. State management lives in `app.rs::ViewerState`, not a separate `state.rs`. Overlay rendering is split across `render.rs` (dispatcher) + `overlays.rs` (per-overlay draw functions).
