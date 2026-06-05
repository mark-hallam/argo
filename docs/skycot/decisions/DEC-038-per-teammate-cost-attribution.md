---
id: DEC-038
date: 2026-06-05
status: active
supersedes: none
---

# DEC-038 — Per-teammate cost attribution (worker_id on spend)

## Decision

Operator approved (2026-06-05) adding **per-teammate cost**: attribute model
spend to the worker that incurred it, so the product can show cost-per-teammate
(and cost-vs-value beside the goal scorecard). **Additive-only** schema change.

## Why

Verified 2026-06-05: spend is real (`spend_ledger`, tokens × model price) but
keyed only by `tenant_id` + `session_id` — there is **no `worker_id`**
(`crates/argo-store/src/migrations/v2.rs:34`) — so cost-per-teammate is
structurally impossible. And the dollar data is invisible in the app (zero
callers of `/costs/*` in `web/`; the billing page shows an action-count vs cap,
not dollars). "Hire a teammate, see what it did and what it cost" requires
per-teammate cost.

## How to apply

- **Additive migration**: `session.worker_id` (nullable, if not already present)
  + `spend_ledger.worker_id` (nullable). Backfill null; never rewrite history
  (additive-migrations invariant).
- **Thread** `worker_id` into `SpendCtx` (`argo-agent-runner/src/helpers.rs:127`,
  built at `turn_run_loop.rs:132`) → `record_turn_spend` → `record_spend`.
- New **tenant-scoped** `/api/v1/tenant/costs/by-worker`; SPA cost surface
  (coordinate with sprint WS4 billing so the two don't collide).
- **Reconcile the stale `claude-session-worker-id-schema` brief first** (it was
  authored but has no ledger row — verify whether `session.worker_id` already
  exists before adding it).
- Briefs: `uf-spend-worker-id-attribution`, `uf-cost-by-worker-and-spa-surface`.

Relates: [[DEC-034]] (paying-ready — cost transparency is part of paying-ready),
sprint WS4 (billing). Phase A of the usefulness plan.
