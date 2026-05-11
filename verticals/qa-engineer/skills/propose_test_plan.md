---
name: propose_test_plan
description: Given a feature spec, draft a layered test plan (unit / integration / e2e / smoke) with per-layer responsibilities, an explicit dropped-coverage rationale, and an estimated test count per layer.
inputs:
  - feature_spec
  - existing_coverage_summary
  - cost_budget_hint
outputs:
  - test_plan
  - dropped_coverage_rationale
  - estimated_count
  - flags
  - confidence
---

# Skill: Propose Test Plan

Produce a layered test plan that surfaces both what gets covered AND
what does not. Operators get the "we ship 80%; here are the 20% we
explicitly dropped" tradeoff visible.

1. **Per-layer responsibilities** — for each layer, name what it owns:
   - **Unit** — pure-function logic, branch coverage, edge cases.
     Mocks at every IO seam. Sub-second per test.
   - **Integration** — boots one or two components together
     (handler + store, worker + queue) with real dependencies for
     the seam under test. Mock the rest.
   - **e2e** — full stack, real services, real fixtures. Sparse —
     1 happy-path + 1 failure-path per critical journey.
   - **Smoke** — boot-only / health-check / contract pact. Runs on
     every deploy; <30s total wall-clock.
2. **Estimated count per layer** — give a number per layer ("unit:
   ~14, integration: ~5, e2e: 2, smoke: 1"). Tie the count to the
   `cost_budget_hint` if provided; if no hint, default to
   user-visible-paths × 1 happy + 1 failure per layer.
3. **Dropped-coverage rationale** — explicit. For each cluster of
   behavior NOT covered, give the reason:
   - `cost_bounded` — would require a paid third-party sandbox.
   - `out_of_scope` — belongs to a sibling team / vertical.
   - `lower_value` — risk × frequency below the budget threshold.
   - `infeasible_deterministically` — true non-determinism (e.g.
     timing race in a third-party API contract) that cannot be
     mocked without invalidating the test.
   Naming the dropped clusters makes the tradeoff visible; silent
   drops are not acceptable.
4. **Determinism check** — confirm none of the proposed test layers
   require `sleep`, `rand` without seed, or live wall-clock. Flag
   any layer that does and route to a human.

Hard rules:

- Never propose only one layer — every plan covers at least unit +
  integration; e2e and smoke are added per the cost budget.
- Never silently drop coverage of a security-or-privacy-sensitive
  path. If you would drop a path that touches auth / PII / payment,
  flag `security_path_dropped` and route for human review.
- The plan is a draft. Test-owners write the syntax in the target
  framework; Cam describes the SHAPE.
