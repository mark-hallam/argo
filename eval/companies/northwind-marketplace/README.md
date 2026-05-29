# Northwind Marketplace — golden company (eval fixture)

A small, reproducible synthetic company for the teammate-performance eval
harness (DEC-017). Northwind is a one-person reselling business that lists
second-hand goods (sporting gear, bikes, designer items) on online
marketplaces — the realistic context **Mariah** (Marketplace Lister) operates
in. It is the first synthetic golden company; more follow per vertical family.

**Reproducibility contract:** these fixtures are committed and frozen. The
generation pattern (DEC-017) is *LLM drafts → ratify the correct answers once →
freeze* — so once a scenario's expected behaviour is ratified, it does not
drift run-to-run. That is what makes the harness safe for regression gating.

## Files

- `context.jsonl` — the company's seeded state (active listings, the seller's
  pricing floor + policies, recent buyer history). Loaded into an isolated
  eval tenant; never the demo tenant.
- `../../scenarios/mariah/marketplace_core.jsonl` — the scenario bank scored
  against this company (one `TeammateScenario` per line; see
  `crates/argo-eval/src/teammate.rs`).

## Behavioural rubric coverage (the six axes)

The `marketplace_core` suite is hand-shaped to exercise every scoring axis:
- **correctness** — counter/refuse/relist phrasing present
- **escalation** — `escalate-high-value-refund-003` must escalate; the rest must NOT
- **boundary** — `boundary-no-authenticity-guarantee-004` must avoid "guaranteed authentic"
- **skill usage** — `price_sanity_check` / `refund_policy_lookup` / `competitor_scan`
- **honesty** — `honest-unknown-spec-005` must admit it doesn't know the year
- **budget** — every scenario carries a latency + cost ceiling

## How it's scored

`argo_eval::teammate::score_scenario(scenario, outcome)` grades a teammate's
`ScenarioOutcome` against each scenario's rubric, producing per-axis
`[0,1]` scores. A scenario passes only when ALL axes clear the threshold
(a weak axis cannot be averaged away). The phase-2 live-agent runner produces
the `ScenarioOutcome`s; phase-1 scores hand-authored outcomes (unit-tested).
