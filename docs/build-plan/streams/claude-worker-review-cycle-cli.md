# V1.1 — Worker periodic review cycle CLI

**Owner**: claude. **Depends on**: claude-worker-kpi-tracker MERGED,
claude-worker-success-metrics-vis MERGED, operator_feedback table
(migration 24) present on main.

## Why

Mission claim 2: a worker has duties, KPIs, and a charter — and an
operator has to **review** them on a cadence. Today the operator must
manually pull KPI values, success rate, audit excerpts, and operator
feedback comments to do a performance review. There's no single
"give me a quarterly review packet" command.

## Scope

- CLI:
  - `argo worker review <id> [--period quarterly|monthly|weekly]
    [--since <iso-date>] [--format markdown|json]`
- Bundles into one report:
  - Session count, last session timestamp
  - Success rate (% Decision::Approve over total auto-approval calls)
  - KPI deltas: latest reading per KPI vs the value at start of period
  - Sample of 3 most-recent operator_feedback rows
  - Sample of 3 most-recent handoffs (sent + received)
  - Anomaly count (joins `worker_anomaly_event` if table exists —
    skip section gracefully if not)
- Markdown output is the primary surface (operator pastes into
  notebook); JSON is for programmatic dashboards.

## Approach

1. `crates/argo-cli/src/handlers/worker_review.rs` (NEW, ≤ 450 LOC)
   — pure read-side aggregator over existing tables. No new schema.
2. Period helper: quarterly = current quarter start; monthly =
   first-of-month; weekly = last 7d. `--since` overrides.
3. Markdown formatter mirrors session messages markdown family.
4. Soft-link to anomaly table — skip cleanly if not present.

## Files you may touch

- BLANKET CLI ALLOWLIST applies
- EXHAUSTIVE-MATCH amendment applies
- `crates/argo-cli/src/handlers/worker_review.rs` (NEW)
- `crates/argo-cli/src/cli_worker_cmds.rs` — `Review` variant
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 1 pub mod line

## Files you must NOT touch

- Migrations — no new schema
- store crate — read-only via existing query helpers (add a few
  internal queries inside `worker_review.rs` if needed, but no new
  store modules)
- TUI — out of scope

## Acceptance criteria

- [ ] `argo worker review <id> --period quarterly` returns markdown
      with all 6 sections (or graceful "No data" placeholders)
- [ ] `--format json` emits structured object with the same data
- [ ] Anomaly section skipped cleanly when table absent
- [ ] Empty worker (no sessions) returns review with all-zero
      counts, not an error
- [ ] 10 unit tests (period arithmetic, KPI delta calc, markdown
      sections) + 2 CLI smoke + 2 markdown snapshots
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-worker-review-cycle-cli`
- PR title: `worker: periodic review report aggregator CLI (claude-worker-review-cycle-cli)`

## Notes

- The first KPI reading in a period may not exist exactly at period
  start — use last reading **before** period start as the baseline,
  and "no baseline" if there's nothing prior.
- Markdown layout: H1 = worker name + period; H2 sections in order
  (Summary, KPIs, Success rate, Operator feedback, Handoffs,
  Anomalies). Keep each section terse — operator skim is the goal.
- Mirror the existing `argo session messages --markdown` heading
  style for consistency.
