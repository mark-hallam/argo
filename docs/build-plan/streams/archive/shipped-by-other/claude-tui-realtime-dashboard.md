# V1.4 — TUI real-time dashboard overlay

**Owner**: claude. **Depends on**: claude-tui-h-stack-panels
(#822), claude-realtime-audit-stream (#831) merged.

## Scope

Existing TUI has 10+ overlays (F/S/M/$/A/P/c/T/o/?). Add ONE
more: `D` for **Dashboard** — a single-screen real-time view
showing:

- Top: workers (one row each, current state + last action +
  spend today)
- Middle: pending approvals queue (newest first)
- Bottom: live audit tail (last 10 audit records, scrolling)

Auto-refreshes every 1s by subscribing to AuditSink. Becomes
the operator's "always-on" tab when running SKYCOT day-to-day.

## Files you may touch
- `crates/argo-tui/src/overlays_dashboard.rs` (NEW)
- `crates/argo-tui/src/lib.rs` — narrowly: register overlay +
  `D` keybinding
- `crates/argo-tui/src/input_dashboard.rs` (NEW — refresh
  interval, scroll keys for the audit tail)
- `crates/argo-tui/Cargo.toml` — no new deps

## Acceptance criteria
- [ ] `D` opens dashboard overlay
- [ ] All 3 panels render with live data
- [ ] Auto-refresh on AuditSink subscription (no polling)
- [ ] Pending-approvals panel shows worker + action_kind +
      age (e.g. "2m ago")
- [ ] Audit tail scrolls; arrow keys navigate
- [ ] Esc / `D` closes overlay
- [ ] 8 unit tests + 1 snapshot test
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `claude-tui-realtime-dashboard`
- PR title: `tui: real-time dashboard overlay (key D) (claude-tui-realtime-dashboard)`
