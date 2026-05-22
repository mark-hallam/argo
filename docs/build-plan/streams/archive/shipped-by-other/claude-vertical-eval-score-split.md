# claude-vertical-eval-score-split — split eval_score.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Phase A architectural prep — defensive split.**

**Authored against**: origin/main 7368d7aa on 2026-05-09 (conductor-authored after observing eval_score.rs at 499/500 LOC).

## Why

`crates/argo-vertical/src/eval_score.rs` is at 499 LOC on current main — 1 LOC of headroom against the 500-LOC HARD RULE 6 ceiling. Phase B eval-pipeline briefs (skill-test-execution-engine extensions, vertical-eval improvements, eval-rubric updates) hit this ceiling immediately. Preemptive split prevents future ceiling-refusals.

Pure refactor; no behaviour change. Mirrors the proven split pattern.

## Approach (concrete)

1. Read `eval_score.rs` end-to-end. Identify a self-contained section.
2. Common candidates: per-predicate scorers (Contains/Equals/NonEmpty etc. each as helpers), aggregation helpers, threshold logic, report-formatting.
3. Move that section to `crates/argo-vertical/src/eval_score_<topic>.rs`.
4. Re-export via `pub(crate) use`.

Target after split: `eval_score.rs` ≤ 350 LOC.

## Files you may touch

- `crates/argo-vertical/src/eval_score.rs` — narrowly: extract sibling, re-export
- `crates/argo-vertical/src/eval_score_<topic>.rs` (NEW, ≤ 250 LOC)
- `crates/argo-vertical/src/lib.rs` — narrowly: 1 `mod` declaration if needed

## Files you must NOT touch

- Other vertical-related modules (`vertical.rs`, `manifest.rs`, etc.)
- Any test file other than to verify no behaviour change

## Acceptance criteria

- [ ] `eval_score.rs` ≤ 350 LOC after the split (≥ 150 LOC headroom)
- [ ] Extracted sibling ≤ 250 LOC
- [ ] `cargo clippy -p argo-vertical --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-vertical` clean (no behaviour change)
- [ ] No public API changes

## Branch + PR

- Branch: `claude-vertical-eval-score-split`
- PR title: `vertical: split eval_score.rs below 500-LOC ceiling (claude-vertical-eval-score-split)`

## Notes for the implementer

- Pattern-match prior splits.
- Predicate-scorers naturally cluster into a sibling.

## Unblocks

- Future eval-pipeline extensions that would otherwise hit the ceiling.
