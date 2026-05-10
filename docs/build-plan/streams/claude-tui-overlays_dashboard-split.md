# claude-tui-overlays_dashboard-split — split argo-tui/src/overlays_dashboard.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Tier 1 split-prep — pure refactor.**

**Authored against**: origin/main c780496b on 2026-05-10.

## Why

`crates/argo-tui/src/overlays_dashboard.rs` is at 417/500 LOC. Pre-emptive split keeps headroom for future TUI overlay additions.

## Approach (concrete)

1. **Identify a cohesive sibling group** — typically by panel cluster or rendering helper group.
2. **Extract to sibling** `overlays_dashboard_<topic>.rs`.
3. **Re-export via `pub(crate) use`**.
4. **Test.** `cargo clippy -p argo-tui --all-targets -- -D warnings` clean. `cargo test -p argo-tui` green.

## Files you may touch

- `crates/argo-tui/src/overlays_dashboard.rs` (must drop below 400 LOC)
- `crates/argo-tui/src/overlays_dashboard_<topic>.rs` (NEW, ≤ 300 LOC)
- `crates/argo-tui/src/lib.rs` — narrowly: 1 `mod` declaration

## Files you must NOT touch

- Any other source file
- Any test file outside `crates/argo-tui/`

## Acceptance criteria

- [ ] `overlays_dashboard.rs` drops below 400 LOC
- [ ] New sibling created, ≤ 300 LOC
- [ ] `cargo clippy -p argo-tui --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-tui` green
- [ ] Public API unchanged

## Branch + PR

- Branch: `claude-tui-overlays_dashboard-split`
- PR title: `argo-tui: split overlays_dashboard.rs below 500-LOC ceiling (claude-tui-overlays_dashboard-split)`
