# mission-cu-macos-executor-tenant-preflight-enforce

**Owner**: claude.
**Depends on**: `mission-cu-macos-cross-tenant-capability-test` (#2063 MERGED-FINDINGS-ONLY).
**Authored against**: origin/main `7ce67c46d` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-cu-macos-executor-tenant-preflight-enforce`.

🟡 **P1 ENTERPRISE-GRADE TENANCY — macOS executor pre-flight is tenant-blind.** Source: PR #2063 findings (Tier-1 review verified). The cross-tenant capability test brief explicitly authored `#[ignore]`'d tests + a FINDINGS-ONLY PR-body note when the worker discovered:

> "`MacosCuRunRequest` carries no `tenant_id`/`worker_id`, `MacosCuExecutor::run`'s 'pre-flight' is `preflight::probe()` (macOS permissions only — Screen Recording / Accessibility / Input Monitoring / Automation), and `MacosCuError` has no `CapabilityDenied`/`TenantMismatch` variant. The per-call capability gate at the executor layer does not exist."

SQL primitives ARE tenant-scoped (verified by #2063's one running test). The gap is purely at the executor's runtime surface — worker_a in tenant_a could theoretically pass worker_b's UUID (from tenant_b) into the executor and the executor would have no way to refuse.

## Scope

Three coordinated additions to `argo-executor-computer-use-macos`:

### 1. Extend `MacosCuRunRequest` with tenant + worker identity

`crates/argo-executor-computer-use-macos/src/lib.rs` (or wherever `MacosCuRunRequest` lives — find via `rg 'pub struct MacosCuRunRequest'`):

```rust
pub struct MacosCuRunRequest {
    // existing fields preserved
    pub tenant_id: TenantId,
    pub worker_id: WorkerId,
}
```

Both fields are `pub` so the caller (gateway / agent runner) must populate them. Backward-compat: this is a breaking change to the struct. Sweep callers in `crates/argo-cli/` + `crates/argo-agent-runner/` and add the fields at every constructor site.

### 2. Add `IdentityStore` dependency to the executor

Pass `Arc<dyn IdentityStore>` (or whatever the canonical identity trait is — confirm) into `MacosCuExecutor::new()`. The pre-flight will use it to verify `tenant_id` owns `worker_id` before any macOS API call.

### 3. New `MacosCuError::TenantMismatch` variant + pre-flight check

```rust
pub enum MacosCuError {
    // existing variants preserved
    TenantMismatch { tenant_id: TenantId, worker_id: WorkerId },
    // …
}

impl MacosCuExecutor {
    async fn pre_flight_tenant_check(&self, req: &MacosCuRunRequest) -> Result<(), MacosCuError> {
        let worker = self.identity.find_worker(&req.worker_id, &req.tenant_id).await
            .map_err(|_| MacosCuError::TenantMismatch { /* ... */ })?;
        if worker.tenant_id != req.tenant_id {
            return Err(MacosCuError::TenantMismatch { /* ... */ });
        }
        Ok(())
    }
}
```

Call `pre_flight_tenant_check` from `MacosCuExecutor::run` BEFORE `preflight::probe()` — refuse the run before any macOS permission negotiation happens.

### 4. Drop `#[ignore]` on #2063's two tests + verify they pass

`crates/argo-executor-computer-use-macos/tests/cross_tenant_capability_test.rs` — the two tests (`cross_tenant_worker_id_returns_tenant_mismatch` + `same_tenant_worker_id_passes_pre_flight`) should now run cleanly. Update assertions if the variant name landed differently.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-executor-computer-use-macos/src/` — extend request struct + executor + error variant. Find the right files via `rg 'pub struct MacosCuRunRequest|pub enum MacosCuError|impl MacosCuExecutor' crates/argo-executor-computer-use-macos/src/`.
- `crates/argo-executor-computer-use-macos/Cargo.toml` — add `argo-identity` dep if missing.
- `crates/argo-executor-computer-use-macos/tests/cross_tenant_capability_test.rs` — drop `#[ignore]`'s.
- `crates/argo-cli/`, `crates/argo-agent-runner/` — sweep `MacosCuRunRequest::new()` callers to populate the new fields. Likely 2-4 call sites.
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- Other executor crates (E2B, Linux, Windows) — DEFERRED-v1.x per operator strategy.
- `argo-identity` — extend if needed via an additive method only; do not refactor.
- The macOS permissions probe itself (`preflight::probe()`) — that's separate from tenant pre-flight.

## Acceptance criteria

- [ ] `MacosCuRunRequest` has `tenant_id` + `worker_id` fields.
- [ ] `MacosCuExecutor::new()` takes an `Arc<dyn IdentityStore>` (or canonical equivalent).
- [ ] `MacosCuError::TenantMismatch` variant added.
- [ ] Pre-flight tenant check runs BEFORE the macOS-permissions probe.
- [ ] Both tests in `cross_tenant_capability_test.rs` no longer `#[ignore]`'d AND pass.
- [ ] All caller sites swept and populate the new request fields.
- [ ] `cargo clippy -p argo-executor-computer-use-macos --all-targets -- -D warnings` clean.
- [ ] `cargo clippy -p argo-cli -p argo-agent-runner --all-targets -- -D warnings` clean (caller sweep).
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `mission-cu-macos-executor-tenant-preflight-enforce`
- PR title: `feat(cu-macos): tenant-mismatch pre-flight enforcement + IdentityStore wiring (mission-cu-macos-executor-tenant-preflight-enforce)`

## Notes for the implementer

- ~250-400 LOC expected. Request struct + executor wiring ~80, error variant ~20, pre-flight check ~30, caller sweep ~50, test un-ignore + fixes ~30.
- **Pre-flight tenant check runs FIRST** — before any macOS API call. Don't even invoke `preflight::probe()` if the tenant doesn't own the worker.
- **The error message should NOT leak whether the worker exists in another tenant.** Return `TenantMismatch` with the request's tenant_id/worker_id, but the variant message should be the same shape whether the worker exists elsewhere or doesn't exist at all (matches the existence-leak fix shipped via mission-cross-tenant-worker-handler-no-existence-leak — same principle, different surface).
- **Caller-sweep is mechanical** but easy to miss a site. Run `rg 'MacosCuRunRequest \{' crates/` after the struct change — every site that constructs the struct needs the new fields, OR a default-via-context helper.
- This brief is the fix-pair to #2063's FINDINGS-ONLY ship.
