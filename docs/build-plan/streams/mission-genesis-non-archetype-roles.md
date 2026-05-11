# Mission load-bearing wire — Genesis for non-archetype roles (broader generation + safety judge)

**Owner**: claude. **Depends on**: `argo-genesis::archetype::{Archetype, classify_charter, classify_llm}` SHIPPED (verified at `crates/argo-genesis/src/lib.rs:56`); `argo-judge` crate SHIPPED; existing `genesis_from_job_description` pipeline (MERGED via #1295) demonstrates the synthesis pattern.
**Authored against**: origin/main `d50e69b2` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-genesis-non-archetype-roles`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 3 #65 — "Build Genesis for non-archetype roles (broader generation + safety judge)." Today the 12-archetype taxonomy gates every Genesis pass — if the operator's charter doesn't match a known archetype, `classify_charter` falls through to the closest match (often wrong). Mission promise: when no archetype scores above a confidence threshold, the pipeline switches to LLM-driven broader generation + ships the output through the existing judge for safety/quality gating.

## Scope

1. **Confidence threshold**: extend `classify_llm` (or sibling) to return `ClassificationResult` = `{ archetype: Option<Archetype>, confidence: f32 }`. Confidence ≥ 0.6 → use the archetype path (current behavior). Confidence < 0.6 → switch to broader-generation path.

2. **Broader-generation path** — NEW `genesis_broad_role(charter, ctx) -> ArgoResult<GenesisDraft>`:
   - Calls the LLM with a prompt template that DOES NOT pre-specify an archetype shape. Asks for: persona, charter, action_vocabulary (3-8 items), success_metrics (3-5), 1-paragraph onboarding script.
   - Synthesises a `GenesisDraft` with `archetype: "broad"` (NEW sentinel) so downstream tooling can distinguish.
   - NO fall back to one of the 12 archetypes — that's the whole point of this brief.

3. **Safety judge gate**: NEW `judge_genesis_draft(draft, judge_client) -> ArgoResult<JudgeVerdict>`:
   - Calls existing `argo-judge` with a fixed prompt asking 3 questions: (a) does the role contain illegal/harmful instructions? (b) does the action_vocabulary cross into restricted MCP surfaces (financial trades, medical advice without disclaimers, etc.)? (c) is the persona impersonating a real person?
   - Returns `JudgeVerdict::Allow`, `JudgeVerdict::AllowWithDisclaimer(reason)`, or `JudgeVerdict::Block(reason)`.
   - Pipeline: `Block` → return error with verdict reason. `AllowWithDisclaimer` → prepend disclaimer to persona description, continue. `Allow` → continue unchanged.

4. **NEW CLI flag**: `argo genesis from-charter --allow-broad-role` (default OFF). Without the flag, low-confidence charters still error with "no archetype confident match; use --allow-broad-role to enable LLM-driven broader generation." This preserves today's behavior for cautious operators.

5. **Existing `argo genesis from-job-description`** (`mission-genesis-job-description-ingest` MERGED #1295): same `--allow-broad-role` plumbing through the underlying call.

This is NOT in scope: a 13th hardcoded archetype (that's anti-mission); operator-supplied custom templates (separate brief — `mission-genesis-custom-templates`); judge-disagreement handling beyond Allow/Block (separate brief `mission-judge-disagreement-eval` exists in backlog); fine-tuning the judge prompt over time.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-genesis/src/archetype.rs` — narrowly scoped: extend `classify_llm` to return `ClassificationResult`; keep `classify_charter` byte-identical (back-compat). File currently 320 LOC; stay under 500.
- `crates/argo-genesis/src/broad_role.rs` (NEW, ≤ 250 LOC) — the `genesis_broad_role` synthesizer + prompt builder.
- `crates/argo-genesis/src/broad_role_tests.rs` (NEW, sibling, ≤ 200 LOC) — 6 unit tests: low-confidence-triggers-broad; high-confidence-stays-archetype; broad-output-sentinel-archetype-broad; broad-prompt-asks-3-8-actions; broad-empty-charter-errors; broader-roundtrip-via-mock-provider.
- `crates/argo-genesis/src/genesis_judge.rs` (NEW, ≤ 200 LOC) — `judge_genesis_draft` + `JudgeVerdict` enum.
- `crates/argo-genesis/src/genesis_judge_tests.rs` (NEW, sibling, ≤ 150 LOC) — 5 unit tests: allow-clean-draft; block-illegal-instructions; allow-with-disclaimer-medical-without-disclaimer; impersonation-detection; judge-error-falls-back-to-block.
- `crates/argo-genesis/src/lib.rs` — narrowly scoped: re-export `broad_role::*`, `genesis_judge::*`.
- `crates/argo-genesis/src/pipeline.rs` (verify path) — narrowly scoped: ADD the confidence-threshold branch + judge gate at the end. ≤ 50 LOC delta.
- `crates/argo-cli/src/cli_genesis_cmds.rs` (verify path) — narrowly scoped: ADD `--allow-broad-role` flag pass-through.
- `crates/argo-cli/src/handlers/genesis.rs` — narrowly scoped: thread the flag.
- `crates/argo-genesis/Cargo.toml` — narrowly scoped: add `argo-judge` dep IF not already present (verify).

## Files you must NOT touch

- `crates/argo-judge/**` — judge surface stable; reuse only
- The 12 archetype slug constants — preserved verbatim; only `classify_llm` confidence wiring changes
- `crates/argo-store/migrations/**` — no schema change
- Any non-genesis/non-cli crate

## Acceptance criteria

- [ ] `classify_llm` returns `ClassificationResult { archetype, confidence }`; existing tests assert the high-confidence path unchanged
- [ ] Low-confidence (< 0.6) + `--allow-broad-role` → `genesis_broad_role` runs
- [ ] Low-confidence WITHOUT `--allow-broad-role` → explicit error mentioning the flag
- [ ] Broad-role drafts carry `archetype: "broad"` sentinel
- [ ] `judge_genesis_draft` runs on every draft (archetype + broad alike); Block → error
- [ ] AllowWithDisclaimer prepends disclaimer to persona description
- [ ] Judge error → conservative fall-back to Block (don't allow on judge crash)
- [ ] 6 broad-role + 5 judge unit tests + 1 integration smoke (e2e via mock provider)
- [ ] No new public API outside the listed exports
- [ ] `cargo build -p argo-genesis -p argo-cli` clean
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-genesis-non-archetype-roles`
- PR title: `genesis: broader-role generation + judge gate for low-confidence charters (mission-genesis-non-archetype-roles)`

## Notes for the implementer

- The 0.6 confidence threshold is a v1 starting point. Document the value as a tunable knob in the broad_role module rustdoc.
- The judge prompt MUST be short + stable. Long judges drift across model upgrades. ≤ 300 chars, three questions, structured-output format.
- For the "broad" sentinel: downstream code (action_suggest, prompt_engineer) may pattern-match on archetype. Add a fallback path so "broad" doesn't crash anywhere — search `archetype.as_str() ==` and add the "broad" arm or a default branch.
- The "conservative Block on judge error" semantic is load-bearing — never silently allow a draft when the safety judge crashes.
- BLOCK and request revision rather than touch `argo-judge`. If the judge surface doesn't have a "generic prompt verdict" API, that's a precursor brief.
