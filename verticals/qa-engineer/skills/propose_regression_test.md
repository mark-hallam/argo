---
name: propose_regression_test
description: Given a bug report and an executable reproduction, propose a deterministic regression test (language-agnostic shape) with an assertion checklist and setup/cleanup notes.
inputs:
  - bug_report
  - reproduction_steps
  - target_system
outputs:
  - proposed_test
  - assertion_checklist
  - setup_cleanup_notes
  - flags
  - confidence
---

# Skill: Propose Regression Test

Given a confirmed bug + reproduction, propose a regression test the
test-owner can implement in whatever framework the target codebase
uses. Cam describes the SHAPE; the framework syntax is the owner's
call.

1. **Proposed test** — name + layer (unit / integration / e2e /
   smoke) + the components under test + the mocks / fakes / fixtures
   substituted at the seams. State which seam isolates the bug from
   the rest of the system; if no seam isolates it cleanly, say so and
   flag the test as `layer_uncertain`.
2. **Assertion checklist** — numbered list of the post-condition
   assertions that must hold after the repro runs. Each assertion
   names the observable (return value / log line / persisted row /
   rendered DOM / emitted metric). Avoid "no exception thrown" as the
   sole assertion — name a positive observable.
3. **Setup / cleanup notes** — what state must exist before the test
   runs (fixture rows / config flags / feature toggles) and what
   must be cleaned up after. If the test mutates shared state, flag
   it `shared_state_mutation` and route — Cam does not author
   shared-state tests autonomously.
4. **Determinism check** — confirm the proposed shape has no
   `sleep`, no `rand` without seed, no wall-clock dependency, no
   live-network call. If any cannot be removed, flag the test and
   route for human triage.

Hard rules:

- If the reproduction references a fixture tagged `pii=true` in
  `test_data_inventory`, STOP. Do not draft the test. Surface the
  fixture row + escalate to the data-owner.
- If the reproduction touches pricing, billing, or revenue logic,
  flag `pricing_or_billing_touched` and route to finance review
  before the test lands.
- Never propose a test that includes a `sleep(n)` or `wait(n)`.
  Polling with a bounded retry against an observable condition is
  acceptable; bare wall-clock pauses are not.
- The proposed test is a draft — the test-owner reviews the shape
  before authoring the syntax in the target codebase.
