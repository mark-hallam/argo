# Claude — Per-worker auto-approval policy override

**Owner**: claude. **Depends on**: H-2 (#800), E-identity
(#781) merged.

## Scope

Today the auto-approval gate reads ONE workspace-level
policy from `.skycot/auto_approval.toml`. Every worker on
the workspace shares the same caps + windows + wake rules.

For a real customer deploy this is too coarse — a marketing
intern worker should be able to AutoApprove draft posts but
the same workspace's billing-followup worker probably
shouldn't AutoApprove anything money-touching.

This brief lets each Worker carry its own
`AutoApprovalPolicy` override. Resolution order:

1. Worker's `auto_approval` field (set at create time or via
   `argo worker update --auto-approval-json ...`)
2. Workspace `.skycot/auto_approval.toml`
3. Disabled default (Ask everything, Block on no-channel)

## Files you may touch
- `crates/argo-identity/src/types.rs` — `Worker` already
  has `auto_approval: AutoApprovalPolicy` (per E-identity
  + H-1). Add a query helper `Worker::resolved_policy(
  workspace_default: &AutoApprovalPolicy)` that picks
  between worker's vs workspace.
- `crates/argo-cli/src/handlers/worker_actions_auto_approval.rs`
  — narrowly: change `load_workspace_policy` callsite to
  `load_resolved_policy(action, workspace_dir, identity_store)`
  that fetches the worker for this action and resolves.
- `crates/argo-cli/src/handlers/worker_resolved_policy.rs`
  (NEW — the lookup helper)
- `crates/argo-cli/src/cli_worker_cmds.rs` — narrowly: add
  `argo worker update --auto-approval-json <path>` flag
- `crates/argo-cli/src/handlers/worker.rs` (or wherever
  worker create/update lives) — narrowly: handle the new
  flag

## Files you must NOT touch
- The H-1 evaluator (`auto_approval.rs`) — its API is fixed
- The H-2 gate orchestration — only the policy-loading
  shim changes
- argo-channels

## Acceptance criteria
- [ ] `Worker::resolved_policy(workspace_default)` returns
      the worker's policy if `enabled`, else falls back to
      the workspace default
- [ ] `argo worker update --auto-approval-json <file>`
      reads a JSON `AutoApprovalPolicy` and writes it to
      the worker's row
- [ ] Gate hookup uses the resolved policy at evaluation
      time; passes the worker_id through so the gate can
      look up its policy
- [ ] Migration is NOT required (the worker table already
      has the `auto_approval_json` column from E-identity
      / H-1)
- [ ] Tests:
      - worker with enabled policy + workspace disabled →
        worker policy used
      - worker disabled + workspace enabled → workspace used
      - both disabled → Ask/Block fallback (existing H-1
        behaviour)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean
- [ ] Every authored file < 500 LOC

## Branch + PR
- Branch: `claude-worker-instance-policy`
- PR title: `identity: per-worker auto-approval policy override (claude-worker-instance-policy)`

## Notes
- Resolution rule for "worker enabled?" — the policy struct
  already has `enabled: bool`. Empty / default policy is
  treated as "not configured" → fall through.
- The dispatch path needs the `worker_id` to do this
  lookup. Stream-E threading already plumbed it via
  `ActionRecord.worker_id` (per H-2 PR's TODO comment) —
  read the actual current `worker_actions_auto_approval.rs`
  to see the threading state.
