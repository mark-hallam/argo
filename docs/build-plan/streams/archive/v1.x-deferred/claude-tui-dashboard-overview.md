# claude-tui-dashboard-overview — fleet view: workers + status + recent activity

**Owner**: claude. **Depends on**: none.

## Scope

Top-level TUI dashboard surface that gives an at-a-glance view of the
worker fleet. Today the TUI lands on either the sessions list or the
last-active overlay; there is no single screen that shows _every_
worker, its current status, and the last 3-5 events without drilling in.

This brief adds a new `dashboard.rs` overlay-style module: a 3-pane
layout (workers column / status column / recent-activity tail) rendered
through ratatui. Pure read-side: no actions, no mutation, no new
manifest fields, no new endpoints — it composes data the gateway and
worker registry already expose.

## Approach (concrete)

1. **NEW** `crates/argo-tui/src/dashboard.rs` (~300 LOC):
   - `pub struct DashboardOverlay { workers: Vec<WorkerRow>, activity: VecDeque<EventRow>, selected: usize }`
   - `WorkerRow { id, status (Idle/Busy/Error/Offline), current_session, last_seen }`
   - `EventRow { ts, worker_id, kind, summary }` — kind enum mirrors gateway event taxonomy
   - `pub fn render(&self, frame: &mut Frame, area: Rect)` — splits area into 3 vertical panes via `Layout::default().constraints([40%, 30%, 30%])`
   - `pub fn handle_key(&mut self, key: KeyEvent) -> DashboardOutcome` — j/k navigation, Enter to drill into selected worker
2. **Hook** into `app.rs` via a single new variant on the existing
   overlay enum + 1 dispatch arm. Do NOT grow `app.rs` past 500 LOC —
   if a split is needed, land it as a precursor PR first.
3. **Reuse** existing styling helpers from `render.rs` (status colors,
   timestamp formatting). Do NOT duplicate.

## Data sources (no new wire)

- Workers list: existing `argo workers list` JSON shape
- Activity tail: existing `argo session events --tail` stream
- Refresh tick: existing 1Hz overlay refresh in `app.rs`

## Files you may touch

- `crates/argo-tui/src/dashboard.rs` (NEW, ≤ 320 LOC)
- `crates/argo-tui/src/dashboard_tests.rs` (NEW, ≤ 200 LOC) — 5 unit tests:
  - render with 0 workers
  - render with 5 workers + 10 events
  - j/k navigation wraps
  - status color mapping
  - activity tail truncates to N
- `crates/argo-tui/src/lib.rs` — narrowly: 1 `pub mod dashboard;` line
- `crates/argo-tui/src/app.rs` — narrowly: 1 enum variant + 1 dispatch arm only

## Files you must NOT touch

- `crates/argo-tui/src/input.rs` (at 500 LOC ceiling)
- Any gateway / CLI / manifest file — TUI-only brief
- `crates/argo-tui/src/render.rs` beyond importing helpers

## Acceptance criteria

- [ ] `D` keybinding opens the dashboard overlay
- [ ] 3-pane layout renders correctly at 80x24 and 200x60 terminal sizes
- [ ] j/k navigates worker list, Enter drills into session detail
- [ ] Status column shows live worker state with color coding
- [ ] Activity tail shows last 20 events, newest first
- [ ] 5 unit tests pass
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-tui` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-tui-dashboard-overview`
- PR title: `tui: top-level fleet dashboard overlay (claude-tui-dashboard-overview)`

## Notes

- `app.rs` is at 498 LOC — if your 1-arm dispatch addition pushes it
  over, land an `app.rs` split as a precursor PR before this one.
- Look at `overlays_dashboard.rs` for an existing 3-pane pattern.
- Keep the activity tail bounded (`VecDeque` with cap of 100) to
  prevent unbounded growth on long-lived TUI sessions.
