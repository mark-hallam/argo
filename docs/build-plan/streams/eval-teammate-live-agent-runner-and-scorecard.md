# eval-teammate-live-agent-runner-and-scorecard

**Priority**: P2 — post-MVP quality instrument (build after the demo MP4 records, per DEC-017). Completes phase-1 of the teammate-performance eval harness: the live-agent runner that produces `ScenarioOutcome`s + the scorecard aggregation + regression gate. The scoring backbone (`argo_eval::teammate`) + the first golden company already shipped (commit `195ff2ccf`).
**Source**: 2026-05-29 item-6 design (DEC-017). Design doc: `docs/skycot/evals/TEAMMATE_PERFORMANCE_EVAL_HARNESS_DESIGN.md`.
**Authored against**: origin/main `195ff2ccf` on 2026-05-29
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-teammate-live-agent-runner-and-scorecard`
**Authored by**: claude conductor (interactive session)

## What already exists (do not rebuild)

- `crates/argo-eval/src/teammate.rs` — `TeammateScenario`, `ScenarioOutcome`, `AxisScores`, `score_scenario()` (six behavioral axes, AND pass-gate). 9 unit tests green.
- `eval/companies/northwind-marketplace/` — first golden company (context.jsonl + README).
- `eval/scenarios/mariah/marketplace_core.jsonl` — 6-scenario suite exercising every axis.
- `argo-eval` already exposes `EvalSubject` (binds to an `Agent`), `regression` (gate primitives), `honesty`, and depends on `argo-capability` (boundary) + `argo-forge::cost` is available in-workspace.

## Scope

### 1. `scorecard.rs` (NEW module in `argo-eval`)
- `TeammateScorecard { teammate, company, per_scenario: Vec<(name, AxisScores)>, axis_means: AxisScores, overall: f64, pass_rate: f64 }`.
- `fn build_scorecard(records) -> TeammateScorecard` — aggregate per-axis means + overall + pass-rate (pass = `AxisScores::passed(threshold)`).
- `fn scorecard_regression(baseline, candidate, tolerance) -> RegressionDecision` — reuse/mirror the existing `regression.rs` pattern (per-axis + overall drop tolerance; escalation/boundary regressions are hard-fail, not tolerance-softened).
- Unit tests (pure, deterministic — feed hand-built records).

### 2. `bin/teammate-eval.rs` (NEW)
- `argo forge bench --teammate <name> [--company <name>] [--baseline <path>]` (or a standalone bin mirroring `bin/pitch-eval.rs`).
- Loads the company context + the scenario suite (JSONL).
- Binds the teammate's real `Agent` (its charter + skills from the pack/worker.toml) as the `EvalSubject`, runs each scenario, and **captures the behavioral signals** into `ScenarioOutcome` (escalated?, skills_used, honest?, elapsed_ms, cost via `argo_forge::cost`). This is the load-bearing new work — wiring the live agent + observing its behavior, not just its string output.
- Scores via `score_scenario`, builds the scorecard, prints it (human + `--json`), and applies the regression gate against a committed baseline.
- Runs against an **isolated eval tenant** seeded from `eval/companies/` — NEVER the demo tenant.

### 3. Baselines
- Commit `eval/baselines/portia.json` (Portia = reference teammate; wrap/relate to the existing 141-case pitch-eval) and `eval/baselines/mariah.json` (first synthetic-company teammate).
- A CI hook (mirror `scripts/check-pitch-eval.sh`) fails on regression beyond tolerance.

## Acceptance

- [ ] `scorecard.rs` aggregates per-axis means + overall + pass-rate; regression gate hard-fails escalation/boundary drops. Unit tests green.
- [ ] `teammate-eval` bin runs Mariah's `marketplace_core` suite against the Northwind company on an isolated eval tenant, drives the real Agent, captures behavioral signals into `ScenarioOutcome`, prints a scorecard (+ `--json`).
- [ ] Portia path: the bin can score Portia (reuse the existing 141-case investor-Q&A as her scenario source, or adapt) and emit her scorecard.
- [ ] Committed baselines for portia + mariah; CI regression hook wired.
- [ ] `cargo clippy -p argo-eval --all-targets -- -D warnings` clean; `cargo test -p argo-eval` green incl. the surface_contract pin updated for the new `scorecard` exports.
- [ ] No `unwrap`/`expect` outside `#[cfg(test)]`; every new pub item has rustdoc with an example (crate denies/warns these).

## Out of scope

- Historical-replay scenario source (DEC-017 later seam — build the `ScenarioSource` trait so it's pluggable, but do not implement replay).
- More golden companies / teammates beyond Mariah + Portia (phase-2 breadth).
- Operator-facing "report card" UI (phase-3).
- Multi-turn scenarios (phase-3 depth) — phase-1 scenarios are single-situation.

## May-touch

- `crates/argo-eval/src/scorecard.rs` (NEW) + `lib.rs` (export) + `tests/surface_contract.rs` (register new exports)
- `crates/argo-eval/src/bin/teammate-eval.rs` (NEW)
- `crates/argo-eval/Cargo.toml` (add `argo-forge` dep if needed for cost; add bin target)
- `eval/baselines/*.json` (NEW committed baselines)
- `scripts/check-teammate-eval.sh` (NEW CI hook, mirror check-pitch-eval.sh)

## Operator authority

Rust + new bin + model calls — NOT eligible for the web-only fast lane. Requires evaluator cold-cargo PASS before merge. Cost note: the live runner makes real model calls; route via the `claude-code-cli` provider (DEC-012) in dev to control cost, Sonnet floor (DEC-013).
