# claude-cli-gateway_smoke_writes-split — split argo-cli/tests/gateway_smoke_writes.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Tier 1 split-prep — pure refactor.**

**Authored against**: origin/main 04b7d2e0 on 2026-05-10.

## Why

`crates/argo-cli/tests/gateway_smoke_writes.rs` is at 484/500 LOC, tight against the hard ceiling. Pre-emptive split keeps headroom for future write-side gateway smoke coverage (new POST/PATCH/DELETE routes shipping under mission-gateway-bearer-auth follow-ups).

## Approach (concrete)

1. **Identify a cohesive sibling group** — likely by HTTP verb cluster (POST vs PATCH vs DELETE), or by resource group (sessions / messages / runs / forge), or extract shared fixtures + helpers (boot + auth-header + tenant-seed) into a `mod common` via `#[path]`. Read the file end-to-end before deciding the cut.
2. **Extract to sibling** `gateway_smoke_writes_<topic>.rs` in `crates/argo-cli/tests/` as a separate integration-test binary, OR keep one test binary and use a `#[path = "gateway_smoke_writes_<topic>.rs"] mod <topic>;` inclusion. Either approach is acceptable as long as both files stay <500 LOC and all tests still run under `cargo test -p argo-cli`.
3. **No re-exports needed** for integration tests; ensure both test binaries are picked up by cargo (auto-discovered from `tests/` for the separate-binary approach; one new `mod` line for the `#[path]` approach).
4. **Keep test coverage unchanged** — every test that ran before must still run, with the same name.
5. **Test.** `cargo clippy -p argo-cli --all-targets -- -D warnings` clean. `cargo test -p argo-cli` green (skip pre-existing flakes per HARD RULE 2).

## Files you may touch

- `crates/argo-cli/tests/gateway_smoke_writes.rs` (must drop below 400 LOC)
- `crates/argo-cli/tests/gateway_smoke_writes_<topic>.rs` (NEW, ≤ 300 LOC)
- (Optional) a `tests/common/` helper module if shared fixtures are extracted

## Files you must NOT touch

- Any source file under `crates/argo-cli/src/`
- Any test file other than `gateway_smoke_writes*.rs` and an optional `common` helper

## Acceptance criteria

- [ ] `gateway_smoke_writes.rs` drops below 400 LOC
- [ ] New sibling test file created, ≤ 300 LOC
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-cli` green (modulo HARD RULE 2 known flakes)
- [ ] Same set of tests still discovered + executed

## Branch + PR

- Branch: `claude-cli-gateway_smoke_writes-split`
- PR title: `argo-cli: split tests/gateway_smoke_writes.rs below 500-LOC ceiling (claude-cli-gateway_smoke_writes-split)`
