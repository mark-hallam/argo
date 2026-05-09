# mvp-split-auction — split auction.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Tier 1 MVP — proven split pattern.**

**Authored against**: origin/main 7fdbbdb8 on 2026-05-10.

## Why

`crates/argo-orchestrator/src/auction.rs` is at **683 LOC** — 183 over the 500-LOC ceiling. Blocks any further auction-pipeline work and trips clippy when CI re-enables. Pure refactor mirrors proven split pattern (#851, #882, #909, #910, #914, #915, #932, #935, #1116, #1121, #1125, #1126, #1130, #1131, #1132, #1133, #1134, #1135, #1136, #1137, #1146, #1147).

## Approach

1. Read `auction.rs` end-to-end. Identify a cohesive section (likely a bid-evaluation block, quorum-resolution block, or persistence helpers).
2. Move to `crates/argo-orchestrator/src/auction_<topic>.rs`. If natural split is large, extract two siblings (`auction_eval.rs` + `auction_persist.rs`).
3. Re-export via `pub(crate) use`.

Target: `auction.rs` ≤ 350 LOC.

## Files you may touch

- `crates/argo-orchestrator/src/auction.rs` — narrowly: extract sibling, re-export
- `crates/argo-orchestrator/src/auction_<topic>.rs` (NEW, ≤ 250 LOC each)
- `crates/argo-orchestrator/src/lib.rs` — narrowly: 1-2 `mod` declarations if needed

## Files you must NOT touch

- `crates/argo-orchestrator/src/persist.rs` / `walker.rs` — separate concerns
- Test files except to verify no behaviour change

## Acceptance criteria

- [ ] `auction.rs` ≤ 350 LOC (≥ 150 LOC headroom)
- [ ] Each sibling ≤ 250 LOC
- [ ] `cargo clippy -p argo-orchestrator --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-orchestrator` clean (same test count pre/post)
- [ ] No public API changes

## Branch + PR

- Branch: `mvp-split-auction`
- PR title: `orchestrator: split auction.rs below 500-LOC ceiling (mvp-split-auction)`
