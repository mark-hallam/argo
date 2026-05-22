# claude-cli-handlers-vertical_doctor-split — split argo-cli/src/handlers/vertical_doctor.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Tier 1 split-prep — pure refactor.**

**Authored against**: origin/main 15214e3e on 2026-05-10.

## Why

`crates/argo-cli/src/handlers/vertical_doctor.rs` is at 418/500 LOC. Pre-emptive split keeps headroom.

Note: distinct from `crates/argo-cli/tests/vertical_doctor.rs` (claude-cli-tests-vertical_doctor-split brief) — this brief targets the source handler, not the test file.

## Approach (concrete)

1. **Identify a cohesive sibling group** — typically by doctor-check kind (eg. manifest validation vs runtime probe) or by JSON output shaping.
2. **Extract to sibling** `vertical_doctor_<topic>.rs`.
3. **Re-export via `pub(crate) use`**.
4. **Test.** `cargo clippy -p argo-cli --all-targets -- -D warnings` clean. `cargo test -p argo-cli` green.

## Files you may touch

- `crates/argo-cli/src/handlers/vertical_doctor.rs` (must drop below 400 LOC)
- `crates/argo-cli/src/handlers/vertical_doctor_<topic>.rs` (NEW, ≤ 300 LOC)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 1 `mod` declaration

## Files you must NOT touch

- Any other source file
- Any test file outside `crates/argo-cli/`
- `crates/argo-cli/tests/vertical_doctor.rs` (different file, separate brief)

## Acceptance criteria

- [ ] `vertical_doctor.rs` drops below 400 LOC
- [ ] New sibling created, ≤ 300 LOC
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-cli` green
- [ ] Public API unchanged

## Branch + PR

- Branch: `claude-cli-handlers-vertical_doctor-split`
- PR title: `argo-cli: split handlers/vertical_doctor.rs below 500-LOC ceiling (claude-cli-handlers-vertical_doctor-split)`
