# Mission load-bearing wire — Per-vertical regression test harness

**Owner**: claude. **Depends on**: none.
**Authored against**: origin/main `392baa27` on `2026-05-13`.

> SOURCE: MISSION_BACKLOG #139. The swarm has shipped ≥20 verticals (customer-support / sales-sdr / marketing-intern / finance-ar / healthcare-intake / recruiter / tax-prep / product-manager / real-estate-agent / customer-onboarding / data-entry-clerk / event-coordinator / fundraising-coordinator / hr-screener / legal-paralegal / marketplace-seller / medical-coder / ops-on-call / procurement-assistant / qa-engineer / receptionist / ai-receptionist / executive-assistant / pr-checker / lead-research / bookkeeping-assistant). Each one has eval fixtures (`eval/seed.jsonl`) but no per-vertical regression test that runs the eval suite against the vertical's actual manifest + verifies the expected actions emit. Add it. **Tier-vertical-infra.**

## Scope

NEW test module `crates/argo-vertical-regression/` (or extend existing `argo-eval` if it's the right home — verify before creating). For each vertical:

1. **Load the vertical manifest** (`verticals/<id>/vertical.toml`) via existing `argo_vertical::manifest::load_vertical()`.
2. **Read every fixture** in `verticals/<id>/eval/seed.jsonl`. Skip fixtures with `expected_executor: "human"` or `requires_e2b: true` (those are smoke-test territory, not regression).
3. **For each fixture**: instantiate the vertical's role + send the fixture's `input` to a stub agent that records what `action_type` strings would be emitted (no actual LLM call — use the manifest's `[actions]` table to validate the fixture's `expected_action_type` is a declared action).
4. **Assert** that every fixture's `expected_action_type` matches a declared action in the manifest's `[actions]` vocab. This catches stale fixtures + actions removed from manifests.
5. **Assert** required-approval consistency: if a fixture says `expected_approval: required`, the manifest's `[escalation.human_approval_required]` must include the action_type; if `expected_approval: auto`, it must NOT.

NOT in scope: live LLM calls; live E2B; full integration tests (those exist via `argo eval run`); per-fixture-output verification.

## Files you may touch

- `crates/argo-vertical-regression/` (NEW crate, ≤ 500 LOC total) OR add to `crates/argo-eval/src/regression_per_vertical.rs` if argo-eval is the right home — worker decides based on existing conventions
- `crates/argo-vertical-regression/tests/regression_<vertical>.rs` if one-file-per-vertical (≤ 100 LOC each) OR a single parameterized `tests/regression_all.rs` enumerating verticals via filesystem walk
- `Cargo.toml` workspace `members`
- `docs/skycot/RUNBOOK.md` — append a "Per-vertical regression" subsection (≤ 30 LOC)

## Files you must NOT touch

- Any `verticals/<id>/` content — read-only inputs
- Any production runtime code — this is a test crate
- Existing `argo-eval` test files (extend additively only)

## Acceptance criteria

- [ ] Regression test runs for every vertical discovered by filesystem walk under `verticals/`
- [ ] Asserts every fixture's `expected_action_type` matches a declared `[actions]` entry
- [ ] Asserts approval-consistency per fixture
- [ ] `cargo test -p argo-vertical-regression --all-targets` (or `--test regression_all`) passes
- [ ] Adding a new vertical with a typo in `expected_action_type` causes the test to fail (verify by transient corruption + revert)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] All touched files < 500 LOC; BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-per-vertical-regression-test`
- PR title: `verticals: per-vertical regression test (mission-per-vertical-regression-test)`

## Notes

- Eval fixtures live at `verticals/<id>/eval/seed.jsonl` per the convention codified by mission-vertical-eval-suite-expand (#1485).
- Some verticals (e.g. `_starter-library`) are skill-libraries, not role-verticals; their manifest has `id` patterns like `starter-library` (no role). Skip these via the manifest's missing `[role]` table.
- This brief is a guard against drift, not a generator of test data. The test bodies are mechanical assertions; the heavy lifting is the filesystem walk + manifest load + fixture parse.
