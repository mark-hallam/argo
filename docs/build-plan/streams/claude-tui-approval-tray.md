# claude-tui-approval-tray — pending approvals tray, accessible from any screen

**Owner**: claude. **Depends on**: none.

## Scope

Approvals currently appear inline in the message stream when a worker
hits a checkpoint. There is no global tray that shows _all_ pending
approvals across all workers, accessible from any TUI surface.

This brief adds a `approval_tray.rs` overlay reachable via `A` from any
screen. Tray lists every pending approval with worker id, action,
checkpoint reason, and quick approve/reject keys.

## Approach (concrete)

1. **NEW** `crates/argo-tui/src/approval_tray.rs` (~280 LOC):
   - `pub struct ApprovalTray { pending: Vec<PendingApproval>, selected: usize }`
   - `PendingApproval { id, session_id, worker_id, action_kind, reason, requested_at }`
   - `pub fn render(&self, frame: &mut Frame, area: Rect)` — bordered overlay (50% width centred), table layout
   - `pub fn handle_key(&mut self, key: KeyEvent) -> ApprovalOutcome` — j/k navigates, y approves selected, n rejects, q closes
   - `pub async fn refresh(&mut self, gateway: &GatewayClient)` — pulls `argo approvals list --pending`
2. **Hook** into `app.rs` global keybinding: 1 new arm in the global
   key dispatcher mapping `A` → open tray, regardless of current overlay.
3. **Approvals** flow through existing `argo approvals approve <id>` /
   `argo approvals reject <id>` — no new gateway endpoints.

## Data sources (no new wire)

- List: existing `argo approvals list --pending --json`
- Approve: existing `argo approvals approve <id>`
- Reject: existing `argo approvals reject <id>`

## Files you may touch

- `crates/argo-tui/src/approval_tray.rs` (NEW, ≤ 300 LOC)
- `crates/argo-tui/src/approval_tray_tests.rs` (NEW, ≤ 200 LOC) — 5 unit tests:
  - render with 0 pending
  - render with 5 pending
  - j/k navigation
  - y triggers approve outcome
  - n triggers reject outcome
- `crates/argo-tui/src/lib.rs` — narrowly: 1 `pub mod approval_tray;` line
- `crates/argo-tui/src/app.rs` — narrowly: 1 enum variant + 1 global key arm only

## Files you must NOT touch

- `crates/argo-tui/src/input.rs` (at 500 LOC ceiling)
- Existing approval flow in message stream — this brief is additive
- Any gateway / CLI / manifest file — TUI-only brief

## Acceptance criteria

- [ ] `A` key opens tray from sessions list, message stream, dashboard, any overlay
- [ ] Tray shows all pending approvals with worker id, action, reason
- [ ] j/k navigates list; y approves; n rejects
- [ ] After action, list auto-refreshes; if empty, shows "No pending approvals"
- [ ] q closes tray, restores previous overlay
- [ ] 5 unit tests pass
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-tui` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-tui-approval-tray`
- PR title: `tui: global pending approvals tray (claude-tui-approval-tray)`

## Notes

- The `A` key must be wired in the global key dispatch path, not
  per-overlay, so it works from anywhere. Look at how the existing `?`
  or `q` global keys are routed.
- Tray should be modal (eats keys until closed) to avoid double-routing.
- Coordinate with `app.rs` split if it lands first.


## Amendment 2026-05-15 (input_globals extension point — supersedes old may-touch)

Worker self-BLOCKED 2026-05-08 noting the original may-touch forbade `crates/argo-tui/src/input.rs` (where global-key dispatch lived). PR #1670 (`claude-tui-input-globals-split`) MERGED 2026-05-15 extracts the global-key dispatcher into a sibling `input_globals.rs` with a registry-builder API: `default_global_keys() -> Vec<GlobalKeyBinding>` and `lookup_global_key(registry, key) -> Option<GlobalKeyAction>`.

This brief is now claimable using the new extension point. Updated may-touch:

- `crates/argo-tui/src/approval_tray.rs` — NEW. The feature module itself (≤300 LOC).
- `crates/argo-tui/src/approval_tray_tests.rs` — NEW. Unit tests (≤200 LOC, 5+ cases).
- `crates/argo-tui/src/input_globals.rs` — narrowly: add ONE new variant `GlobalKeyAction::ApprovalTrayOverlay` + ONE new entry in `default_global_keys()` mapping key `A` to that variant. **≤15 modified lines total.**
- `crates/argo-tui/src/input.rs` — narrowly: add the dispatch arm that matches `GlobalKeyAction::ApprovalTrayOverlay` and invokes the new feature module. **≤15 modified lines.** (Previously forbidden — now in scope because the file is no longer the centralized dispatcher; it delegates to `input_globals` per #1670.)
- `crates/argo-tui/src/lib.rs` — narrowly: `pub mod approval_tray;` declaration. **≤3 modified lines.**

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
- `crates/argo-tui/src/approval_tray.rs` — NEW. Feature module. ≤300 LOC.
- `crates/argo-tui/src/approval_tray_tests.rs` — NEW. Unit tests, ≥5 cases. ≤200 LOC.
- `crates/argo-tui/src/lib.rs` — narrowly: `pub mod approval_tray;` declaration. ≤3 modified lines.
- `crates/argo-tui/src/input_globals.rs` — narrowly: add ONE new `GlobalKeyAction` variant + ONE entry in `default_global_keys()`. ≤15 modified lines.
- `crates/argo-tui/src/input.rs` — narrowly: dispatch arm matching the new GlobalKeyAction variant. ≤15 modified lines. (Input.rs is post-boot-helpers-split; no longer at LOC ceiling.)
- `crates/argo-tui/src/app.rs` — narrowly: state field on ViewerState (or equivalent) for the overlay's open/closed + cursor state. ≤30 modified lines.
- `crates/argo-tui/src/render.rs` — narrowly: call site in render loop when this overlay is the active overlay. ≤15 modified lines.
- `crates/argo-tui/src/overlays.rs` — narrowly: `pub fn draw_approval_tray(state, frame, area)` function. ≤200 LOC for the new function + helpers if cohesive.

**Forbidden edits:**
- Any other crate outside `argo-tui`.
- Anything not in the permitted list above.

**Rationale:** The previously-claimed `at 500 LOC ceiling` constraint on input.rs is stale — boot-helpers split (#1670) freed it to 122 LOC. State management lives in `app.rs::ViewerState`, not a separate `state.rs`. Overlay rendering is split across `render.rs` (dispatcher) + `overlays.rs` (per-overlay draw functions).
