# Mission load-bearing wire — relax LLM refinement 3-action cap

**Owner**: claude. **Depends on**: `argo-genesis::refine` SHIPPED (verified at `crates/argo-genesis/src/refine.rs:181 refine_plan`, `:225 apply_refinement`); broader refinement flow SHIPPED (`claude-genesis-llm-refinement` MERGED).
**Authored against**: origin/main `8c4bafaf` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-llm-refine-uncap-actions`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 3 item #59 ("Allow LLM refinement to add MORE than 3 actions"). The broader refinement infrastructure landed (`claude-genesis-llm-refinement`), but a 3-action cap remains either in the LLM prompt template (instructing the model "propose up to 3 new actions") or in parsing/truncation. Mission Claim 3 ("platform builds workers") wants refinement to keep up with charters that genuinely need more than 3 new actions.

## Scope

1. **First 5 min**: locate the 3-action cap. Likely candidates:
   - LLM prompt template literal in `refine.rs` or `refine_*.md` (search: `rg -n "3 actions\|three actions\|up to 3\|max.*3" crates/argo-genesis/`)
   - Parser truncation in `parse_refinement_response` or similar
   - JSON schema `maxItems: 3` constraint
2. **Replace the cap** with a configurable bound. NEW `RefineConfig.max_new_actions: usize` (default **8**, configurable via `ARGO_GENESIS_REFINE_MAX_ACTIONS`). Prompt template updated to interpolate the bound: "propose up to {max_new_actions} new actions". Parser respects the same bound; anything past it is silently truncated (logged at debug-level — operator can see the cap firing).
3. **Safety judge gating preserved**: every refined action still passes through `safety_judge::auto_fix` + `safety_judge::judge` exactly as today. Lifting the count cap does NOT lift the safety gate.

Tests:
- Existing tests with 1-3 actions still pass byte-identical (the new default 8 is permissive enough).
- NEW test: 5 actions returned by mock LLM all land in the manifest (proves cap is lifted).
- NEW test: setting `max_new_actions: 2` truncates a 5-action response to 2 (proves config knob works).
- NEW test: safety judge still fires on a refined action with sensitive verb (proves gating preserved).

This is NOT in scope: changing `safety_judge`; per-vertical / per-tenant cap; rejecting (vs. silently truncating) over-cap responses.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-genesis/src/refine.rs` — narrowly scoped: ADD `pub max_new_actions: usize` to `RefineConfig`; update `from_env` to parse env var; update prompt template to interpolate; cap truncation in parsing. NO change to `apply_refinement` or `merge_proposed_actions`. NO change to `safety_judge` calls.
- `crates/argo-genesis/src/refine_tests.rs` (or sibling) — narrowly scoped: append 3 NEW tests as above. Existing tests unchanged.
- `crates/argo-genesis/templates/refine_system.md` (or wherever the system prompt template lives — verify in first 5min) — narrowly scoped: replace hardcoded "3" with `{{ max_new_actions }}` Handlebars / template placeholder.

## Files you must NOT touch

- `crates/argo-genesis/src/safety_judge.rs` — safety gating is upstream; reuse only
- `crates/argo-genesis/src/action_suggest.rs` — seed-action machinery is upstream
- Any non-genesis crate
- `crates/argo-store/migrations/**` — no schema change

## Acceptance criteria

- [ ] `RefineConfig.max_new_actions: usize` exists, default 8
- [ ] `ARGO_GENESIS_REFINE_MAX_ACTIONS` env var overrides default
- [ ] Prompt template interpolates the bound (no hardcoded "3" in template)
- [ ] Parser truncates over-cap responses with a debug-level log; does NOT error
- [ ] All existing `refine_tests.rs` tests still pass byte-identical
- [ ] 3 new tests cover: >3 actions land; configurable cap truncates; safety judge still gates
- [ ] No new public API outside `RefineConfig.max_new_actions`
- [ ] `cargo build -p argo-genesis` clean
- [ ] `cargo clippy -p argo-genesis --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-genesis` passes (skip pre-existing flakes)
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-llm-refine-uncap-actions`
- PR title: `genesis: configurable max_new_actions in refinement (mission-llm-refine-uncap-actions)`

## Notes for the implementer

- 8 is the brief's default — generous enough to cover real charters that need 4-6 new actions, conservative enough that runaway LLM hallucination is still bounded.
- If the cap turns out to be ONLY in the prompt template (model is asked for ≤3 but parser accepts arbitrary), the change is one-line: update the template. Still ship the config knob for operator control.
- Don't error on over-cap; truncate silently with a debug log. The cap is a soft-guard, not a hard validation rule. Operators who hit the cap and care will see the log.
- If the implementer discovers the cap lives in the LLM response JSON schema, update the schema's `maxItems` to use the configured bound at request time.
