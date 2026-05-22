# Genesis quality checks — gate generated verticals against minimums

**Owner**: claude. **Depends on**: none (extends existing Genesis crate).

## Scope

Without quality gates, Genesis can emit a vertical that compiles but is
useless: 1-action vocabulary, 2-line system prompt, 0 eval seeds.
This brief adds quality gates that block emission of a vertical that
fails minimums:

- action vocabulary ≥ 3
- eval seed count ≥ 10
- memory schema layers ≥ 1
- system prompt ≥ 200 words

Returns a structured report so the CLI can surface "your vertical is
missing X, Y, Z" instead of bailing silently.

## Approach (concrete)

1. **NEW** `crates/argo-genesis/src/quality.rs` (~220 LOC):
   - `struct QualityReport { passed: bool, checks: Vec<CheckResult> }`
   - `struct CheckResult { name: String, passed: bool, observed: String, expected: String }`
   - `pub fn check_vertical(generated: &GeneratedVertical) -> QualityReport`
     — runs all 4 checks; returns a struct; never errors
   - `pub fn require_pass(report: &QualityReport) -> ArgoResult<()>`
     — converts a failing report into `ArgoError::Validation` with
     a multi-line message
2. **Extend** `lib.rs` to `pub mod quality;`
3. Wire into `pipeline.rs`: call `check_vertical` after generation, call
   `require_pass` before writing the vertical to disk (unless
   `--allow-low-quality` flag is set).

## Files you may touch

- `crates/argo-genesis/src/quality.rs` (NEW, ≤ 240 LOC)
- `crates/argo-genesis/src/quality_tests.rs` (NEW, ≤ 220 LOC) — 6 tests:
  - all 4 minimums met → passed = true
  - 2-action vocabulary → fails with action_vocabulary_min check
  - 9 eval seeds → fails with eval_seed_min check
  - 0 memory layers → fails with memory_layers_min check
  - 199-word prompt → fails with prompt_word_min check
  - require_pass(passing) → Ok(()), require_pass(failing) → Err(Validation)
- `crates/argo-genesis/src/lib.rs` (narrowly: 1 pub mod line)
- `crates/argo-genesis/src/pipeline.rs` (narrowly: 1 call site)

## Files you must NOT touch

- `crates/argo-cli/` — surfacing the report in CLI output is a follow-up
  brief; here we only return the structured value
- `crates/argo-vertical/` — schema unchanged

## Acceptance criteria

- [ ] check_vertical() runs without panic on any GeneratedVertical
- [ ] All 4 minimums enforced; thresholds are constants at top of file
- [ ] Report includes observed + expected for every failed check
- [ ] require_pass formats failures as a multi-line ArgoError::Validation message
- [ ] 6 unit tests pass
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-genesis` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-genesis-quality-checks`
- PR title: `genesis: quality gates for generated verticals (≥3 actions / ≥10 seeds / ≥1 layer / ≥200 words) (claude-genesis-quality-checks)`

## Notes for the implementer

- Word counter: split on whitespace, no fancy tokenisation. ≥ 200
  whitespace-separated tokens is the bar.
- Thresholds live as `const ACTION_MIN: usize = 3;` etc — easy to tune
  later without API churn.
- This brief deliberately does NOT introduce a "warning" tier yet —
  every check is pass/fail. Warnings can land in a follow-up.
