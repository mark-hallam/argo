# mission-document-executor-tenant-stamp

**Owner**: claude
**Priority**: P2 — claim 5 multi-tenant substrate hardening
**Authored against**: origin/main `50d329dc2` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-document-executor-tenant-stamp`.

## Source

`eval-document-executor-cross-tenant-scoping` (#2127) found that the public document executor surfaces are tenant-blind. The follow-up `eval-document-executor-cross-tenant-scoping-v2` PR body names this fix as `mission-document-executor-tenant-stamp`: land `tenant_id: TenantId` on the five pinned surfaces and replace the FINDINGS-only contract with live positive tests.

## Goal

Thread `TenantId` through the document executor public surfaces without changing document content semantics:

- `DocExecutor::run`
- `OutputPath::resolve`
- `MsGraphExecutor::create` / `MsGraphExecutor::read`
- `DocumentCsv::write`
- `DocumentGsuite::write`

This is a structural API hardening pass. It does not need to fully solve credential lookup or output-root isolation; sibling briefs own those.

## May Touch

- `crates/argo-executor-document/src/executor.rs`
- `crates/argo-executor-document/src/msgraph.rs`
- `crates/argo-executor-document/src/msgraph_tests.rs`
- `crates/argo-executor-document/tests/eval_findings_cross_tenant.rs`
- `crates/argo-executor-document-csv/src/document_csv.rs`
- `crates/argo-executor-document-csv/src/types.rs`
- `crates/argo-executor-document-csv/tests/eval_findings_cross_tenant.rs`
- `crates/argo-executor-document-gsuite/src/document_gsuite.rs`
- `crates/argo-executor-document-gsuite/src/types.rs`
- `crates/argo-executor-document-gsuite/tests/eval_findings_cross_tenant.rs`
- `crates/argo-cli/src/handlers/worker_actions_executor_document.rs`
- `crates/argo-cli/src/handlers/worker_actions_executor_document_gsuite.rs`
- New or existing tests under the same three executor crates that pin the tenant-bearing signatures.

## Must Not Touch

- Do not add migrations.
- Do not change real provider credentials in this brief.
- Do not alter PDF/CSV/Gsuite document payload semantics except adding tenant identity to request/report surfaces where needed.
- Do not edit unrelated gateway routes.

## Acceptance

- [ ] All five public surfaces above require or carry `argo_types::TenantId` in a way that compile-time tests can pin.
- [ ] CLI dispatch shims pass the workspace tenant id into document executor calls.
- [ ] Existing behavior for same-tenant document generation remains unchanged.
- [ ] Cross-tenant structural tests fail on the old tenant-blind shape and pass on the new tenant-bearing shape.
- [ ] If PR #2173's `surface_contract_pin_f1.rs` files have landed, retire or invert the tenant-blind LIVE pins in the same patch.
- [ ] Every touched source/test file remains under 500 LOC.
- [ ] `cargo clippy -p argo-executor-document -p argo-executor-document-csv -p argo-executor-document-gsuite -p argo-cli --all-targets -- -D warnings` is clean, or the PR body records an unrelated clean-main blocker.

## Notes

This is intentionally the API-stamp brief only. Per-tenant credential lookup and tenant-scoped output roots have separate briefs so the blast radius stays reviewable.
