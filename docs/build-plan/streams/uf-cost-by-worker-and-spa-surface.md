# uf-cost-by-worker-and-spa-surface (UF Phase-A · A7)

**Owner**: claude.
**Depends on**: **A6** (`spend_ledger.worker_id`). **Coordinate:** sprint WS4 billing.
**Authored against**: origin/main `2264560a2` on `2026-06-05`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh uf-cost-by-worker-and-spa-surface`.
**BLOCKED until A6 merges** — `GROUP BY worker_id` needs A6's column; conductor flips this TODO when A6 lands.

## Why
The dollar-cost endpoints exist (`routes_costs_by_day.rs`, `routes_post_costs.rs` — verified present) but have **ZERO `web/` callers**, are workspace-wide (not tenant-scoped), and the turn loop never stamps `cost_usd` on the audit row (so `ActivityRow.tsx:109` only populates for forge/eval).

## Grep-before-write (verified 2026-06-05)
- `rg 'resolve_tenant_for_request' crates/argo-cli/src/handlers/gateway/` — tenant-scope the `/costs` reads with the keystone resolver (mirror the WS1-1b migrations).
- `web/lib/api/usage.ts` — the client template to copy for `cost.ts`.

## Files you may touch
- tenant-scope the `/costs/*` reads (`resolve_tenant_for_request`) + new `/api/v1/tenant/costs/by-worker` (`GROUP BY worker_id`, using A6's column).
- `web/lib/api/cost.ts` (copy `usage.ts`) + a dollar panel on `/billing` and a cost chip on worker-detail/scorecard (the cost-vs-value pairing).
- stamp `cost_usd` on the agent-turn audit row so `ActivityRow.tsx:109` populates for normal work (today only forge/eval do).

## Acceptance
The operator sees total $ spend, $ per teammate, and $ beside goal attainment; tenant A can NOT read tenant B's costs.

## On ship (same commit)
Update `docs/build-plan/FEATURE_MATRIX.md` (cost-by-worker row) + regen HTML (`python3 scripts/render-feature-matrix.py`).

## Effort
~1–2 briefs (M).
