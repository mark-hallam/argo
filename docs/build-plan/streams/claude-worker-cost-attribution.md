# V1.1 — Per-worker cost attribution CLI

**Owner**: claude. **Depends on**: `claude-session-worker-id-schema`
MERGED (this is the **hard** dep — sessions/messages/actions need a
`worker_id` column for any of this to be queryable).

## Why

Workers are employees, and employees have a P&L. The operator needs
"this AR clerk cost $42.18 in tokens last week to drive 17 collection
sessions" to make hire/fire/promote decisions. Today every session is
attributed to a tenant but not a worker — token spend is a tenant-level
aggregate, useless for delegation economics.

Once `claude-session-worker-id-schema` lands, the worker stamp is
available; this brief is the read-side that turns the stamp into
operator-facing cost reports.

## Scope

- CLI:
  - `argo worker costs <id> [--since <duration>] [--breakdown
    none|outcome|action_kind|day|week] [--format markdown|json]`
- Cost dimensions:
  - Token cost (sum of input + output tokens × per-model rate)
  - Action cost (count × per-action-kind unit cost from existing
    `spend_baseline` table — migration 26)
  - Total = token + action
- Breakdown by outcome (Approve/Deny/Timeout/Defer) lets the operator
  spot "we spent $30 on Deferred sessions that produced nothing".
- Use existing per-model rate tables / constants — DO NOT introduce
  new pricing schema; this brief is read-only over what's already there.

## Approach

1. `crates/argo-cli/src/handlers/worker_costs.rs` (NEW, ≤ 450 LOC) —
   joins messages → sessions (now stamped with worker_id) and actions
   → worker_id. Aggregates cost per dimension.
2. Token cost calc reuses existing `argo budget` cost helpers — search
   for the per-model price table to avoid duplicating it. If no shared
   helper, write a `cost_for_message(msg)` private fn referencing the
   same table.
3. Markdown table per breakdown axis; JSON shape mirrors `argo budget`
   for consistency.

## Files you may touch

- BLANKET CLI ALLOWLIST applies
- EXHAUSTIVE-MATCH amendment applies
- `crates/argo-cli/src/handlers/worker_costs.rs` (NEW)
- `crates/argo-cli/src/cli_worker_cmds.rs` — `Costs` variant
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 1 pub mod line

## Files you must NOT touch

- Pricing tables / `spend_baseline` migration — read-only
- `argo budget` handlers — extend, don't reimplement; reuse cost
  helpers if exposed
- Migrations — depends on schema added by `claude-session-worker-id-schema`,
  no new schema in this brief
- TUI — defer

## Acceptance criteria

- [ ] `argo worker costs <id>` returns total token + action cost in
      markdown
- [ ] `--breakdown outcome` returns 4-row table (Approve/Deny/Timeout/
      Defer) with cost per outcome
- [ ] `--breakdown action_kind` groups by action's kind field
- [ ] `--breakdown day|week` time-buckets
- [ ] `--since 7d` filters
- [ ] Worker with zero sessions → returns `total: 0`, no error
- [ ] Cost numbers reproduce within ±$0.001 of `argo budget` total
      when summed across all workers in a tenant (sanity invariant)
- [ ] 10 unit tests + 2 CLI smoke + 1 markdown snapshot + 1 invariant
      test (sum-of-workers == budget total for tenant)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-worker-cost-attribution`
- PR title: `worker: per-worker cost attribution CLI (claude-worker-cost-attribution)`

## Notes

- **Hard sequencing**: this brief is BLOCKED on
  `claude-session-worker-id-schema` — verify the worker_id columns
  exist on session + message + action tables before starting. If the
  upstream PR re-shaped the column name, update queries here to match.
- The "sum-of-workers == budget total" invariant is the single most
  important test — it catches any join error that double-counts or
  loses sessions.
- Markdown table style mirrors `argo budget` so operator output looks
  cohesive across the cost surface.
