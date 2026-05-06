# Stream H — Auto-approval + orchestrator + skill library

**Owner**: claude. **Depends on**: A, D-email, E-identity,
F-memory-layering all MERGED. This is the integration capstone.

## Scope

Three sub-deliverables, ship together because they're tightly
coupled:

1. **AutoApprovalPolicy** on the Worker: per-instance budget,
   time windows, risk-class allowances, wake-rules.
2. **argo-orchestrator**: multi-worker sessions, handoffs,
   shared memory, escalation paths.
3. **Skill library evolution**: two-tier (workspace +
   library), trust grades on provenance, learning loop wired
   to the existing `pending_review` pattern.

## Files you may touch
- `crates/argo-orchestrator/**` (NEW)
- `crates/argo-skill/**` (extend, do NOT replace)
- `crates/argo-cli/src/handlers/worker_actions_dispatch.rs`
  (auto-approval gate before BLE / channel notify)
- `crates/argo-identity/src/auto_approval.rs` (NEW within the
  E crate)
- `crates/argo-cli/src/handlers/skill_review.rs` (extend)

## Files you must NOT touch
- `Executor` trait
- `NotifyChannel` trait (lock from D-email)
- `argo-channels` impls
- Any connector

## Acceptance criteria
- [ ] `AutoApprovalPolicy` evaluator: takes an action +
      worker state + recent spend → `Decision::{AutoApprove,
      Ask(channel), Block}`
- [ ] Per-action gate: `worker_actions_dispatch::execute`
      consults the policy BEFORE notifying any channel; if
      auto-approves, writes a `approved_by = "auto:<policy>"`
      audit row
- [ ] Per-worker budget ledger (`worker_budget` table) with
      daily / monthly rollups
- [ ] `argo-orchestrator::Session` type — a thread of work
      that may involve N workers + N humans
- [ ] `Handoff { from, to, payload, expected_outcome,
      deadline }` persisted; receiving worker sees it on next
      tick
- [ ] `MemoryNs::shared_team(tenant)` accessible by all
      workers on the team; writes are stamped with author
- [ ] Escalation path declarative on the Role:
      `[support_agent, sales_agent, human:mark@acmeco.com]`;
      action that exhausts retry budget walks the chain
- [ ] Skill library two-tier: workspace (mutable, per-tenant)
      → library (read-mostly, cross-tenant via existing OCI
      marketplace). Lookups walk workspace → library.
- [ ] Trust grades on `Provenance`: `builtin | tenant |
      published(verified) | published(community) |
      learned(unreviewed)`; `list_skills_for_action` filters
      by min-trust per action's risk
- [ ] Skill learning: when ComputerUseExecutor (Stream B)
      completes a novel sequence with `success_metric >
      threshold`, runtime serializes the trajectory + Claude-
      distilled `SKILL.md` into the existing `skill_candidate`
      table for review
- [ ] Updated evals to cover the new gates and skill flow
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean
- [ ] Every file < 500 LOC

## Branch + PR
- Branch: `claude-H-auto-approval`
- PR title: `orchestrator: auto-approval + multi-worker handoffs + skill library evolution (H-auto-approval)`

## Notes
- This is the largest brief. Split into sub-PRs if it doesn't
  fit one (each sub-PR a coherent slice; update BUILD_STATUS
  to track them).
- The integration demo (the v1 success criterion in
  MISSION.md) lives downstream of this PR landing. After H
  merges, run the demo end-to-end and capture a recording.
