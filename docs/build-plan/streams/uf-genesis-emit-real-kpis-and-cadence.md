# uf-genesis-emit-real-kpis-and-cadence (UF Phase-A · A2)

**Owner**: claude.
**Depends on**: **A1** (`uf-genesis-capture-cadence-exceptions-success` — needs the `success_criteria` + `cadence` slots). tenant keystone (#2676, MERGED).
**Authored against**: origin/main `2264560a2` on `2026-06-05`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh uf-genesis-emit-real-kpis-and-cadence`.
**BLOCKED until A1 merges** — consumes A1's `success_criteria` field; conductor flips this TODO when A1 lands.

## Why
The materializer emits a hardcoded SLA skeleton for EVERY teammate. Instead, emit the user's captured success-criteria (from A1) as real `[[kpi]]` rows in the generated `vertical.toml`, and carry cadence for duty creation — so dashboards score against the teammate's ACTUAL goals.

## Grep-before-write (the manifest already supports this — verified 2026-06-05)
- `KpiDefinition` at `crates/argo-vertical/src/manifest.rs:228`; unified read `all_kpis()` at `:324`. Reuse — do NOT add a parallel KPI type.

## Files you may touch
- `crates/argo-genesis/src/materialize.rs`: replace the constant `SuccessMetricsSection` (used ~:283) with a map from A1's `success_criteria` → `Vec<KpiDefinition>` (built near :262); default `gate=Warn` so dashboards score against them; keep a sensible default ONLY when the user gave no criteria.

## Acceptance
Two teammates built from DIFFERENT success-criteria produce DIFFERENT KPI targets in their `vertical.toml`; `all_kpis()` reads them back; manifest round-trip (`argo_vertical::load_dir`) still validates.

## On ship (same commit)
Update `docs/build-plan/FEATURE_MATRIX.md` (genesis KPI-emit row) + regen HTML (`python3 scripts/render-feature-matrix.py`).

## Effort
~1 brief (S–M).
