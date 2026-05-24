# mission-document-executors-per-tenant-credentials

**Owner**: claude
**Priority**: P2 — document provider credential isolation
**Authored against**: origin/main `50d329dc2` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-document-executors-per-tenant-credentials`.

## Source

`eval-document-executor-cross-tenant-scoping` findings F-3 and F-4:

- `MsGraphClient::from_env()` reads one process-wide `ARGO_MSGRAPH_ACCESS_TOKEN`.
- `GsuiteAuthConfig::from_env()` reads one process-wide `ARGO_GSUITE_SERVICE_ACCOUNT_EMAIL` / `ARGO_GSUITE_SERVICE_ACCOUNT_KEY`.

Both are acceptable for a single-tenant gateway process, but they become credential bleed once a process serves multiple tenants.

## Goal

Add tenant-aware credential constructors for Microsoft Graph and Gsuite document executors, then thread those constructors through the CLI dispatch shims. Keep the old env constructors as explicit fallback/dev paths if needed, but production dispatch should prefer tenant-keyed credentials.

## May Touch

- `crates/argo-executor-document/src/msgraph.rs`
- `crates/argo-executor-document/src/msgraph_tests.rs`
- `crates/argo-executor-document/tests/eval_findings_cross_tenant.rs`
- `crates/argo-executor-document-gsuite/src/auth.rs`
- `crates/argo-executor-document-gsuite/src/document_gsuite.rs`
- `crates/argo-executor-document-gsuite/src/document_gsuite_tests.rs`
- `crates/argo-executor-document-gsuite/tests/eval_findings_cross_tenant.rs`
- `crates/argo-cli/src/handlers/worker_actions_executor_document.rs`
- `crates/argo-cli/src/handlers/worker_actions_executor_document_gsuite.rs`
- `crates/argo-config/src/cred/**` or `crates/argo-credentials/src/**` only for narrow tenant-keyed lookup helpers required by this brief.

## Must Not Touch

- Do not change Google or Microsoft API wire formats.
- Do not introduce networked tests; use mock servers and env/credential fixtures.
- Do not delete the old env constructors unless all existing call sites are migrated and tests prove the replacement.
- Do not add migrations unless an existing tenant-credential table is demonstrably absent and the PR body explains why.

## Acceptance

- [ ] Add tenant-aware constructors, e.g. `MsGraphClient::from_tenant_credentials(tenant_id, ...)` and `GsuiteAuthConfig::from_tenant_credentials(tenant_id, ...)`, using the repo's existing tenant credential store pattern.
- [ ] CLI document dispatch uses tenant-aware constructors when it has workspace tenant context.
- [ ] Same-tenant dispatch still succeeds with test credentials.
- [ ] Cross-tenant regression proves tenant A and tenant B resolve distinct provider credentials in one process.
- [ ] Env fallback remains isolated to tests/dev path and is documented as process-scoped.
- [ ] If F-3/F-4 FINDINGS-only tests exist on the worker base, un-ignore/invert or replace them with live passing tests.
- [ ] Every touched file remains under 500 LOC.
- [ ] `cargo clippy -p argo-executor-document -p argo-executor-document-gsuite -p argo-cli --all-targets -- -D warnings` is clean, or the PR body records unrelated clean-main drift.

## Notes

Do not invent a parallel credential abstraction. Follow the existing `argo-config` / `argo-credentials` tenant-keyed lookup shape already used elsewhere in the repo.
