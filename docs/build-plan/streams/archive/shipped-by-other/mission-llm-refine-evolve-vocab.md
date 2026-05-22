# Mission load-bearing wire — LLM refinement evolves seed action vocabulary

**Owner**: claude. **Depends on**: `argo-genesis::refine::apply_refinement` SHIPPED (verified at `crates/argo-genesis/src/refine.rs:225`); `merge_proposed_actions` SHIPPED; `safety_judge` SHIPPED.
**Authored against**: origin/main `c8c959a6` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-llm-refine-evolve-vocab`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 3 item #60 ("Allow LLM refinement to evolve the seed action vocabulary"). Today's refinement (`merge_proposed_actions`) accepts only ProposedActions whose `action_type` ALREADY exists as a key in the seed vocabulary — silent drop otherwise. New verbs (`schedule_meeting`, `escalate_to_legal`, etc.) get filtered out before reaching the manifest. Mission Claim 3 ("platform builds workers") needs refinement to grow the vocabulary, not just refine within it.

## Scope

Change refinement merge semantics:

- **Today**: `merge_proposed_actions(actions, proposed)` skips proposed entries whose `action_type` is not already in `actions.actions`. (Verify exact filter in first 5min — implementer locates via `rg -n "contains_key\|already.*exists" crates/argo-genesis/src/refine.rs`.)
- **NEW**: proposed entries with NEW action_types are ACCEPTED into the vocabulary IF they pass `safety_judge::judge` AND there's room under `RefineConfig.max_vocab_size` (default 64, configurable via `ARGO_GENESIS_REFINE_MAX_VOCAB`).

Add `evolved_vocab: Vec<String>` to `RefinementResponse` (action_types newly added in this refinement pass). Empty when no new vocab grew. Useful for operator visibility — `argo genesis refine --show-vocab-evolution` prints the list.

Safety gate is load-bearing:
- Every new action_type goes through `safety_judge::auto_fix` then `safety_judge::judge` BEFORE landing in the manifest.
- If a new action_type's `risk` is `Sensitive` or `Destructive`, `requires_approval = true` is forced on (defense-in-depth — model may have proposed `requires_approval = false`).

This is NOT in scope: removing actions from vocabulary (Forge handles low-quality skill removal — separate brief); renaming action_types; per-tenant vocab caps; action-type aliases.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-genesis/src/refine.rs` — narrowly scoped: ADD `pub max_vocab_size: usize` to `RefineConfig` (default 64), `from_env` parses override; change `merge_proposed_actions` to ACCEPT new action_types (under cap); ADD `evolved_vocab: Vec<String>` to `RefinementResponse`; defense-in-depth force `requires_approval = true` on Sensitive/Destructive risk classes. NO change to `apply_refinement`'s safety-judge invocation order.
- `crates/argo-genesis/src/refine_tests.rs` (or sibling) — narrowly scoped: append 5 NEW tests: new-action-type-lands-in-vocab; new-action-type-rejected-when-cap-reached; sensitive-risk-forces-requires-approval; existing-action-type-still-merges-normally; evolved-vocab-field-populated-correctly. Existing tests unchanged.
- `crates/argo-cli/src/handlers/genesis.rs` (or wherever Genesis CLI lives) — narrowly scoped: ADD `--show-vocab-evolution` flag to `argo genesis refine`, prints evolved_vocab list. NO change to default output shape.

## Files you must NOT touch

- `crates/argo-genesis/src/safety_judge.rs` — safety gating is upstream; reuse only
- `crates/argo-genesis/src/action_suggest.rs` — seed-action generation is upstream
- Any non-genesis/non-cli crate
- `crates/argo-store/migrations/**` — no schema change (vocabulary lives in the manifest, not DB)

## Acceptance criteria

- [ ] `RefineConfig.max_vocab_size: usize` exists, default 64
- [ ] `ARGO_GENESIS_REFINE_MAX_VOCAB` env override works
- [ ] `merge_proposed_actions` accepts proposed entries with NEW action_types (under cap)
- [ ] Vocabulary cap enforced — over-cap proposals silently dropped with debug log
- [ ] Sensitive/Destructive risk classes force `requires_approval = true` regardless of model output
- [ ] `RefinementResponse.evolved_vocab: Vec<String>` populated with newly-added action_types this pass
- [ ] All existing refine tests still pass byte-identical (existing-action-type path unchanged)
- [ ] 5 new tests cover the matrix
- [ ] `argo genesis refine --show-vocab-evolution` CLI flag works
- [ ] No new public API outside the listed exports
- [ ] `cargo build -p argo-genesis -p argo-cli` clean
- [ ] `cargo clippy -p argo-genesis -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-genesis -p argo-cli` passes (skip pre-existing flakes)
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-llm-refine-evolve-vocab`
- PR title: `genesis: allow LLM refinement to evolve seed action vocabulary (mission-llm-refine-evolve-vocab)`

## Notes for the implementer

- This brief and `mission-llm-refine-uncap-actions` (#59) overlap conceptually but are orthogonal: #59 lifts the COUNT cap, this brief lifts the VOCAB cap. Both should land independently; if both are claimed simultaneously, the second to land needs a small rebase.
- The 64-entry vocab cap is generous — any real charter has 8-20 actions. The cap exists to prevent runaway LLM hallucination from growing the vocabulary to 200+ entries on a single refinement pass.
- Defense-in-depth on `requires_approval` is load-bearing: a malicious-or-confused LLM could mark `delete_all_data` as `requires_approval = false`. The forced override at safety-judge time prevents that from landing in the manifest.
- The `evolved_vocab` field on RefinementResponse is a tiny addition that pays huge operator visibility dividends. Always populate it (empty vec when no growth) — never make it Optional.
