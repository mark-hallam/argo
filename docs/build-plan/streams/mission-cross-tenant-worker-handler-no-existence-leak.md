# mission-cross-tenant-worker-handler-no-existence-leak

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `470cf830a` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-cross-tenant-worker-handler-no-existence-leak`.

🟡 **P2 ENTERPRISE-GRADE TENANCY HARDENING.** Source: evaluator round-4 (`docs/skycot/evals/scheduled/2026-05-22/cross-tenant-existence-leak-six-handlers.md`):

> "Six handlers distinguish 'exists in another tenant' (403) from 'doesn't exist at all' (404), letting a caller enumerate which worker_ids belong to other tenants. Not P0 (no row contents leak), but an identifier-enumeration tool that an attacker can use to size up the target organization's worker pool. Brief I'd queue: `mission-cross-tenant-worker-handler-no-existence-leak`."

Per operator memory `feedback_enterprise_grade_defaults.md`: "Enterprise-grade architecture; SMB-shaped v1 behind seams. Strict defaults are the target architecture." Distinguishable error codes for cross-tenant vs nonexistent worker_ids violates that default.

**The contrast that proves the right behavior exists in-codebase**: `crates/argo-cli/src/handlers/gateway/routes_workflows.rs:13-21` has an explicit invariant comment:

> "Authenticated callers without the required capability get a generic `403 Forbidden` so the response does NOT leak whether the `(tenant, vertical, name)` triple exists."

The 6 worker handlers do the opposite. Same problem, opposite solution, same codebase — inconsistency is the bug.

## Scope

Change the 6 worker handlers' response shape so the cross-tenant existence probe internally still runs (preserves telemetry + logging) but the user-facing HTTP response is **identical** whether the worker exists in another tenant or doesn't exist at all.

**Option chosen: A** (matches workflows handler convention) — return generic `404 Not Found` with body `{"error":"worker not found"}` in BOTH branches. Probe-and-discriminate code path is preserved internally for telemetry; user-facing response is identical.

### Handlers to fix (7 callsites across 6 files)

```
crates/argo-cli/src/handlers/gateway/routes_worker_activity.rs:65
crates/argo-cli/src/handlers/gateway/routes_worker_detail.rs:179
crates/argo-cli/src/handlers/gateway/routes_worker_overview.rs:85
crates/argo-cli/src/handlers/gateway/routes_worker_policies.rs:64
crates/argo-cli/src/handlers/gateway/routes_worker_policies.rs:128
crates/argo-cli/src/handlers/gateway/routes_worker_settings_helpers.rs:60
crates/argo-cli/src/handlers/gateway/routes_duties_helpers.rs:71
```

Each currently has the pattern:

```rust
match probe_cross_tenant_existence(store, tenant, worker_id).await {
    Ok(true)  => return forbidden(&format!("worker {worker_id} not in this tenant")),
    Ok(false) => return not_found(&format!("worker {worker_id} not found")),
    Err(e)    => return server_error(&format!("cross-tenant probe: {e}")),
}
```

Change to:

```rust
match probe_cross_tenant_existence(store, tenant, worker_id).await {
    // Both branches return identical 404 to prevent worker_id enumeration across tenants.
    // The probe still runs for telemetry — the result is logged but not surfaced in the response.
    // See routes_workflows.rs:13-21 for the canonical invariant comment.
    Ok(true)  => {
        tracing::debug!(target: "tenancy", %worker_id, "cross-tenant probe: exists in other tenant");
        return not_found("worker not found");
    }
    Ok(false) => return not_found("worker not found"),
    Err(e)    => return server_error(&format!("cross-tenant probe: {e}")),
}
```

Critical details:
- The error message must NOT include `{worker_id}` — that's another small leak (a 404 with the requested id confirms the request was parsed; a 404 with a static "worker not found" is identical for any input shape).
- The `tracing::debug!` line is the audit trail — internal logs still see the distinction; HTTP clients do not.

### Invariant comment replicated across all 6 files

Each fixed handler gets a 3-line comment above the `match` block:

```rust
// Cross-tenant existence is hidden from the HTTP response.
// See routes_workflows.rs:13-21 for the canonical invariant.
// Probe runs internally for telemetry; response is identical for both branches.
```

### Regression tests

One test per handler in the corresponding `_tests.rs` sibling file. Test name template:

```
fn cross_tenant_worker_<verb>_returns_same_404_as_unknown_worker
```

Each test:
1. Bootstraps tenant_a + tenant_b with a worker in tenant_a.
2. Calls the endpoint with tenant_b's cookie + tenant_a's worker_id → asserts response status + body.
3. Calls the same endpoint with tenant_b's cookie + a UUID that doesn't exist anywhere → asserts response status + body.
4. Asserts step (2) and step (3) returned **byte-identical** status + body.

The byte-identical assertion is what locks in the invariant; a 404-with-different-message regression slips past status-only assertion.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_worker_activity.rs` — narrowly scoped: line 65 + comment block.
- `crates/argo-cli/src/handlers/gateway/routes_worker_detail.rs` — narrowly scoped: line 179 + comment block.
- `crates/argo-cli/src/handlers/gateway/routes_worker_overview.rs` — narrowly scoped: line 85 + comment block.
- `crates/argo-cli/src/handlers/gateway/routes_worker_policies.rs` — narrowly scoped: lines 64 + 128 + comment block (2 callsites in same file).
- `crates/argo-cli/src/handlers/gateway/routes_worker_settings_helpers.rs` — narrowly scoped: line 60 + comment block.
- `crates/argo-cli/src/handlers/gateway/routes_duties_helpers.rs` — narrowly scoped: line 71 + comment block.
- The 6 sibling `_tests.rs` files (create them where they don't exist; each test ≤ 60 LOC).
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- The `probe_cross_tenant_existence` helper itself. It already does the right thing at the SQL layer — the bug is in the response layer.
- The workflow handler (`routes_workflows.rs`) — that's the reference. Don't touch the reference.
- Any data-store layer / migration / tenant primitives. Pure response-shape fix.

## Acceptance criteria

- [ ] All 7 callsites updated to return generic 404 with body `{"error":"worker not found"}` for both probe branches.
- [ ] No HTTP response includes `{worker_id}` substring in the error body for these 6 handlers.
- [ ] `tracing::debug!` line preserves the cross-tenant existence signal for internal log analysis.
- [ ] 6 new regression tests (one per handler) named `cross_tenant_worker_<verb>_returns_same_404_as_unknown_worker`, each asserting byte-identical status + body between the "exists-in-other-tenant" and "nonexistent" branches.
- [ ] Replicated invariant comment (3 lines) on each fixed handler pointing at `routes_workflows.rs:13-21`.
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.
- [ ] `bash scripts/workspace-clippy-fast-fail.sh` clean.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `mission-cross-tenant-worker-handler-no-existence-leak`
- PR title: `gateway(workers): hide cross-tenant existence in 6 handler responses (mission-cross-tenant-worker-handler-no-existence-leak)`

## Notes for the implementer

- ~150-220 LOC expected. Response-shape change ~30 LOC across 7 callsites; 6 regression tests at ~25 LOC each = ~150; comment blocks ~20.
- **Cite the evaluator round-4 report verbatim in the PR body** so the trail from "evaluator finds drift → conductor authors brief → worker fixes" stays visible. Report path: `docs/skycot/evals/scheduled/2026-05-22/cross-tenant-existence-leak-six-handlers.md`.
- **The byte-identical assertion is load-bearing.** If you just assert "both return 404", a regression like `not_found(&format!("worker {worker_id} not found"))` slips past. Use `assert_eq!(resp_a.bytes, resp_b.bytes)`.
- **Don't enlarge scope to other handlers**. The evaluator noted (🟢 INFO) that a broader inconsistency audit could find more offenders, but that's a separate auditor task. Stay tight to the 6 worker handlers identified.
- **The `probe_cross_tenant_existence` helper is the right primitive** — don't refactor it. The fix is purely at the call-site.
- This brief is the evaluator round-4 recommended fix. PR body should make that lineage explicit.
