# claude-tui-render-tests-split — split render_tests.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Phase A architectural prep — defensive split.**

**Authored against**: origin/main on 2026-05-09 (conductor-authored bulk-replenishment after observing render_tests.rs at 485/500 LOC).

## Why

`crates/argo-tui/src/render_tests.rs` is at 485 LOC on current main — 15 LOC of headroom. Pure refactor; mirrors proven pattern.

## Approach

1. Read `render_tests.rs` end-to-end. Identify a cohesive section.
2. Move to a sibling file `render_tests_<topic>.rs`.
3. Re-export via `pub(crate) use`.

Target: ≤ 350 LOC.

## Files you may touch

- `crates/argo-tui/src/render_tests.rs` — extract sibling, re-export
- `crates/argo-tui/src/render_tests_<topic>.rs` (NEW, ≤ 250 LOC)
- `crates/argo-tui/src/lib.rs` — narrowly: 1 `mod` line if needed

## Files you must NOT touch

- Other crate modules
- Any test file other than to verify no behaviour change

## Acceptance criteria

- [ ] `render_tests.rs` ≤ 350 LOC
- [ ] Sibling ≤ 250 LOC
- [ ] `cargo clippy -p argo-tui --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-tui` clean

## Branch + PR

- Branch: `claude-tui-render-tests-split`
- PR title: `tui: split render_tests.rs below 500-LOC ceiling (claude-tui-render-tests-split)`

## Unblocks

Future briefs that extend `render_tests.rs` without hitting the ceiling.
