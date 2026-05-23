# eval-mcp-client-pool-tenant-keyed

**Priority**: P2 (claim-5 substrate hardening — pin tenant-keyed pool isolation as a regression test)
**Owner**: claude
**Type**: Regression test (no production code changes)
**Size**: ~200-350 LOC in one new integration test file
**Authored against**: origin/main `cd338914f` on 2026-05-24
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-mcp-client-pool-tenant-keyed`

## Why

`crates/argo-mcp/src/pool.rs` module documentation reads (lines 22-28):

> ## Per-tenant isolation
>
> The pool key is `(TenantId, manifest_id)` — never just
> `manifest_id`. Two tenants asking for the same manifest get
> distinct subprocesses with distinct credentials. Cross-tenant
> collision would leak credentials across tenants and is the only
> *correctness* hazard in this module.

This is one of MISSION.md's five non-negotiable invariants ("multi-tenant
by construction" — invariant #2) and **the entire claim-5 substrate
depends on it**. Round 4's evaluator audit observed that a prior
`McpConnectionPool` parallel surface in `argo-agent-runner` was a P0
cross-tenant credential leak — the bug already happened once on this
codebase. Today only `pool_tests.rs` (unit tests, in-module) protect this
invariant; the gap-manifest §"`argo-mcp` (core)" lists this brief as the
P2 follow-up.

What's missing is a **black-box regression test** that exercises
`acquire(TenantId, manifest_id, ...)` from a consumer crate — not the
in-module pool tests, which are friendly to the producer. The substrate
already exists; this brief pins the contract.

## What

Single new integration test file at
`crates/argo-mcp/tests/cross_tenant_pool_isolation_test.rs` (~200-350 LOC,
~4 `#[tokio::test]`s). Each test must use the public `McpClientPool` /
`ClientFactory` API; do **NOT** read private module state.

### Test 1: two tenants get distinct client identities

- `let pool = McpClientPool::new(...);`
- `let t1 = TenantId::new(); let t2 = TenantId::new();`
- `pool.acquire(t1, "echo-connector", || async { fake_client_a() }).await`
  → assert returns guard A.
- `pool.acquire(t2, "echo-connector", || async { fake_client_b() }).await`
  → assert returns guard B (distinct from A).
- The factory closure is called **twice** (once per tenant) — assert this
  via a `Arc<AtomicUsize>` counter in the closure. If the pool keyed on
  `manifest_id` alone, the counter would be 1.

### Test 2: same tenant + same manifest reuses the client

- Same setup as Test 1 with `t1` only.
- Two sequential `acquire(t1, "echo-connector", ...)` calls.
- Assert the factory closure ran **once** (cache hit on second call).
- The guards may be distinct values but the underlying client identity
  must be the same — use a unique-id field stamped by the factory.

### Test 3: credentials provided to factory are not shared across tenants

- Build a fake `ClientFactory` that records the credentials it was handed.
- Run `acquire(t1, m, || async { creds_a })` and
  `acquire(t2, m, || async { creds_b })` interleaved.
- Assert the recorded credential set for tenant_a never contains
  `creds_b`'s bytes and vice versa — this is the actual leak vector the
  bug exists to prevent.

### Test 4: pool key drift sentinel

- Walk `McpClientPool`'s public surface via reflection-style assertions
  (e.g. assert the signature of `acquire` is `fn(self, TenantId,
  &str, ...)`).
- If a future PR changes the `acquire` signature to drop `TenantId`, this
  test FAILS at compile time — the type system carries the invariant.
- Concretely: a `const _: fn(&McpClientPool, TenantId, &str, ...) = ...`
  binding that won't compile if the signature changes.

## Acceptance

- [ ] All 4 tests pass on `cargo test -p argo-mcp --test cross_tenant_pool_isolation_test`.
- [ ] `cargo clippy -p argo-mcp --all-targets -- -D warnings` clean.
- [ ] Workspace-clippy fast-fail clean.
- [ ] The test file cites
      `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md`
      §"`argo-mcp` (core)" verbatim in a doc-comment at the top, so a
      reader can trace the regression back to the audit gap that
      authorised it.
- [ ] No production code touched — `crates/argo-mcp/src/*.rs` unchanged.

## May-touch

- `crates/argo-mcp/tests/cross_tenant_pool_isolation_test.rs` (NEW)

**Out of may-touch**: `crates/argo-mcp/src/*.rs`, every other crate.

## Parallel surfaces

`rg 'cross_tenant_pool' crates/` returns zero hits — net-new test. The
sibling `connector_inventory_pin_test.rs` already exists in
`crates/argo-mcp/tests/` and is the structural inventory pin; this test
is the *behavioural* pin (different concern, separate file).

`rg 'McpClientPool::acquire' crates/` returns the unit tests in
`pool_tests.rs`. Inspect those at claim time to make sure you don't
duplicate an existing scenario — those tests are in-module (white-box);
this brief's tests are integration-style (black-box). The scenarios above
should add coverage, not duplicate it.

## Sensible deviations

- If the public `McpClientPool` API requires a non-trivial setup
  (rate-limiter, factory wiring, etc.), build a `fixtures::test_pool()`
  helper inside the new test file. **Do not export it.**
- If `McpClientPool::acquire` returns a `Result<_, _>`, use
  `?` and `.unwrap()` where appropriate per `argo-mcp`'s existing test
  idioms.
- If a factory closure can't return a real `McpClient` without spawning a
  subprocess, use a fake — the contract under test is the **pool's
  keying** behaviour, not what the client does once you have it. Mock at
  the factory boundary, not below.
- If you find Test 4's `const _: fn(...)` binding doesn't compile because
  of generic parameters, swap it for a runtime assertion in a `#[test]`
  block that calls `acquire` with explicit type-annotated args — the goal
  is to catch signature drift, not to be pedantic about how.
