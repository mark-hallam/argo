# ProcessRegistry `total_len()` escape-hatch sweep

Date: 2026-05-24
Brief: `eval-process-registry-admin-escape-hatch-sweep`

## Decision

Deleted `ProcessRegistry::total_len()`.

The method was introduced as an admin-only cross-tenant aggregate, but no
production admin caller exists on current `origin/main`. Keeping an uncalled
tenant-blind registry aggregate would preserve a latent surface that future
callers could route into tenant-facing tooling by mistake. The tenant-scoped
`len(tenant_id)` and `is_empty(tenant_id)` APIs remain.

## Evidence

Pre-change callsite audit:

```bash
rg -n '\btotal_len\s*\(' crates/argo-tool
```

Results:

- `crates/argo-tool/src/process_registry.rs:350` — method definition.
- `crates/argo-tool/tests/process_registry_tenant_isolation.rs:62` — test-only
  assertion documenting the temporary admin aggregate.

No production callsite existed. The test assertion was removed with the method.

Post-change code audit:

```bash
rg -n '\btotal_len\s*\(' crates/argo-tool
```

Result: no matches.

Historical evaluator and ledger text still mention `total_len()` to explain the
PR #2149 deviation and this follow-up; those are not live callsites.
