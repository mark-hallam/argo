# claude-cli-tests-gateway-smoke-v2-split — split argo-cli/tests/gateway_smoke_v2.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Tier 1 split-prep — pure refactor.**

**Authored against**: origin/main 61782374 on 2026-05-10.

## Why

`crates/argo-cli/tests/gateway_smoke_v2.rs` is at 482/500 LOC, tight against the hard ceiling. Pre-emptive split keeps headroom for future feature briefs.

## Approach (concrete)

1. **Identify a cohesive sibling group** — the file has shared HTTP helpers (`argo_bin`, `pick_free_port`, `http_get`, `http_post`, `body_of`, `await_listening`) plus 7+ `#[test]` fns. Move ~3-4 of the latter `#[test]` fns (e.g., `forge_runs_returns_empty_array_on_clean_db`, `routes_endpoint_lists_known_paths`, `workspace_artifacts_returns_empty_array_on_clean_db`) to a sibling integration-test file. Each integration-test file is its own crate compilation unit, so duplicate the small helpers as needed (or factor into a `common/` module per cargo convention).
2. **Extract to sibling** `crates/argo-cli/tests/gateway_smoke_v2_extras.rs` (or split helpers into `tests/common/gateway.rs` and re-import).
3. **Re-export via `pub(crate) use`** (N/A for tests; tests are independent binaries).
4. **Keep public API unchanged.**
5. **Test.** `cargo clippy -p argo-cli --all-targets -- -D warnings` clean. `cargo test -p argo-cli` green.

## Files you may touch

- `crates/argo-cli/tests/gateway_smoke_v2.rs` (must drop below 400 LOC)
- `crates/argo-cli/tests/gateway_smoke_v2_extras.rs` (NEW, ≤ 300 LOC)
- `crates/argo-cli/tests/common/mod.rs` or `crates/argo-cli/tests/common/gateway.rs` (NEW, only if helpers are factored)

## Files you must NOT touch

- Any other source file
- Any test file outside `crates/argo-cli/tests/`

## Acceptance criteria

- [ ] `gateway_smoke_v2.rs` drops below 400 LOC
- [ ] New sibling created, ≤ 300 LOC
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-cli` green
- [ ] Public API unchanged

## Branch + PR

- Branch: `claude-cli-tests-gateway-smoke-v2-split`
- PR title: `argo-cli: split tests/gateway_smoke_v2.rs below 500-LOC ceiling (claude-cli-tests-gateway-smoke-v2-split)`
