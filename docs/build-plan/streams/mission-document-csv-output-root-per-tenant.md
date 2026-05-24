# mission-document-csv-output-root-per-tenant

**Owner**: claude
**Priority**: P2 — CSV output isolation
**Authored against**: origin/main `50d329dc2` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-document-csv-output-root-per-tenant`.

## Source

`eval-document-executor-cross-tenant-scoping` finding F-5: `crates/argo-executor-document-csv/src/sandbox.rs` has `DEFAULT_OUTPUT_ROOT = "~/.argo/exports/csv/"` with no tenant segment. The sandbox prevents traversal outside the root, but not co-tenant collisions inside the root.

## Goal

Add tenant-scoped CSV output resolution so `DocumentCsv::write` writes beneath a tenant-specific subdirectory by default.

Expected shape:

`<expanded output_root>/<tenant_id>/<requested-relative-path>`

Absolute paths that already point under the tenant-scoped root may remain accepted if the existing API supports that, but absolute paths under another tenant's subdir must fail.

## May Touch

- `crates/argo-executor-document-csv/src/sandbox.rs`
- `crates/argo-executor-document-csv/src/document_csv.rs`
- `crates/argo-executor-document-csv/src/types.rs`
- `crates/argo-executor-document-csv/src/document_csv_tests.rs`
- `crates/argo-executor-document-csv/tests/eval_findings_cross_tenant.rs`
- `crates/argo-cli/src/handlers/worker_actions_executor_document_csv.rs`
- `crates/argo-config/src/sections_executor.rs` only if the config docs/default string must clarify tenant interpolation without changing config shape.

## Must Not Touch

- Do not change CSV rendering semantics or RFC 4180 output.
- Do not add a migration.
- Do not touch Gsuite/MsGraph credentials.
- Do not remove traversal or symlink-escape checks.

## Acceptance

- [ ] `DocumentCsv::write` or its request/config path carries tenant identity to the sandbox resolver.
- [ ] The resolver creates or targets a tenant-specific subdirectory under the configured root.
- [ ] Same-tenant writes still succeed for normal relative paths.
- [ ] Cross-tenant collision regression: tenant A and tenant B using the same relative path resolve to different absolute paths.
- [ ] Escape regression: a path that reaches another tenant's subdir is rejected.
- [ ] If F-5 FINDINGS-only tests exist on the worker base, un-ignore/invert or replace them with live passing tests.
- [ ] Every touched file remains under 500 LOC.
- [ ] `cargo test -p argo-executor-document-csv` and `cargo clippy -p argo-executor-document-csv --all-targets -- -D warnings` are clean, or the PR body records unrelated clean-main drift.

## Notes

Keep the operator-configured root intact; this brief adds an automatic tenant segment under it, not a new global config system.
