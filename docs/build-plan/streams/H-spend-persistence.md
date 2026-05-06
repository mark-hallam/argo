# Stream H (follow-up) — Dispatch-time spend persistence

**Owner**: codex. **Depends on**: H-1 (#796) + H-2 (#800) merged.

## Scope

Wire the H-2 auto-approval gate to actually persist the
worker's per-action charge into the `worker_budget` table that
H-1 created. Currently the gate passes
`SpendSnapshot::zero()` to the evaluator — the daily-budget
cap path is therefore correct but always sees zero
accumulated spend, so the cap never triggers in practice.

This brief closes that loop:
1. **Before** evaluating, query today's accumulated spend
   from `worker_budget` for this worker and pass it as the
   `SpendSnapshot`.
2. **After** an `AutoApprove` decision, write a charge row
   `(action_id, worker_id, amount_usd_micros, created_at)`
   so the next action's pre-evaluation read sees it.

## Files you may touch
- `crates/argo-cli/src/handlers/worker_actions_auto_approval.rs`
  (extend — see §Acceptance for the seam to add)
- `crates/argo-cli/src/handlers/worker_actions_auto_approval_tests.rs`
  (extend with the new tests)
- `crates/argo-identity/src/worker_budget.rs` — narrowly
  scoped: add `spend_today_for_worker` query + `record_charge`
  insert helpers. Do NOT modify the existing
  `WorkerBudget` ledger struct or its rollup methods.

## Files you must NOT touch
- `crates/argo-identity/src/auto_approval.rs` — the H-1
  evaluator is already correct; only the inputs change.
- `crates/argo-cli/src/handlers/worker_actions_dispatch.rs` —
  the gate hookup site is fixed; this brief only changes
  what the gate's helper passes to the evaluator.
- Any new migration — the `worker_budget` table from H-1
  already has the schema we need. If it doesn't, FILE A
  BLOCKED reason on this brief in BUILD_STATUS rather than
  add a new migration here.

## Acceptance criteria
- [ ] `worker_budget::spend_today_for_worker(pool, worker_id,
      now)` returns the sum of `amount_usd_micros` charges
      with `created_at >= today_start_utc(now)` for that
      worker.
- [ ] `worker_budget::record_charge(pool, action_id,
      worker_id, amount_usd_micros, created_at)` inserts a
      row.
- [ ] `maybe_auto_approve` calls `spend_today_for_worker`
      before `evaluate`; passes the result as
      `SpendSnapshot { daily_micros: <sum> }`.
- [ ] On `Decision::AutoApprove`, after the existing
      `transitions::approve()` call, `record_charge` is
      invoked with the action's estimated cost from the
      manifest's `executor.budget_usd` (use 0 if unset).
- [ ] Tests:
      - over-the-cap test: charge an under-cap amount, then
        a second one that crosses the daily cap → second
        evaluates to `Block`
      - exactly-at-cap test: charge fills the cap; next
        action of cost 0 still auto-approves
      - cross-worker isolation: worker A's charges do NOT
        count toward worker B's budget
      - cross-day reset: a charge dated yesterday does NOT
        count toward today's spend
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean
- [ ] Every modified file stays < 500 LOC

## Branch + PR
- Branch: `codex-H-spend-persistence`
- PR title: `dispatch: persist auto-approval charges + read today's spend (H-spend-persistence)`

## Notes
- "Today" is UTC midnight to UTC midnight — the fixed-window
  daily cap is simpler than a rolling 24-hour window and
  matches what the H-1 brief said.
- The action's estimated cost comes from the
  `ActionDeclaration.executor.budget_usd` field (Stream A).
  Convert USD to micros via `* 1_000_000.0` and saturating
  cast to `u64`.
- The H-2 PR #800 left a TODO referencing this brief —
  search `argo-cli/src/handlers/worker_actions_auto_approval.rs`
  for it before starting.
