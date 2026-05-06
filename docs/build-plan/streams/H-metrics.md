# Stream H (follow-up) — Prometheus metrics for orchestrator + auto-approval

**Owner**: codex. **Depends on**: H-1 (#796) + H-2 (#800) +
H-3 (#799) merged. **Independent** of persist briefs.

## Scope

Wire counters + histograms into the existing Prometheus
`/metrics` endpoint so operators can dashboard the H-stack:

Counters:
- `argo_auto_approval_decisions_total{decision, reason}` —
  bumped on every gate evaluation. `decision` ∈
  `auto_approve | ask | block`. `reason` is a short label.
- `argo_session_created_total{tenant}` — when a Session is
  created.
- `argo_handoff_emitted_total{from_kind, to_kind}` — when a
  Handoff is created. `from_kind` always `worker`; `to_kind`
  ∈ `worker | human`.
- `argo_handoff_settled_total{outcome}` — `outcome` ∈
  `settled | expired`.

Histograms:
- `argo_auto_approval_evaluate_duration_seconds` — wall time
  of the evaluator + (post-H-spend-persistence) the spend
  query. Targets `<5ms` p99.

## Files you may touch
- `crates/argo-orchestrator/src/metrics.rs` (NEW)
- `crates/argo-orchestrator/src/lib.rs` — narrowly scoped:
  ONE `pub mod metrics;` line + ONE `pub use` line.
- `crates/argo-orchestrator/Cargo.toml` — add `prometheus`
  workspace dep (already in workspace).
- `crates/argo-cli/src/handlers/worker_actions_auto_approval_metrics.rs`
  (NEW)
- `crates/argo-cli/src/handlers/worker_actions_auto_approval.rs`
  — narrowly scoped: ONE call site that bumps the decision
  counter. Do NOT modify the policy logic.
- `crates/argo-cli/src/handlers/mod.rs` — ONE line.

## Files you must NOT touch
- Any existing metric definitions in argo-cli (don't risk
  collisions; new metrics live in new files).
- `crates/argo-identity/**` — pure data layer; no metrics.
- The `/metrics` route handler itself — the new counters
  register globally and the existing handler picks them up
  automatically.

## Acceptance criteria
- [ ] All 4 counters + 1 histogram register with the global
      Prometheus registry.
- [ ] Auto-approval gate bumps the decision counter on every
      evaluation, with the right labels.
- [ ] Session::new + Handoff::new instrumented (where the
      types are constructed in the orchestrator persist API
      — coordinate via the brief comment if persist hasn't
      landed; use a no-op `record_session_created()` helper
      that other code calls).
- [ ] Hitting the existing `/metrics` endpoint includes the
      new metric names + the same `# HELP` / `# TYPE` lines
      Prometheus expects.
- [ ] Test: bump each counter manually, scrape /metrics,
      assert the names appear.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean
- [ ] Every authored file < 500 LOC

## Branch + PR
- Branch: `codex-H-metrics`
- PR title: `metrics: orchestrator + auto-approval Prometheus counters (H-metrics)`

## Notes
- Use `lazy_static!` or `once_cell::sync::Lazy` to define
  the static metrics; the workspace already does this in
  the existing metrics.
- Histograms default buckets: `[0.001, 0.005, 0.01, 0.05,
  0.1]` — the gate is supposed to be sub-millisecond, the
  buckets bias toward fast.
