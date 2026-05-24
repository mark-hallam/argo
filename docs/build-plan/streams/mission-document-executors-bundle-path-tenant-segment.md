# mission-document-executors-bundle-path-tenant-segment

**Owner**: claude
**Priority**: P2 — claim 5 output isolation hardening
**Authored against**: origin/main `50d329dc2` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-document-executors-bundle-path-tenant-segment`.

## Source

`eval-document-executor-cross-tenant-scoping` finding F-6: the document dispatch shims build bundle paths as `cwd/.skycot/outputs/<job_id>/actions/<action_id>/` with no tenant segment. That is safe only under the current single-tenant-per-cwd assumption.

## Goal

Insert a tenant segment into document executor bundle paths so two tenants with colliding job/action ids cannot interleave output files under a shared gateway cwd.

Target shape:

`<cwd>/.skycot/outputs/<tenant_id>/<job_id>/actions/<action_id>/`

Retain backward-compatible tests for path escaping and result JSON, but update expected paths to include tenant identity.

## May Touch

- `crates/argo-cli/src/handlers/worker_actions_executor_document.rs`
- `crates/argo-cli/src/handlers/worker_actions_executor_document_gsuite.rs`
- `crates/argo-cli/src/handlers/worker_actions_executor_document_csv.rs`
- Existing or new tests adjacent to those dispatch shims.
- `crates/argo-executor-document/src/executor.rs` only for doc comments that describe the caller-owned bundle shape.
- `crates/argo-executor-document/tests/eval_findings_cross_tenant.rs` only to retire/invert the F-6 finding if present.

## Must Not Touch

- Do not change document payload schemas.
- Do not alter provider credential loading.
- Do not move non-document executor bundle paths.
- Do not add migrations.

## Acceptance

- [ ] `ensure_bundle_dir` for document creation, document CSV, and document Gsuite includes the workspace tenant id before `job_id`.
- [ ] Same-tenant execution writes to the expected tenant-prefixed bundle and reports that bundle in `ActionOutcome`.
- [ ] A regression test proves tenant A and tenant B with the same job/action ids resolve to different bundle directories.
- [ ] Existing path traversal protections remain intact.
- [ ] If F-6 FINDINGS-only tests exist on the worker base, un-ignore/invert or remove them with a clear comment naming this brief.
- [ ] Every touched file remains under 500 LOC.
- [ ] `cargo clippy -p argo-cli -p argo-executor-document --all-targets -- -D warnings` is clean, or the PR body records an unrelated clean-main blocker.

## Notes

Prefer a tiny shared helper over three divergent path builders, but only if the helper keeps file boundaries small and obvious.
