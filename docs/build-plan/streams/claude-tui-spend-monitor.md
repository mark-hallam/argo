# claude-tui-spend-monitor — live spend chart with sparkline (today / week / month)

**Owner**: claude. **Depends on**: none.

## Scope

Spend visibility in the TUI today is per-session only (cost field on
session detail). There is no aggregate, fleet-wide live spend view with
a time-series sparkline.

This brief adds a `spend_monitor.rs` overlay: 3 sparklines (today by
hour / this week by day / this month by day) plus current totals and
budget headroom indicator. Pure read-side over the existing
`argo budget` / `argo session cost` endpoints.

## Approach (concrete)

1. **NEW** `crates/argo-tui/src/spend_monitor.rs` (~310 LOC):
   - `pub struct SpendMonitor { today: SpendSeries, week: SpendSeries, month: SpendSeries, budget: BudgetSnapshot }`
   - `SpendSeries { points: Vec<f64>, total: f64, label: &'static str }`
   - `pub fn render(&self, frame: &mut Frame, area: Rect)` — 3 stacked panes via ratatui `Sparkline` widget + total/headroom line per pane
   - `pub fn handle_key(&mut self, key: KeyEvent) -> SpendOutcome` — q closes, r forces refresh
   - `pub async fn refresh(&mut self, gateway: &GatewayClient)` — pulls `argo budget show --json` + `argo spend timeseries --range today,week,month --json`
2. **Hook** into `app.rs`: 1 new enum variant + 1 dispatch arm + `$`
   keybinding (mnemonic for spend).
3. **If** a `spend timeseries` endpoint does not yet exist, this brief
   STOPS and surfaces the gap — does NOT add a new gateway endpoint.
   Look at existing `overlays_budget.rs` for the spend data path
   already in use.

## Data sources (no new wire)

- Budget: existing `argo budget show --json`
- Time series: existing `argo spend timeseries` if present, else
  aggregate from `argo session cost` across sessions

## Files you may touch

- `crates/argo-tui/src/spend_monitor.rs` (NEW, ≤ 330 LOC)
- `crates/argo-tui/src/spend_monitor_tests.rs` (NEW, ≤ 220 LOC) — 6 unit tests:
  - render empty (zero spend)
  - render with today/week/month all populated
  - sparkline scales correctly with outlier
  - budget headroom shows correct color (green/amber/red)
  - r triggers refresh outcome
  - q closes
- `crates/argo-tui/src/lib.rs` — narrowly: 1 `pub mod spend_monitor;` line
- `crates/argo-tui/src/app.rs` — narrowly: 1 enum variant + 1 dispatch arm + 1 keybinding only

## Files you must NOT touch

- `crates/argo-tui/src/input.rs` (at 500 LOC ceiling)
- `crates/argo-tui/src/overlays_budget.rs` beyond importing helpers
- Any gateway / CLI / manifest file — TUI-only brief

## Acceptance criteria

- [ ] `$` opens spend monitor overlay
- [ ] 3 sparklines render: today (24 points by hour), week (7 by day), month (30 by day)
- [ ] Each pane shows total spend + budget headroom (if budget set)
- [ ] Color coding: <50% headroom green, 50-80% amber, >80% red
- [ ] r forces refresh, q closes
- [ ] 6 unit tests pass
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-tui` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-tui-spend-monitor`
- PR title: `tui: live fleet spend monitor with sparklines (claude-tui-spend-monitor)`

## Notes

- ratatui has a built-in `Sparkline` widget — use it, do not roll your own.
- If `argo spend timeseries` does not exist, file a follow-up brief
  rather than expanding scope here.
- `overlays_budget.rs` already pulls budget data; reuse its client path.
