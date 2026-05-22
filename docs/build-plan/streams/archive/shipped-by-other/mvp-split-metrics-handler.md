# mvp-split-metrics-handler — split gateway/metrics.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Tier 1 MVP — proven split pattern.**

**Authored against**: origin/main 7fdbbdb8 on 2026-05-10.

## Why

`crates/argo-cli/src/handlers/gateway/metrics.rs` is at **545 LOC**. Blocks Prometheus-export brief and trips clippy when CI re-enables.

## Approach

1. Read `metrics.rs` end-to-end. Identify a cohesive section (likely a metric-family group: spend, latency, queue depth, audit counts).
2. Move to `crates/argo-cli/src/handlers/gateway/metrics_<topic>.rs`.
3. Re-export via `pub(super) use`.

Target: `metrics.rs` ≤ 350 LOC.

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/metrics.rs`
- `crates/argo-cli/src/handlers/gateway/metrics_<topic>.rs` (NEW, ≤ 250 LOC)
- `crates/argo-cli/src/handlers/gateway/mod.rs` — narrowly: 1 `mod` declaration

## Acceptance criteria

- [ ] `metrics.rs` ≤ 350 LOC
- [ ] Sibling ≤ 250 LOC
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-cli` clean (boundary note OK for HARD RULE 2 flakes)
- [ ] No public route or response-shape changes

## Branch + PR

- Branch: `mvp-split-metrics-handler`
- PR title: `cli: split gateway/metrics.rs below 500-LOC ceiling (mvp-split-metrics-handler)`
