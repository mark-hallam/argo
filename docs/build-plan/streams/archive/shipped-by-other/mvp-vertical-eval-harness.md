# mvp-vertical-eval-harness — `argo eval validate` runner over vertical eval seeds

**Owner**: claude. **Depends on**: nothing (relies on existing `argo-vertical` + `argo-judge` primitives). **Tier 1 MVP — closes mission-claim "verticals come with eval coverage".**

**Authored against**: origin/main 7fdbbdb8 on 2026-05-10.

## Why

Each vertical has `eval/seeds.jsonl` (or similar) but there is no single command to run them and emit a pass-rate metric. v1.0 needs a reproducible eval gate so vertical authors can verify their seeds pass before shipping.

## Approach (concrete)

1. **CLI**: NEW top-level `Eval` enum in `cli_command_enum.rs` (or split sibling `cli_eval_cmds.rs` if needed for LOC):
   - `argo eval validate <vertical-name-or-path> [--json]`
   - Loads the vertical via `argo-vertical::registry::Registry::load_dir`
   - Reads each `eval/*.jsonl` seed file
   - For each seed: dispatches the seeded input through the vertical's system_prompt + judge (use existing `argo-judge::Judge` trait + an env-configured backend; default to a deterministic-mock backend so the harness is testable without LLM credentials)
   - Tallies pass/fail counts; emits per-seed verdicts and an overall pass rate
2. **Metric**: emit a Prometheus-style counter at `vertical_eval_pass_rate{vertical="<name>"}` via existing `gateway/metrics.rs` IF gateway is up (best-effort; never blocks the eval result). Print to stdout always.
3. **Tests**: 1 unit test in `argo-cli/src/handlers/eval.rs` (parses seeds, scores deterministically). 1 integration smoke `tests/eval_validate_smoke.rs` runs `argo eval validate finance-ar` against the bundled seeds and asserts a parseable JSON envelope with `pass_rate` field.

## Files you may touch

- `crates/argo-cli/src/cli_command_enum.rs` OR `crates/argo-cli/src/cli_eval_cmds.rs` (NEW, ≤ 100 LOC) — Eval enum + Validate variant
- `crates/argo-cli/src/handlers/eval.rs` (NEW, ≤ 350 LOC) — runner + judge dispatch
- `crates/argo-cli/src/handlers/eval_tests.rs` (NEW, ≤ 200 LOC) — unit tests
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 1 `mod` declaration
- `crates/argo-cli/src/handlers/gateway/metrics.rs` — narrowly: register the counter (if gateway path needs registration; otherwise plumb at first emit)
- `crates/argo-cli/tests/eval_validate_smoke.rs` (NEW, ≤ 200 LOC)

## Files you must NOT touch

- Any vertical's `eval/seeds.jsonl` (those are the inputs, do not mutate)
- `argo-judge` crate internals — only call the public trait
- Any other handler file

## Acceptance criteria

- [ ] `argo eval validate finance-ar --json` emits `{vertical, total, passed, failed, pass_rate, seeds: [...]}`
- [ ] Deterministic-mock judge produces the same output across runs
- [ ] Smoke test runs and asserts envelope shape
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] No new files exceed their LOC cap

## Branch + PR

- Branch: `mvp-vertical-eval-harness`
- PR title: `cli: argo eval validate + pass-rate metric (mvp-vertical-eval-harness)`
