# claude-followup-pr-2021-test-run-gateway-persist

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `9faa2cd58` on `2026-05-22`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-followup-pr-2021-test-run-gateway-persist`.

Re-author of closed-as-stale PR #2021 (`claude-web-workflow-test-run-gateway-persist`) after the tick-38 rebase pass surfaced a migration-slot collision. Original intent — server-side `workflow_test_run` history persistence behind `POST /api/v1/verticals/<v>/workflows/<n>/test_runs` and `GET …/test_runs?limit=N` — is unchanged.

## Why a re-author (not a rebase)

The closed branch claimed migration slot **129 / v77**. Both slots are now occupied on `origin/main`:

- `crates/argo-store/migrations/129__workplace_knowledge.sql` (landed in admin-wave-6)
- `crates/argo-store/src/migrations/v77.rs`

Re-numbering 129→130 and v77→v78 in a stale 17-file branch with auto-merge conflicts on `crates/argo-audit/src/event.rs::AuditKind`, plus a drive-by `47__skill_canary.sql` edit that PR #2025 (`claude-migration-comment-aware-splitter`) already superseded, would be three layers of conflict resolution on a 1352-LOC PR. Cleaner to re-author against current main.

## Scope

Re-implement the closed PR's deliverable end-to-end:

- Persistence: new `workflow_test_run` table (slot **130 / v78**) with columns `(id, tenant_id, vertical_id, workflow_name, started_at, ended_at, status, summary_json, step_states_json, transport_error)`.
- Gateway routes:
  - `POST /api/v1/verticals/<v>/workflows/<n>/test_runs` — accepts a `CompletedTestRun` payload (same shape as `useTestRun.ts::CompletedTestRun`), persists, returns the row id.
  - `GET  /api/v1/verticals/<v>/workflows/<n>/test_runs?limit=N` — returns the most-recent N rows for the tenant + workflow.
- Audit: add `AuditKind::WorkflowTestRunPersisted` variant (additive enum variant; exhaustive-match allowance applies).
- Web client: `web/lib/api/test-runs.ts` posts each `useTestRun.onComplete` to the persist endpoint; loads history from the GET endpoint on builder mount.

Use slot **130 / v78** in this re-author. Verify against `ls crates/argo-store/migrations/ | sort -V | tail -3` at claim time.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-audit/src/event.rs` + `event_tests.rs` (new `AuditKind` variant only).
- `crates/argo-cli/src/handlers/gateway/mod.rs` + `routes_dispatch_get.rs` + `routes_dispatch_post.rs` (route registration).
- `crates/argo-cli/src/handlers/gateway/routes_workflows_test_runs.rs` + `_tests.rs` (new files).
- `crates/argo-store/migrations/130__workflow_test_run.sql` (new file, slot 130).
- `crates/argo-store/src/lib.rs` + `migrations/mod.rs` + `migrations/registry_compliance.rs` + `migrations/v78.rs` (new file, v78) + `workflow_test_run.rs` (new module).
- `web/lib/api/test-runs.ts` (new file).
- `web/app/(app)/workflows/[verticalId]/[name]/build/useTestRun.ts` + `.test.ts` (call-site wiring; stays under 500 LOC).

## Files you must NOT touch

- `crates/argo-store/migrations/47__skill_canary.sql` — leave at the post-#2025 state.
- Any other audit variant or migration slot.

## Acceptance criteria

- [ ] Migration runs cleanly on a `:memory:` SessionStore, idempotent on a second invocation.
- [ ] Tenant isolation enforced: the GET endpoint filters by `(tenant_id, vertical_id, workflow_name)`.
- [ ] `POST` happy-path + 400 (malformed) + 401 (missing token) + tenant-mismatch tests.
- [ ] `GET` returns `[]` for an unknown workflow (not 404).
- [ ] Every modified file < 500 LOC.
- [ ] `AuditKind::WorkflowTestRunPersisted` variant added with payload `{worker_id, vertical_id, workflow_name, status}`.
- [ ] Web client tests cover the optimistic-record-then-persist flow.

## Branch + PR

- Branch: `claude-followup-pr-2021-test-run-gateway-persist`
- PR title: `web/workflow — server-side test-run history (re-author of closed #2021, slot 130/v78)`

## Notes for the implementer

- ~700-900 LOC across ~10-12 files (the original was 1352 LOC for 17 files; some original churn was drive-by edits to `47__skill_canary.sql` and now-superseded `AuditKind` shuffling that this re-author drops).
- The original PR's test-runs persistence module (`workflow_test_run.rs`) is a reasonable starting template — read it from `git show 5d7141e26:crates/argo-store/src/workflow_test_run.rs` before writing from scratch.
- Source of original intent: closed PR #2021 PR body + commit `5d7141e2`.
