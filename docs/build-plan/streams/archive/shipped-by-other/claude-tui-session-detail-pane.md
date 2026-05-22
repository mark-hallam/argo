# claude-tui-session-detail-pane — drill-in view: messages + audit + cost + skills

**Owner**: claude. **Depends on**: none (composes existing read endpoints).

## Scope

Today, selecting a session in the TUI sessions list opens the message
stream only. There is no single drill-in pane that shows the full
session shape: messages, audit trail, accumulated cost, and active
skills, all in one view.

This brief adds a `session_detail.rs` module: a 4-tab pane (Messages /
Audit / Cost / Skills) reachable from any session row via Enter. Each
tab is a thin reader over an existing gateway endpoint — no new wire.

## Approach (concrete)

1. **NEW** `crates/argo-tui/src/session_detail.rs` (~340 LOC):
   - `pub struct SessionDetailPane { session_id: SessionId, tab: Tab, messages: Vec<MessageRow>, audit: Vec<AuditRow>, cost: CostSummary, skills: Vec<SkillRow> }`
   - `enum Tab { Messages, Audit, Cost, Skills }` with `Tab::next/prev()`
   - `pub fn render(&self, frame: &mut Frame, area: Rect)` — top tab bar + body pane
   - `pub fn handle_key(&mut self, key: KeyEvent) -> DetailOutcome` — Tab/Shift-Tab cycles tabs, j/k scrolls body, q closes
   - `pub async fn refresh(&mut self, gateway: &GatewayClient)` — re-pulls all 4 tab data sources
2. **Hook** into `app.rs`: 1 new enum variant + 1 dispatch arm.
3. **Reuse** existing `MessageRow` rendering from `viewer.rs`; DO NOT
   duplicate message formatting logic.

## Data sources (no new wire)

- Messages: existing `argo session messages --json`
- Audit: existing `argo session audit --json`
- Cost: existing `argo session cost --json` (or aggregate from messages)
- Skills: existing `argo session skills --json`

## Files you may touch

- `crates/argo-tui/src/session_detail.rs` (NEW, ≤ 360 LOC)
- `crates/argo-tui/src/session_detail_tests.rs` (NEW, ≤ 220 LOC) — 6 unit tests:
  - render Messages tab with 5 messages
  - render Audit tab with 3 events
  - render Cost tab with non-zero spend
  - render Skills tab with 2 active skills
  - Tab/Shift-Tab cycles correctly
  - q returns DetailOutcome::Close
- `crates/argo-tui/src/lib.rs` — narrowly: 1 `pub mod session_detail;` line
- `crates/argo-tui/src/app.rs` — narrowly: 1 enum variant + 1 dispatch arm only

## Files you must NOT touch

- `crates/argo-tui/src/input.rs` (at 500 LOC ceiling)
- `crates/argo-tui/src/viewer.rs` beyond importing helpers
- Any gateway / CLI / manifest file — TUI-only brief

## Acceptance criteria

- [ ] Enter on a session row opens the detail pane
- [ ] Tab/Shift-Tab cycles 4 tabs forward/backward
- [ ] Each tab renders correctly with sample data + with empty state
- [ ] j/k scrolls within the active tab body
- [ ] q closes the pane and returns to sessions list
- [ ] 6 unit tests pass
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-tui` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-tui-session-detail-pane`
- PR title: `tui: session detail pane with messages/audit/cost/skills tabs (claude-tui-session-detail-pane)`

## Notes

- `app.rs` is at 498 LOC — coordinate with whoever lands an `app.rs`
  split first, or land one yourself as a precursor.
- Cost tab is the simplest — start there.
- Tab bar styling: reuse the existing tab pattern from
  `overlays_h_stack.rs` if present.
