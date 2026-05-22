# mission-cu-macos-cross-tenant-capability-test

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `6278d31c1` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-cu-macos-cross-tenant-capability-test`.

🟡 **P2 ENTERPRISE-GRADE TENANCY — computer-use cross-tenant guard not proven.** Source: evaluator round-4 (`docs/skycot/evals/scheduled/2026-05-22/computer-use-macos-executor-unit-tests.md`):

> "Run the cross-tenant negative version (does worker_a from tenant_a get capability to drive worker_b from tenant_b? Should refuse.) — not a separate test file I can find; worth a brief."

Context: round 4 evaluated `argo-executor-computer-use-macos` and confirmed 37/37 unit tests pass for preflight / run-loop / screenshot / events / a11y. **But none assert cross-tenant.** Mission claim 5 (real-world execution) needs the macOS computer-use executor to be tenant-scoped at the capability layer — worker_a in tenant_a should not be able to spawn a computer-use action targeting tenant_b's namespace.

The SQL primitives are tenant-scoped (`worker.tenant_id`, `audit_record.tenant_id`). The question this brief tests: **does the capability check in the executor's pre-flight refuse a tenant-mismatched worker_id**?

## Scope

Add a tokio test (or async unit test if the executor crate already has a test harness) that:

1. Bootstraps two tenants in an `:memory:` SessionStore: `tenant_a` with worker_a, `tenant_b` with worker_b.
2. Constructs a `MacosCuExecutor` instance with tenant_a's principal context.
3. Calls the executor's `run_step` (or whatever the canonical entry point is) with `worker_id = worker_b` (from tenant_b).
4. Asserts the executor returns a typed error indicating cross-tenant refusal — most likely `MacosCuError::CapabilityDenied(...)` or `MacosCuError::TenantMismatch(...)`. If no such variant exists yet, that's a finding — DO NOT add the variant in this brief (it's out of scope); shipping `#[ignore]`'d test + finding is correct.
5. **Paired positive test**: same setup, but with `worker_id = worker_a` (same tenant) — assert the executor proceeds past the pre-flight (it may then fail on screen-recording permission or whatever — that's fine, the assertion is about the pre-flight gate, not the action completing).

### Where the test lives

`crates/argo-executor-computer-use-macos/tests/cross_tenant_capability_test.rs` — new integration test file. The crate already has tests under `crates/argo-executor-computer-use-macos/src/`; an integration test is the right home.

### What this test catches

- Regression: someone refactors the executor's pre-flight to look up the worker by ID without checking tenant_id.
- Regression: someone adds a new entry point to the executor that bypasses the pre-flight.
- Defines a contract: the macOS computer-use executor MUST refuse cross-tenant worker_ids.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `NEW: crates/argo-executor-computer-use-macos/tests/cross_tenant_capability_test.rs` (~80-120 LOC integration test).
- `crates/argo-executor-computer-use-macos/Cargo.toml` — narrowly scoped: add `[dev-dependencies]` if missing (`argo-store`, `argo-identity`, `tokio` with `macros` + `rt` features).
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- The executor's pre-flight logic — this brief is a TEST, not a refactor. If the test reveals the pre-flight is missing tenant check, ship the test as `#[ignore]`'d with a FINDINGS-ONLY note.
- `argo-runtime` capability layer — out of scope.
- Other executor crates (E2B, Linux, Windows) — those are DEFERRED-v1.x per operator strategy.

## Acceptance criteria

- [ ] Test file exists at the spec'd path.
- [ ] Negative test asserts cross-tenant worker_id is refused at pre-flight (typed error, not silent ok).
- [ ] Positive test asserts same-tenant worker_id passes the pre-flight gate.
- [ ] Test passes with `cargo test -p argo-executor-computer-use-macos --test cross_tenant_capability_test`.
- [ ] If the test reveals there is NO tenant check in pre-flight → STOP, ship `#[ignore]`'d test + FINDINGS-ONLY PR-body note. Conductor authors a fix-brief.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `mission-cu-macos-cross-tenant-capability-test`
- PR title: `test(cu-macos): cross-tenant capability refusal at pre-flight (mission-cu-macos-cross-tenant-capability-test)`

## Notes for the implementer

- ~120-180 LOC expected. Integration test ~100, Cargo.toml dev-deps ~5, docs ~0.
- **If the test exposes a missing tenant check, that's a P1 finding.** The macOS computer-use executor is the v1 demo path. A cross-tenant bypass at the executor layer would be a real bug. FINDINGS-ONLY ship + fix-brief follows.
- **Use the existing executor test harness pattern.** Other tests in the crate already construct executor instances; clone that setup.
- **The positive-test assertion is "passes pre-flight"**, not "action succeeds". Don't fight the screen-recording permission in CI — the assertion is about the gate, not the outcome.
- Cite the evaluator report verbatim in the PR body.
