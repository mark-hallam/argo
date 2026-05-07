# claude-tui-alert-notifications — toast notifications for incoming approvals + errors

**Owner**: claude. **Depends on**: claude-tui-approval-tray (#TBD) — soft, not strict; the tray brief defines the approval data shape this surface listens to.

## Scope

Today, when a worker hits an approval checkpoint or errors, the user
only sees it if they are looking at that session's message stream.
There is no global, transient surface that surfaces these events from
any TUI screen.

This brief adds a `notifications.rs` module: a toast queue rendered in
the top-right of the screen across all overlays. Toasts auto-dismiss
after a configurable timeout (default 5s) and stack vertically.

## Approach (concrete)

1. **NEW** `crates/argo-tui/src/notifications.rs` (~270 LOC):
   - `pub struct ToastQueue { toasts: VecDeque<Toast>, max_visible: usize }`
   - `Toast { id, kind: ToastKind, title: String, body: String, created_at: Instant, ttl: Duration, dismissable: bool }`
   - `enum ToastKind { ApprovalRequested, Error, Info, Warning }`
   - `pub fn push(&mut self, toast: Toast)` — caps at `max_visible` (default 5), evicts oldest
   - `pub fn tick(&mut self, now: Instant)` — drops expired toasts
   - `pub fn render(&self, frame: &mut Frame, area: Rect)` — top-right column, each toast a 3-line bordered block with theme-aware color per kind
   - `pub fn handle_key(&mut self, key: KeyEvent) -> ToastOutcome` — `x` dismisses topmost dismissable toast; otherwise returns Pass
2. **Hook** into `app.rs` event loop: every event tick, the app
   inspects the gateway event stream, and for `ApprovalRequested` or
   `Error` events not already shown, it pushes a toast. Tick the queue
   each frame to expire old toasts.
3. **Render order**: toasts render LAST (on top of any overlay) but do
   NOT eat input — they are passive surface unless `x` is pressed.

## Data sources (no new wire)

- Approval events: existing gateway event stream
- Error events: existing gateway event stream

## Files you may touch

- `crates/argo-tui/src/notifications.rs` (NEW, ≤ 290 LOC)
- `crates/argo-tui/src/notifications_tests.rs` (NEW, ≤ 220 LOC) — 7 unit tests:
  - push/tick/expire lifecycle
  - max_visible eviction (push 6, see oldest dropped)
  - render with 0/1/3 toasts at various widths
  - color per ToastKind
  - x dismisses topmost dismissable
  - non-dismissable toast ignores x
  - tick at exact ttl boundary expires correctly
- `crates/argo-tui/src/lib.rs` — narrowly: 1 `pub mod notifications;` line
- `crates/argo-tui/src/app.rs` — narrowly: 1 field on App + 1 tick call in event loop + 1 render call. NO new dispatch arm (toasts are not an overlay).

## Files you must NOT touch

- `crates/argo-tui/src/input.rs` (at 500 LOC ceiling)
- Gateway event stream code — this brief consumes existing events only
- Any manifest / CLI / gateway file — TUI-only brief

## Acceptance criteria

- [ ] Toast appears top-right when an approval is requested for any session
- [ ] Toast appears top-right when an error event arrives for any session
- [ ] Toasts stack vertically, max 5 visible
- [ ] Toasts auto-dismiss after 5s (configurable via `[tui.notifications] ttl_ms = 5000`)
- [ ] `x` dismisses the topmost toast immediately
- [ ] Toasts do NOT consume keys other than `x` — overlay below remains interactive
- [ ] Color per kind matches theme (ApprovalRequested = accent, Error = error color, etc.)
- [ ] 7 unit tests pass
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-tui` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-tui-alert-notifications`
- PR title: `tui: toast notifications for approvals + errors (claude-tui-alert-notifications)`

## Notes

- Render LAST so toasts overlay everything else.
- Do NOT consume input — toast handling is `KeyEvent` peek only;
  unrecognized keys fall through to the active overlay.
- Coordinate color usage with `claude-tui-theme-system` — if that PR
  has not landed, hard-code colors that match the dark preset and TODO
  the theme switch.
- ttl_ms config goes under a new `[tui.notifications]` config section
  via `argo-config`.
