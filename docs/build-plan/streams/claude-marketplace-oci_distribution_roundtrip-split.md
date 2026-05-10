# claude-marketplace-oci_distribution_roundtrip-split — split argo-marketplace/tests/oci_distribution_roundtrip.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Tier 1 split-prep — pure refactor.**

**Authored against**: origin/main 8889b40c on 2026-05-10.

## Why

`crates/argo-marketplace/tests/oci_distribution_roundtrip.rs` is at 442/500 LOC. Pre-emptive split keeps headroom.

## Approach (concrete)

1. **Identify a cohesive sibling group** — typically by roundtrip phase (push vs pull vs verify) or by registry variant.
2. **Extract to sibling** `tests/oci_distribution_roundtrip_<topic>.rs`.
3. **Test.** `cargo clippy -p argo-marketplace --all-targets -- -D warnings` clean. `cargo test -p argo-marketplace --tests` green.

## Files you may touch

- `crates/argo-marketplace/tests/oci_distribution_roundtrip.rs` (must drop below 400 LOC)
- `crates/argo-marketplace/tests/oci_distribution_roundtrip_<topic>.rs` (NEW, ≤ 300 LOC)

## Files you must NOT touch

- Any source file outside `crates/argo-marketplace/tests/`

## Acceptance criteria

- [ ] `oci_distribution_roundtrip.rs` drops below 400 LOC
- [ ] New sibling created, ≤ 300 LOC
- [ ] `cargo clippy -p argo-marketplace --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-marketplace --tests` green

## Branch + PR

- Branch: `claude-marketplace-oci_distribution_roundtrip-split`
- PR title: `argo-marketplace: split tests/oci_distribution_roundtrip.rs below 500-LOC ceiling (claude-marketplace-oci_distribution_roundtrip-split)`
