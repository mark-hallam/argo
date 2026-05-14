# claude-vertical-eval-seed-generation-test — Test Genesis emits eval seeds + fixtures (Claim 3 completeness)

**Owner**: claude. **Depends on**: nothing.
**Authored against**: origin/main `c928d8fe` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-vertical-eval-seed-generation-test`.

## Scope

Phase C audit found that Claim 3 (the platform builds workers) is ADEQUATE-covered by `argo-genesis/tests/marketing_intern.rs` for charter → vertical generation, but **eval seed + fixture generation is not separately tested**. MISSION.md §2 claim 3 explicitly says Genesis emits "eval seeds" — we need to prove that.

This brief adds a focused test that Genesis-from-charter produces a generated vertical with ≥5 eval fixtures + at least 1 honesty/safety seed.

LOC: ~120.

## Files you may touch

- NEW `crates/argo-genesis/tests/eval_seed_generation.rs` — integration test. Pipes a fixture charter through Genesis; asserts the generated vertical directory contains:
  - `eval/` directory with ≥5 fixture files
  - At least 1 eval fixture with a `honesty` or `safety` tag
  - Eval fixtures pass schema validation
- `crates/argo-genesis/tests/fixtures/eval_seed_test_charter.md` — fixture charter prose.

## Files you must NOT touch

- Genesis production code (this is a TEST brief).
- argo-vertical production code.
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] `cargo test --test eval_seed_generation -p argo-genesis` passes.
- [ ] Test is deterministic: uses a recorded/mocked LLM response so eval-seed count is stable.
- [ ] If Genesis is non-deterministic in eval-seed COUNT (e.g., model emits 5-7 depending on temperature), assert `>=5` not `==N`.
- [ ] Negative case: a charter that LACKS testable behaviors (intentionally vague) produces a Genesis WARNING (not a silent zero-eval generation).
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `claude-vertical-eval-seed-generation-test`
- PR title: `test(genesis): eval seed + fixture generation coverage (claude-vertical-eval-seed-generation-test)`

## Notes for the implementer

- Mission-claim alignment: claim 3 (platform builds workers) — completeness test for the eval-seed half of Genesis output.
- The existing `marketing_intern.rs` test covers the vertical-emission half. This brief is the eval-seed half.
- If Genesis uses live API calls in tests today: gate behind `ANTHROPIC_API_KEY` env or use a recorded transcript fixture. Don't make CI dependent on real provider calls.
- Drift-prevention: brief adds NEW test file only; no new pub types.
