# claude-tui-render-split — split argo-tui/src/render.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Tier 1 split-prep — pure refactor.**

**Authored against**: origin/main 444da442 on 2026-05-10.

## Why

`crates/argo-tui/src/render.rs` is at 443/500 LOC. Pre-emptive split keeps headroom for future feature briefs.

## Approach (concrete)

1. **Identify a cohesive sibling group** — typically by view (e.g. session-list render vs detail render) or by widget cluster (header vs body vs footer). Read end-to-end.
2. **Extract to sibling** `render_<topic>.rs`.
3. **Re-export via `pub(crate) use`**.
4. **Test.** `cargo clippy -p argo-tui --all-targets -- -D warnings` clean. `cargo test -p argo-tui` green.

## Files you may touch

- `crates/argo-tui/src/render.rs` (must drop below 400 LOC)
- `crates/argo-tui/src/render_<topic>.rs` (NEW, ≤ 300 LOC)
- `crates/argo-tui/src/lib.rs` — narrowly: 1 `mod` declaration

## Files you must NOT touch

- Any other source file
- Any test file outside `crates/argo-tui/`

## Acceptance criteria

- [ ] `render.rs` drops below 400 LOC
- [ ] New sibling created, ≤ 300 LOC
- [ ] `cargo clippy -p argo-tui --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-tui` green
- [ ] Public API unchanged

## Branch + PR

- Branch: `claude-tui-render-split`
- PR title: `argo-tui: split render.rs below 500-LOC ceiling (claude-tui-render-split)`
