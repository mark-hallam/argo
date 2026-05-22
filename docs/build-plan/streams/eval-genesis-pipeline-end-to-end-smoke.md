# eval-genesis-pipeline-end-to-end-smoke

**Owner**: claude (evaluator-shape brief — investigation + cargo integration test).
**Depends on**: none.
**Authored against**: origin/main `432aac842` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-genesis-pipeline-end-to-end-smoke`.

🟡 **P1 MISSION-CLAIM-3 (Platform builds workers).** Source: round-8 manifest Claim 3 row for `argo-genesis`.

> "argo-genesis — 8881 LOC. 20+ pub modules: archetype, charter_dialogue, conflict, eval_seed, genesis_judge, job_description, manifest_writer, memory_schema, pipeline, prompt_engineer, prompt_writer, quality, refine, safety_judge, templates, worker_config. Need verification each module is wired AND that the pipeline produces a valid vertical tree."

Claim 3 ("Genesis turns plain-English role → vertical") is mission-load-bearing. The crate is massive (8.8K LOC), 20+ pub modules — none verified end-to-end. We don't actually know if Genesis-generated verticals work.

## Scope

Write a cargo integration test that drives the Genesis pipeline end-to-end:

1. Given a plain-English role description: `"I need a Slack-based customer-support agent that triages incoming messages, escalates billing questions to the operator, and learns from approved responses."`
2. Invoke the full Genesis pipeline (find the entry point — likely `genesis::pipeline::run` or `Genesis::generate`).
3. **Assert the output tree**:
   - `vertical.toml` exists + parses via `argo-vertical::parse_workflow`.
   - `worker.toml` exists + parses.
   - `system_prompt.md` exists + non-empty.
   - `memory_schema.toml` exists + parses.
   - At least 1 eval seed in `evals/INDEX.json`.
4. **Assert quality gates fired**:
   - `safety_judge` was invoked and rejected ANY action with a sensitive verb (e.g. "delete", "transfer money") without `requires_approval = true`.
   - `genesis_judge` returned a quality score above the configured threshold.
5. **Negative test**: feed in a role string with a clearly-unsafe action (e.g. "auto-send refunds without approval") → assert `safety_judge` returns Reject + pipeline halts.

6. **Round-trip via argo-vertical loader**: write the generated tree to a tempdir + call `argo_vertical::load_dir(tempdir)` → assert no parse errors.

If the test reveals Genesis produces invalid output OR safety_judge doesn't fire OR the round-trip fails → ship `#[ignore]`'d + FINDINGS-ONLY note.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `NEW: crates/argo-genesis/tests/pipeline_end_to_end_test.rs` (~150-300 LOC integration test).
- `crates/argo-genesis/Cargo.toml` — narrowly scoped: add dev-deps if needed.
- `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md` — flip argo-genesis row.
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- `crates/argo-genesis/src/**` — read-only audit, NOT a refactor.
- `crates/argo-vertical/src/**` — read-only.
- The pipeline's LLM prompts (if any are file-based).

## Acceptance criteria

- [ ] End-to-end test from plain-English role string to validated vertical tree.
- [ ] Safety-judge negative test pinned.
- [ ] argo-vertical round-trip pinned.
- [ ] If pipeline produces invalid output, ship `#[ignore]`'d + FINDINGS-ONLY.
- [ ] Gap manifest updated.
- [ ] Every modified file <500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `eval-genesis-pipeline-end-to-end-smoke`
- PR title: `test(genesis): end-to-end pipeline smoke + safety-judge negative (eval-genesis-pipeline-end-to-end-smoke)`

## Notes for the implementer

- ~200-400 LOC expected. Pipeline test ~150, safety negative ~50, round-trip ~50.
- **The safety-judge negative is the load-bearing piece**. Without it, you've only verified Genesis CAN produce a tree, not that it produces SAFE trees.
- LLM prompts inside Genesis may make the test slow or non-deterministic. If so, mock the LLM client OR mark the test `#[ignore]` with a "requires ANTHROPIC_API_KEY" note + ship the mock-driven variant as the steady-state test.
- Cite round-8 manifest verbatim.
