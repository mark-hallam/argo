# claude-cu-executor-extract-helper — extract helper from cu executor.rs + pre-clear cu-fail-restart may-touch

**Owner**: claude. **Depends on**: nothing. **Phase A architectural prep.**

**Authored against**: origin/main c2057ac1 on 2026-05-09 (conductor-authored prereq for claude-cu-fail-restart-strategy).

## Why

`crates/argo-executor-computer-use/src/executor.rs` is 481 LOC — 19 LOC below the ceiling. `claude-cu-fail-restart-strategy` needs to add restart-policy state and would push past it. Plus that brief is currently blocked on `#[serde(deny_unknown_fields)]` on `profile.rs`, which it can't address without may-touch widening.

This brief: (a) extracts a helper from `executor.rs` to give it headroom, and (b) pre-clears the `profile.rs` may-touch issue.

## Approach (concrete)

1. Read `executor.rs` end-to-end. Identify a self-contained section (typically the run-loop helpers or step-result accumulator) that can move to a sibling without behaviour change.
2. Move that section to `crates/argo-executor-computer-use/src/executor_<topic>.rs` (e.g. `executor_step.rs`).
3. Re-export from `executor.rs` via `pub use` so call sites are unchanged.
4. **Companion change**: append a note to `claude-cu-fail-restart-strategy.md` widening its may-touch list to include `crates/argo-executor-computer-use/src/profile.rs`.

## Files you may touch

- `crates/argo-executor-computer-use/src/executor.rs` — narrowly: extract sibling, re-export
- `crates/argo-executor-computer-use/src/executor_<topic>.rs` (NEW)
- `crates/argo-executor-computer-use/src/lib.rs` — narrowly: pub mod
- `docs/build-plan/streams/claude-cu-fail-restart-strategy.md` — narrowly: append `crates/argo-executor-computer-use/src/profile.rs` to may-touch

## Files you must NOT touch

- The Anthropic CU loop logic itself — refactor only
- Any test file other than verifying no behaviour change

## Acceptance criteria

- [ ] `executor.rs` ≤ 380 LOC after the split (≥ 100 LOC headroom)
- [ ] Extracted sibling ≤ 250 LOC
- [ ] `cargo clippy -p argo-executor-computer-use --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-executor-computer-use` clean
- [ ] No public API changes
- [ ] `claude-cu-fail-restart-strategy.md` may-touch list includes `profile.rs`

## Branch + PR

- Branch: `claude-cu-executor-extract-helper`
- PR title: `executor(cu): extract helper from executor.rs + pre-clear cu-fail-restart may-touch (claude-cu-executor-extract-helper)`

## Unblocks

- `claude-cu-fail-restart-strategy` — currently BLOCKED on executor.rs ceiling + profile.rs may-touch.
