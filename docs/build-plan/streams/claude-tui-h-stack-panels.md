# Claude — TUI panels for the H-stack (sessions / handoffs / budget)

**Owner**: claude. **Depends on**: H-orchestrator-persist
merged (so the TUI has rows to render); H-spend-persistence
merged (so budget panel has data).

## Scope

Extend the existing `argo-tui` (ratatui) with three new
overlay panels matching the existing 10-overlay pattern:
- `S` — Sessions panel (list active sessions + participants)
- `H` — Handoffs panel (list pending handoffs for the
  current worker, status, deadline)
- `$` — Budget panel (today's spend + monthly rollup +
  remaining headroom against the policy caps)

Each follows the existing overlay shape (modal popup, hotkey
to toggle, ESC to close, `j/k` to scroll). Reads via the
gateway's HTTP endpoints (`H-orchestrator-routes` and
`H-skill-library-routes` for cross-tier).

## Files you may touch
- `crates/argo-tui/src/overlays/sessions.rs` (NEW)
- `crates/argo-tui/src/overlays/handoffs.rs` (NEW)
- `crates/argo-tui/src/overlays/budget.rs` (NEW)
- `crates/argo-tui/src/overlays/mod.rs` — narrowly: register
  the three new modules
- `crates/argo-tui/src/app.rs` (or wherever overlays are
  toggled) — narrowly: add the three new hotkey handlers.
  Mirror the shape of an existing overlay (e.g. `M` memory).
- `crates/argo-tui/Cargo.toml` — add `reqwest` if not already
  there

## Files you must NOT touch
- Any non-tui crate
- The 10 existing overlays (don't refactor; only add new ones)

## Acceptance criteria
- [ ] Three new overlays render with sensible content
- [ ] Hotkeys `S`, `H`, `$` toggle the overlays
- [ ] ESC closes any open overlay
- [ ] `j` / `k` scroll lists when content overflows
- [ ] Each overlay handles "no data yet" cleanly (e.g.
      "No active sessions" rendered centred in the panel)
- [ ] Each overlay handles "gateway unreachable" — render
      a clear error inside the panel rather than crash
- [ ] Snapshot test for each overlay's render output
      (use the existing TUI snapshot pattern; search the
      crate for examples)
- [ ] `cargo clippy -p argo-tui --all-targets -- -D warnings`
      clean
- [ ] Every authored file < 500 LOC

## Branch + PR
- Branch: `claude-tui-h-stack-panels`
- PR title: `tui: sessions / handoffs / budget overlays (claude-tui-h-stack-panels)`

## Notes
- The TUI ships with 10 overlays today (F/S/M/$/A/P/c/T/o/?
  per the project state memory). `S` and `$` are already
  taken — pick alternative hotkeys (e.g. `O` for sessions,
  `H` already taken? check). Coordinate by looking at the
  existing key dispatcher first.
- Don't gate-block on missing gateway endpoints — if the
  routes briefs (`H-orchestrator-routes`) haven't merged
  yet, the panel renders the "gateway unreachable" state
  cleanly. The brief is independently mergeable.
