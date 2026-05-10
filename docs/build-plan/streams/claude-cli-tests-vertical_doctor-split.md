# claude-cli-tests-vertical_doctor-split — split argo-cli/tests/vertical_doctor.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Tier 1 split-prep — pure refactor.**

**Authored against**: origin/main c780496b on 2026-05-10.

## Why

`crates/argo-cli/tests/vertical_doctor.rs` is at 425/500 LOC. Pre-emptive split keeps headroom.

## Approach (concrete)

1. **Identify a cohesive sibling group** — typically by doctor-check kind or scenario (happy-path vs failure-path).
2. **Extract to sibling** `tests/vertical_doctor_<topic>.rs` (Cargo treats each test file as its own binary).
3. **Test.** `cargo clippy -p argo-cli --all-targets -- -D warnings` clean. `cargo test -p argo-cli --tests` green.

## Files you may touch

- `crates/argo-cli/tests/vertical_doctor.rs` (must drop below 400 LOC)
- `crates/argo-cli/tests/vertical_doctor_<topic>.rs` (NEW, ≤ 300 LOC)
- `crates/argo-cli/tests/common/` — narrowly: shared helpers if needed

## Files you must NOT touch

- Any source file outside `crates/argo-cli/tests/`

## Acceptance criteria

- [ ] `vertical_doctor.rs` drops below 400 LOC
- [ ] New sibling created, ≤ 300 LOC
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-cli --tests` green

## Branch + PR

- Branch: `claude-cli-tests-vertical_doctor-split`
- PR title: `argo-cli: split tests/vertical_doctor.rs below 500-LOC ceiling (claude-cli-tests-vertical_doctor-split)`
