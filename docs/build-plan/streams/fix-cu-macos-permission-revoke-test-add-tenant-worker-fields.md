# fix-cu-macos-permission-revoke-test-add-tenant-worker-fields

**Owner**: claude.
**Depends on**: nothing.
**Authored against**: origin/main `bb251a25d` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-cu-macos-permission-revoke-test-add-tenant-worker-fields`.

🔴 **P0 — workspace clippy is broken on main; pre-merge gate FAILS; no PR can admin-merge until this lands.**

## Why this is P0

Workspace clippy fails on `bb251a25d` (tick-96 pre-merge gate, 2026-05-23 06:55Z):

```
error[E0063]: missing fields `tenant_id` and `worker_id` in initializer of `argo_executor_computer_use_macos::MacosCuRunRequest`
   --> crates/argo-executor-computer-use-macos/tests/permission_revoke_mid_run_test.rs:139:5
```

PR #2074 (2026-05-22) landed the test as-authored. PR #2076 (2026-05-22, same wave) tightened `MacosCuRunRequest` with two new required fields (`tenant_id: TenantId` + `worker_id: WorkerId`) for the tenant pre-flight gate, but did not update the sibling test in the same crate. Crate-scoped `cargo test -p argo-executor-computer-use-macos` happens to skip this test file on macOS where it's `#[cfg(target_os = "macos")]`-gated indirectly — but workspace-wide `cargo clippy --all-targets` sees it and fails.

Result: every conductor pre-merge gate run blocks the wave. Two CLEAN read-only audit PRs (#2107 + #2108) sat unmergeable through tick-95 and tick-96 because of this drift.

## Scope

Add the two missing fields to the test's `run_request` builder. Use test-fixture identities (e.g. `TenantId::from("tenant-test")` + `WorkerId::from("worker-permission-revoke")`).

## Exact change

`crates/argo-executor-computer-use-macos/tests/permission_revoke_mid_run_test.rs` lines 138-145:

```rust
fn run_request(audit_dir: std::path::PathBuf) -> MacosCuRunRequest {
    MacosCuRunRequest {
        tenant_id: TenantId::from("tenant-permission-revoke-test"),
        worker_id: WorkerId::from("worker-permission-revoke-test"),
        action_id: "act-permission-revoke".into(),
        system_prompt: "Type two notes; the second is doomed.".into(),
        audit_dir,
        anthropic_api_key: "test-key".into(),
    }
}
```

You may need to import `TenantId` + `WorkerId` from `argo_types` (or wherever they are re-exported in `argo-executor-computer-use-macos`). Check `crates/argo-executor-computer-use-macos/tests/tenant_preflight_test.rs` for the existing import pattern.

## Files you may touch

- `crates/argo-executor-computer-use-macos/tests/permission_revoke_mid_run_test.rs` (the only required edit).

## Files you must NOT touch

- `crates/argo-executor-computer-use-macos/src/**` — the struct shape is correct; only the test is out of date.
- Any other test file.

## Acceptance

1. `cargo clippy -p argo-executor-computer-use-macos --all-targets -- -D warnings` — clean.
2. `bash scripts/conductor-pre-merge-gate.sh` — exits 0 (workspace clippy passes).
3. `cargo test -p argo-executor-computer-use-macos --test permission_revoke_mid_run_test` — passes (no behavior change; the existing assertions remain valid with the additional fields).

## Notes

- The fix MUST preserve the test's existing identity (it pins a real audit-chain-integrity invariant from round-7 evaluator finding); only the constructor arguments change.
- If the tenant pre-flight gate trips on the placeholder identities in the test (it shouldn't — these tests use `PermissionRevokeRunner`, not the live executor), defer to a sibling `tenant_preflight_test.rs` adjustment in a follow-up brief — don't expand scope here.
