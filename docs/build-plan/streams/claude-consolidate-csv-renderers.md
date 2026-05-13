# claude-consolidate-csv-renderers — 5 parallel CSV renderers → 1 canonical

**Owner**: claude. **Depends on**: nothing.
**Authored against**: origin/main `6e3e31d8` on `2026-05-14`.

> Audit cluster 4 found 5 parallel `render_csv` impls writing RFC-4180-ish output: `compliance_soc2.rs:263`, `approval_export.rs:179`, `audit_export.rs:108`, `argo-executor-document/src/csv.rs:22`, `argo-executor-document-csv/src/document_csv.rs`. None share code.

## Scope

Pick `argo-executor-document-csv::DocumentCsv` (the canonical typed-schema impl) as the single source of truth. The 4 ad-hoc handler-level renderers replace their bodies with a `argo_executor_document_csv::render_rfc4180(rows, &writer)` call.

## Files you may touch
- `crates/argo-cli/src/handlers/{compliance_soc2,approval_export,audit_export}.rs` — narrowly: ≤40 LOC each.
- `crates/argo-executor-document/src/csv.rs` — narrowly: delegate to argo-executor-document-csv OR delete this 22-LOC duplicate.
- `crates/argo-executor-document-csv/src/document_csv.rs` — narrowly: expose `render_rfc4180` helper if it doesn't already.

## Acceptance
- [ ] One canonical CSV writer; 4 handler-level renderers thin wrappers.
- [ ] All existing CSV test fixtures pass byte-identical (operator-visible no-change).
- [ ] Workspace clippy clean.
- [ ] Net LOC -200+.

## Branch
- `claude-consolidate-csv-renderers`
