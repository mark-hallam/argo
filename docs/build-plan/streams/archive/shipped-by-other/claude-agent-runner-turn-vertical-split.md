# claude-agent-runner-turn-vertical-split — split turn_vertical.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Phase A architectural prep — defensive split.**

**Authored against**: origin/main 828e5e1b on 2026-05-10.

## Why

`crates/argo-agent-runner/src/turn_vertical.rs` is at 474 LOC. Future vertical-aware turn hooks (per-vertical retries, escalation policy) push this over 500. Pure refactor mirrors proven pattern.

## Approach

1. Read `turn_vertical.rs` end-to-end. Identify a cohesive group (likely a per-hook category — pre-turn vs. mid-turn vs. post-turn).
2. Move to `crates/argo-agent-runner/src/turn_vertical_<topic>.rs`.
3. Re-export via `pub(crate) use` if needed.

Target: `turn_vertical.rs` ≤ 350 LOC.

## Files you may touch

- `crates/argo-agent-runner/src/turn_vertical.rs` — narrowly: extract sibling
- `crates/argo-agent-runner/src/turn_vertical_<topic>.rs` (NEW, ≤ 250 LOC)
- `crates/argo-agent-runner/src/lib.rs` — narrowly: 1 `mod` declaration if needed

## Acceptance criteria

- [ ] `turn_vertical.rs` ≤ 350 LOC
- [ ] `cargo clippy -p argo-agent-runner --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-agent-runner` clean
- [ ] No public API changes

## Branch + PR

- Branch: `claude-agent-runner-turn-vertical-split`
- PR title: `agent-runner: split turn_vertical.rs below 500-LOC ceiling (claude-agent-runner-turn-vertical-split)`
