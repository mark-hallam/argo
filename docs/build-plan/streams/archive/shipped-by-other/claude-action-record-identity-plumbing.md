# Claude — Thread worker_id through ActionRecord

**Owner**: claude. **Depends on**: E-identity (#781), H-2 (#800),
H-orchestrator-persist (#806) merged.

## Scope

The H-2 PR #800 left a TODO referenced by every dependent
slice: the dispatch path doesn't yet pass `worker_id` to the
auto-approval gate. As a result:
- `maybe_auto_approve` reads only the workspace policy.
- claude-worker-instance-policy can't actually resolve a
  per-worker policy override.
- The skill-learning loop (`learn::maybe_propose`) gets a
  `session_id: None` for every call site because no caller
  has plumbed it through.
- The e2e integration test #810 had to SKIP 3 steps for the
  same reason.

This brief threads `worker_id: WorkerId` through `ActionRecord`
+ every emit / dispatch / transition seam so all the
downstream consumers can stop using the "default-tenant /
default-persona" sentinels.

## Files you may touch
- `crates/argo-types/src/lib.rs` (or wherever `ActionRecord`
  lives) — add `worker_id: WorkerId` field with `serde(default)`
  for back-compat with persisted JSON
- `crates/argo-cli/src/handlers/worker_actions_emit.rs` —
  pass through at emit time
- `crates/argo-cli/src/handlers/worker_actions_executor*.rs`
  — read worker_id when building executor RunRequest /
  notify recipients
- `crates/argo-cli/src/handlers/worker_actions_auto_approval.rs`
  — pass worker_id to a new `load_resolved_policy` shim (the
  shim itself lands in claude-worker-instance-policy; this
  brief only adds the parameter passing)
- `crates/argo-cli/src/handlers/worker_actions_dispatch.rs` —
  thread through the existing call chain
- Any test fixtures that build ActionRecord by literal —
  update to include `worker_id: WorkerId::nil()` (a new
  helper)

## Files you must NOT touch
- `crates/argo-identity/src/types.rs` — `WorkerId` is fixed
- `crates/argo-orchestrator/**` — read-only consumer
- argo-channels

## Acceptance criteria
- [ ] `ActionRecord` has `worker_id: WorkerId` field
- [ ] `WorkerId::nil()` helper for tests + the v1 default
      worker (the platform-shipped Worker every workspace
      gets at first init)
- [ ] Migration extends `action` table with `worker_id`
      column (NULLable; backfill nil for existing rows)
- [ ] Every emit-side caller passes the worker's id from
      the calling context
- [ ] e2e integration test #810's 3 SKIPPED steps now
      execute (the test enables the now-plumbed
      assertions)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean
- [ ] Every modified file < 500 LOC

## Branch + PR
- Branch: `claude-action-record-identity-plumbing`
- PR title: `types: thread worker_id through ActionRecord (claude-action-record-identity-plumbing)`

## Notes
- This is foundation-laying — the brief that closes the gap
  between "data types exist" (E-identity) and "the data is
  used everywhere" (every H-stack consumer). Several other
  briefs (claude-worker-instance-policy, the e2e test) only
  finish their stories once this lands.
- Migration: `17__action_worker_id.sql` (or next available).
  ALTER TABLE action ADD COLUMN worker_id TEXT; backfill
  the platform-default worker id for existing rows.
- This brief is intentionally medium-sized — it's
  refactoring more than feature work. Resist the temptation
  to also fix the per-worker policy or the e2e SKIPs in
  the same PR; let those land separately on top.
