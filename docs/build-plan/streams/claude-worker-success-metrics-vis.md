# V1.1 — Per-worker success metrics from auto-approval outcomes

**Owner**: claude.

**Authored against**: origin/main 2026-05-09 (rewrite v2 by conductor — original v1 assumed an `auto_approval_decision` table that doesn't exist on main. v2 retargets to roll up over the existing `audit_record` table, filtering by typed `argo_audit::AuditKind` variants ApprovalGranted / ApprovalRevoked / ApprovalConsumed via worker-bound session_id). **Depends on**: auto-approval Decision recording
on main (search for `Decision::Approve` in
`worker_actions_auto_approval.rs`).

## Why

Mission claim 2: a worker has KPIs. The most fundamental KPI of all is
**did the operator (or the policy) approve what you proposed?** Today
each `Decision::Approve|Deny|Timeout|Defer` outcome is recorded but
there's no per-worker rollup that says "this AR clerk gets approved
93% of the time; the recruiter screener gets approved only 41%".

Without this, the promotion workflow + KPI tracker have no canonical
"success rate" to feed off — every other brief has to recompute.

## Scope

- Read-side aggregator only — no new schema. Reads from existing
  `audit_record (auto_approval_decision table doesn't exist; rollup is over typed audit-kind variants from argo-audit)` table (or whatever the canonical name is —
  inspect `worker_actions_auto_approval.rs` for the table).
- CLI:
  - `argo worker success <id> [--since <duration>] [--bucket none|day|
    week|month] [--format markdown|json]`
- Metrics emitted:
  - Total decisions, broken down by outcome (Approve/Deny/Timeout/Defer)
  - Approval rate = Approve / (Approve + Deny)
  - Deferred-rate = Defer / total
  - Timeout-rate = Timeout / total
  - Bucketed time series if `--bucket` not none
- Markdown table is the primary surface; JSON for dashboards.

## Approach

1. `crates/argo-cli/src/handlers/worker_success.rs` (NEW, ≤ 380 LOC)
   — pure query wrapper. Group-by SQL with `strftime` for buckets.
2. Markdown formatter mirrors `argo session messages --markdown`
   convention: H1 worker name + period, then table.
3. JSON shape stable for downstream consumers (review CLI, promotion
   workflow): `{ approve, deny, timeout, defer, approval_rate,
   buckets: [...] }`.

## Files you may touch

- `crates/argo-cli/src/handlers/worker_metrics.rs` (NEW) — read-only rollup over the `audit_record` table, aggregating ApprovalGranted / ApprovalRevoked / ApprovalConsumed kinds (typed `argo_audit::AuditKind` variants), filtered by worker-bound session_id. Query via `SessionStore`'s existing `sqlx::Pool<Sqlite>` directly — no argo-store schema changes.
- `crates/argo-cli/src/handlers/worker_metrics_tests.rs` (NEW sibling) — unit tests for the rollup logic.
- `crates/argo-cli/src/cli_worker_cmds.rs` — narrowly: new `WorkerCmd::Metrics` variant (or sibling enum if at-ceiling).
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 1 `pub mod` line.
- `crates/argo-cli/src/main.rs` — narrowly: dispatch arm.

## Files you must NOT touch

- `worker_actions_auto_approval.rs` — read-only consumer
- Any auto-approval write path — this brief NEVER mutates decisions
- Migrations — no new schema
- TUI — defer

## Acceptance criteria

- [ ] `argo worker success <id>` returns markdown with totals + rate
- [ ] `--bucket week` returns one row per ISO week (using
      `strftime('%Y-%W', decided_at)`) with per-bucket rates
- [ ] `--since 30d` filters decided_at >= now - 30d
- [ ] `--format json` emits stable schema (matches docstring)
- [ ] Worker with zero decisions → returns `approval_rate: null`,
      not divide-by-zero
- [ ] 10 unit tests (rate calc, empty case, bucket arithmetic,
      since filter) + 2 CLI smoke + 1 markdown snapshot
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-worker-success-metrics-vis`
- PR title: `worker: success-rate metrics from auto-approval outcomes (claude-worker-success-metrics-vis)`

## Notes

- **First step**: confirm the auto-approval decision table name +
  columns by `grep -rn 'CREATE TABLE.*approval' crates/argo-store/
  migrations/`. The aggregator must match the actual schema.
- This brief is intentionally thin (1 file + clap variant) — its
  whole job is to be the canonical "success rate" producer that
  promotion + review + KPI consume.
- Default `--bucket none` means single-row roll-up. Always include
  `since`-bounded total alongside any bucket series.
