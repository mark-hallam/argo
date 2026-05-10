# claude-cli-tests-gateway-smoke-streaming-split — split argo-cli/tests/gateway_smoke_streaming.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Tier 1 split-prep — pure refactor.**

**Authored against**: origin/main 61782374 on 2026-05-10.

## Why

`crates/argo-cli/tests/gateway_smoke_streaming.rs` is at 475/500 LOC, tight against the hard ceiling. Pre-emptive split keeps headroom for future feature briefs.

## Approach (concrete)

1. **Identify a cohesive sibling group** — the file has 9+ `#[test]` fns covering streaming, audit, costs, db_stats, metrics. Cohesive split: extract the audit-and-costs cluster (`audit_breaks_returns_empty_array_for_clean_chain`, `costs_total_returns_zero_rollup_for_empty_store`, `audit_count_returns_zero_for_clean_chain`, `audit_session_verify_returns_ok_true_for_empty_session`) to a sibling test file. Leave streaming + workspace + metrics in the original.
2. **Extract to sibling** `crates/argo-cli/tests/gateway_smoke_audit_costs.rs`. Each integration-test file is its own binary; duplicate small HTTP helpers if present (or factor into `tests/common/gateway.rs`).
3. **Re-export via `pub(crate) use`** (N/A for tests).
4. **Keep public API unchanged.**
5. **Test.** `cargo clippy -p argo-cli --all-targets -- -D warnings` clean. `cargo test -p argo-cli` green.

## Files you may touch

- `crates/argo-cli/tests/gateway_smoke_streaming.rs` (must drop below 400 LOC)
- `crates/argo-cli/tests/gateway_smoke_audit_costs.rs` (NEW, ≤ 300 LOC)
- `crates/argo-cli/tests/common/mod.rs` or `crates/argo-cli/tests/common/gateway.rs` (NEW, only if helpers are factored)

## Files you must NOT touch

- Any other source file
- Any test file outside `crates/argo-cli/tests/`

## Acceptance criteria

- [ ] `gateway_smoke_streaming.rs` drops below 400 LOC
- [ ] New sibling created, ≤ 300 LOC
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-cli` green
- [ ] Public API unchanged

## Branch + PR

- Branch: `claude-cli-tests-gateway-smoke-streaming-split`
- PR title: `argo-cli: split tests/gateway_smoke_streaming.rs below 500-LOC ceiling (claude-cli-tests-gateway-smoke-streaming-split)`
