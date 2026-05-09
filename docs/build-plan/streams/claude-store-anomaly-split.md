# claude-store-anomaly-split — split anomaly.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Phase A architectural prep — defensive split.**

**Authored against**: origin/main 2bb5464d on 2026-05-10.

## Why

`crates/argo-store/src/anomaly.rs` is at 470 LOC. Future anomaly-detection extensions will exceed 500. Pure refactor mirrors proven pattern.

## Approach

1. Read `anomaly.rs` end-to-end. Identify a cohesive group (likely a detector family or per-rule helpers).
2. Move to `crates/argo-store/src/anomaly_<topic>.rs`.
3. Re-export via `pub(crate) use` where needed.

Target: `anomaly.rs` ≤ 350 LOC.

## Files you may touch

- `crates/argo-store/src/anomaly.rs` — narrowly: extract sibling
- `crates/argo-store/src/anomaly_<topic>.rs` (NEW, ≤ 250 LOC)
- `crates/argo-store/src/lib.rs` — narrowly: 1 `mod` declaration if needed

## Acceptance criteria

- [ ] `anomaly.rs` ≤ 350 LOC
- [ ] Sibling ≤ 250 LOC
- [ ] `cargo clippy -p argo-store --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-store` clean
- [ ] No public API changes

## Branch + PR

- Branch: `claude-store-anomaly-split`
- PR title: `store: split anomaly.rs below 500-LOC ceiling (claude-store-anomaly-split)`
