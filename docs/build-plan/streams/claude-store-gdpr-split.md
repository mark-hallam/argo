# claude-store-gdpr-split — split gdpr.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Phase A architectural prep — defensive split.**

**Authored against**: origin/main 6a753bae on 2026-05-10.

## Why

`crates/argo-store/src/gdpr.rs` is at 473 LOC. Future GDPR work (right-to-erasure expansions, audit-log redaction, retention windows) will push this over 500. Preemptive split mirrors proven pattern.

## Approach

1. Read `gdpr.rs` end-to-end. Identify a cohesive group (likely an operation family — erasure vs. redaction vs. retention).
2. Move to `crates/argo-store/src/gdpr_<topic>.rs`.
3. Re-export via `pub(crate) use`.

Target: `gdpr.rs` ≤ 350 LOC.

## Files you may touch

- `crates/argo-store/src/gdpr.rs` — narrowly: extract sibling, re-export
- `crates/argo-store/src/gdpr_<topic>.rs` (NEW, ≤ 250 LOC)
- `crates/argo-store/src/lib.rs` — narrowly: 1 `mod` declaration if needed

## Files you must NOT touch

- Migration files — pure refactor, no schema change
- Any test file other than to verify no behaviour change

## Acceptance criteria

- [ ] `gdpr.rs` ≤ 350 LOC (≥ 150 LOC headroom)
- [ ] Sibling ≤ 250 LOC
- [ ] `cargo clippy -p argo-store --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-store` clean
- [ ] No public API changes

## Branch + PR

- Branch: `claude-store-gdpr-split`
- PR title: `store: split gdpr.rs below 500-LOC ceiling (claude-store-gdpr-split)`
