# uf-scorecard-goal-attainment-surface (UF Phase-A · A5)

**Owner**: claude.
**Depends on**: **A2** (KPI targets exist); tenant keystone (#2676, MERGED). **Coordinate:** sprint WS2 empty-states.
**Authored against**: origin/main `2264560a2` on `2026-06-05`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh uf-scorecard-goal-attainment-surface`.
**BLOCKED until A2 merges** — needs the emitted KPI targets to score against; conductor flips this TODO when A2 lands.

## Why
Dashboards measure generic activity and DROP every domain/goal metric (`kpi_table.rs:62` returns `NotComputableV1`; `routes_worker_overview_kpis.rs:196`). Add **goal-vs-actual**: for KPIs with a countable audit source, emit `target` + `attainment_pct` + `status`; add a scorecard surface.

## Grep-before-write (reuse — verified 2026-06-05)
- `compute_kpis` at `routes_worker_overview_kpis.rs:186`.
- reuse `KpiSparkline` (#1435, `web/components/skycot/worker/`) + the scoring concepts in `crates/argo-eval/src/teammate_scorecard.rs`. Do NOT reinvent.
- `rg 'resolve_tenant_for_request'` — the scorecard route MUST tenant-scope via the keystone resolver (mirror the WS1-1b migrations).

## Files you may touch
- `routes_worker_overview_kpis.rs:186` `compute_kpis`: join manifest `all_kpis()` targets with audit actuals; stop returning `None` for sourced metrics.
- new `routes_worker_scorecard.rs` + a `scorecard` arm at `routes_dispatch_get_api_groups.rs:225` (grep the dispatcher first).
- `web/lib/api/scorecard.ts` (copy the `web/lib/api/usage.ts` template) + a scorecard surface on worker-detail + a team "goals hit this week" rollup.
- **NO THEATER:** goals with no countable source yet render "target set — measurement not wired", NEVER a fabricated number.

## Acceptance
A teammate with a "respond < 60s" target + audit rows at 45s renders "met / >100%"; a missed target renders "at risk"; the scorecard route is **tenant-scoped** (A can't read B's scorecard).

## On ship (same commit)
Update `docs/build-plan/FEATURE_MATRIX.md` (scorecard goal-attainment row) + regen HTML (`python3 scripts/render-feature-matrix.py`).

## Effort
~2 briefs (M–L) — split into the backend compute + the SPA surface if it nears the 500-LOC ceiling.
