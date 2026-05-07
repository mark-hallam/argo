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
