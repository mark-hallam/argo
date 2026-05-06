# Claude — Genesis LLM-driven refinement pass

**Owner**: claude. **Depends on**: G-genesis (#790) merged.

## Scope

The G-genesis PR shipped a fully deterministic offline
charter→vertical generator with 12 archetypes + keyword
classifier. The PR body explicitly noted:

> The brief mentions LLM-driven refinement passes (Anthropic
> calls with strict JSON schemas) for archetype classification
> + action vocab refinement. v1 ships the deterministic
> offline pipeline ... The orchestrator's shape leaves room
> for an LLM refinement pass to slot in without reshaping.

This brief adds that refinement pass as an OPT-IN flag.

## Behaviour

```
argo genesis create --charter "..." --persona "..." --output ./out
  → uses deterministic offline pipeline (current behaviour)

argo genesis create --charter "..." --persona "..." --output ./out --refine
  → after the deterministic pass produces a draft, makes an
    Anthropic call with a strict JSON schema asking the model
    to:
    1. Confirm or revise the archetype assignment
    2. Suggest 0-3 additional actions the deterministic seed
       missed for this charter
    3. Tighten the system prompt with charter-specific
       context (5-10 line additions, NOT a rewrite)
  → applies the model's output ONLY if it parses cleanly
    against the schema; on any validation failure, falls back
    to the deterministic output and emits a warning
```

## Files you may touch
- `crates/argo-genesis/src/refine.rs` (NEW)
- `crates/argo-genesis/src/pipeline.rs` (extend — add the
  optional `refine` step at the end of the existing pipeline
  + thread the flag through)
- `crates/argo-genesis/src/lib.rs` — narrowly: declare the
  new module + export `RefineConfig`
- `crates/argo-cli/src/cli_genesis_cmds.rs` — narrowly: add
  the `--refine` flag
- `crates/argo-cli/src/handlers/genesis.rs` — narrowly: pass
  the flag through to the pipeline

## Files you must NOT touch
- The 12-archetype classifier (it's the deterministic floor;
  the refine pass either confirms or revises after)
- The safety judge (hard rule — even refined actions must
  pass it)
- argo-provider trait impls (use the existing
  AnthropicProvider via the workspace pattern)

## Acceptance criteria
- [ ] `--refine` flag added; default off (v1 behaviour
      unchanged when flag absent)
- [ ] Refinement step runs strictly AFTER the safety judge,
      and any actions it adds re-run the safety judge —
      sensitive verbs that would auto-approve are still
      blocked
- [ ] If `ANTHROPIC_API_KEY` is unset and `--refine` is
      passed, fail with a clear error message at startup,
      NOT mid-run
- [ ] JSON schema validation: model output that doesn't
      parse cleanly is rejected; pipeline falls back to the
      deterministic output + emits a `tracing::warn!`
- [ ] Unit tests:
      - schema-rejection path with synthetic bad JSON
      - safety-judge re-run path with a synthetic refined
        action that has a sensitive verb but wrong
        `requires_approval` flag (assert it gets fixed)
- [ ] Integration test gated on `ANTHROPIC_API_KEY` that
      runs the full --refine path against a real model
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean
- [ ] Every authored file < 500 LOC

## Branch + PR
- Branch: `claude-genesis-llm-refinement`
- PR title: `genesis: optional LLM-driven refinement pass behind --refine (claude-genesis-llm-refinement)`

## Notes
- Use the existing `argo-provider` Anthropic transport — do
  NOT add a new HTTP client.
- Strict JSON schema: define it inline as a `serde::Deserialize`
  struct; reject anything that doesn't match.
- The refine pass is opt-in for v1 because deterministic
  output is the safer default for the demo. Document this
  in the brief's PR body.
