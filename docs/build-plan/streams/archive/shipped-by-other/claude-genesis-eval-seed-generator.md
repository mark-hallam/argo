# Genesis eval seed generator — 10–20 sample tasks + expected outputs

**Owner**: claude. **Depends on**: none (extends existing Genesis crate).

## Scope

Every vertical needs eval seeds (sample tasks the worker should be able
to handle, with rough expected outputs) so the eval runner has
something to grade against. Today operators hand-author these. This
brief generates 10–20 starter eval seeds from the role description.

Mirrors mission claim 3 (eval seeds generation).

## Approach (concrete)

1. **NEW** `crates/argo-genesis/src/eval_seed.rs` (~300 LOC):
   - `struct EvalSeed { id: String, task: String, expected_summary: String, difficulty: Difficulty, rubric_keywords: Vec<String> }`
   - `enum Difficulty { Easy, Medium, Hard }`
   - `pub fn generate(role_description: &str, actions: &[SuggestedAction]) -> ArgoResult<Vec<EvalSeed>>`
     — emits ≥ 10 and ≤ 20 seeds, ~40% Easy / ~40% Medium / ~20% Hard
   - `pub fn to_toml(seeds: &[EvalSeed]) -> String`
   - Generation strategy: ask the LLM for sample tasks given role +
     action list; post-filter to ensure each seed mentions at least
     one suggested action OR the role's primary verb
2. **Extend** `lib.rs` to `pub mod eval_seed;`
3. Wire into `pipeline.rs` so the seed file lands at
   `verticals/<name>/eval-seed.toml`

## Files you may touch

- `crates/argo-genesis/src/eval_seed.rs` (NEW, ≤ 320 LOC)
- `crates/argo-genesis/src/eval_seed_tests.rs` (NEW, ≤ 230 LOC) — 6 tests:
  - generate() returns 10–20 seeds for any of the demo role descriptions
  - difficulty distribution is roughly 40/40/20 (±10%)
  - every seed has a non-empty rubric_keywords
  - duplicate ids do not appear
  - to_toml() round-trips through `toml::from_str`
  - empty actions slice still works (uses role description only)
- `crates/argo-genesis/src/lib.rs` (narrowly: 1 pub mod line)
- `crates/argo-genesis/src/pipeline.rs` (narrowly: 1 call site)

## Files you must NOT touch

- `crates/argo-eval/` — eval runner consumes the TOML; no API change here
- `verticals/<demo>/eval-seed.toml` — leave the existing 5 demo seeds alone

## Acceptance criteria

- [ ] generate() always returns 10 ≤ N ≤ 20 seeds
- [ ] Each seed has id, task, expected_summary, difficulty, rubric_keywords (all non-empty)
- [ ] No duplicate ids within a single generate() call
- [ ] to_toml() round-trips via `toml::from_str`
- [ ] 6 unit tests pass
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-genesis` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-genesis-eval-seed-generator`
- PR title: `genesis: generate 10–20 starter eval seeds from role description (claude-genesis-eval-seed-generator)`

## Notes for the implementer

- Use `argo-llm` (workspace dep) for generation; cap at one model
  call per generate() invocation for cost control.
- Rubric keywords drive `argo-judge` matching — keep them lowercase,
  no punctuation, ≤ 5 per seed.
- IDs are stable hashes of the task text so re-running generate()
  produces the same id for the same task (helps git diffs).
