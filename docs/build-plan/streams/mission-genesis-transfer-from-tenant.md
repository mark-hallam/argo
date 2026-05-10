# Mission load-bearing wire — Genesis from existing tenant vertical (transfer-learning seed)

**Owner**: claude. **Depends on**: `argo-genesis::pipeline::plan` SHIPPED (verified at `crates/argo-genesis/src/pipeline.rs:35`); `argo-vertical::manifest::parse` SHIPPED; existing in-tree verticals as seeds.
**Authored against**: origin/main `b74b9cf2` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-genesis-transfer-from-tenant`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 3 item #62 ("Add Genesis from existing tenant verticals (transfer learning hint)"). Today Genesis starts from a fresh charter every time, even when the operator's tenant already has 5 working verticals with the right shape. Mission Claim 3 ("platform builds workers") promises Genesis can BOOTSTRAP from a related sibling vertical instead of starting from scratch — operator picks "make me a new vertical like sales-sdr but for legal" and Genesis seeds from sales-sdr's shape.

## Scope

Add `plan_from_seed_vertical(charter: &str, persona: &str, seed_vertical_id: &str) -> Result<GenesisPlan, GenesisError>` to `argo-genesis::pipeline`:

1. Load the seed `VerticalManifest` (from `verticals/<seed_id>/vertical.toml` via `argo-vertical::loader`).
2. Run the standard `plan()` pipeline to get a baseline `GenesisPlan` from the charter alone.
3. **Merge** the seed's action vocabulary + skill stubs + workflow shape into the baseline:
   - Actions: union of seed's `[actions]` + plan's; seed wins on `risk` classification.
   - Skills: include the seed's skill list as "starter skills" (operator can prune).
   - Workflow: prefer seed's workflow structure when plan has none.
   - Persona's voice profile: inherit seed's `tone`/`formality` defaults; freeform preamble from operator's input wins.
4. Stamp a `seeded_from: Some(seed_vertical_id)` field on the manifest so downstream eval/forge knows the lineage.

CLI: `argo genesis from-vertical <charter-file> --seed <vertical-id>` invokes the seeded pipeline. Without `--seed`, behavior identical to today.

This is NOT in scope: cross-tenant seed (only same-tenant verticals); merging eval suites (deferred); seed-pinning at version-level (seed is "current"); auto-suggest seed via similarity scoring (separate brief).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-genesis/src/pipeline.rs` — narrowly scoped: ADD `plan_from_seed_vertical(charter, persona, seed_vertical_id)`. Internal helper `merge_seed_into_plan(baseline: GenesisPlan, seed: VerticalManifest) -> GenesisPlan` that does the union logic. NO change to existing `plan()` / `plan_with_quality()`. File currently ~150 LOC.
- `crates/argo-genesis/src/pipeline_seed_tests.rs` (NEW, sibling) — 8 unit tests: seed-actions-merge; seed-skills-included; baseline-wins-on-conflict; seeded_from-stamped; missing-seed-errors; voice-profile-inherits-tone; workflow-inherits-when-baseline-empty; baseline-workflow-preserved-when-set.
- `crates/argo-genesis/src/lib.rs` — narrowly scoped: re-export `plan_from_seed_vertical`.
- `crates/argo-genesis/Cargo.toml` — narrowly scoped: verify `argo-vertical` is already a dependency or add it.
- `crates/argo-cli/src/handlers/genesis.rs` (verify path in first 5min via `rg -l "fn cmd_genesis" crates/argo-cli/src/`) — narrowly scoped: ADD `argo genesis from-vertical --seed <id>` subcommand wiring.
- `crates/argo-cli/tests/genesis_from_seed_smoke.rs` (NEW, ≤ 120 LOC) — integration smoke: run `argo genesis from-vertical` with `--seed sales-sdr` (an existing in-tree vertical), assert produced manifest contains sales-sdr's "schedule_followup" action.

## Files you must NOT touch

- `crates/argo-genesis/src/archetype.rs` — classifier is upstream; reuse only
- `crates/argo-genesis/src/refine.rs` — refinement is upstream
- `crates/argo-vertical/src/manifest.rs` — manifest types are upstream; reuse only
- Any non-genesis/non-cli crate
- `crates/argo-store/migrations/**` — no schema change (Genesis is store-less; manifest is returned, not persisted in this brief)

## Acceptance criteria

- [ ] `plan_from_seed_vertical(charter, persona, seed_vertical_id)` exists, returns `Result<GenesisPlan, GenesisError>`
- [ ] Errors clearly when `seed_vertical_id` doesn't exist (`GenesisError::Validation("seed vertical 'X' not found at <path>")`)
- [ ] Action merge: union, baseline wins on conflict, risk classification carries forward from seed
- [ ] Skill list: seed's skills included as starter skills, operator can prune via refinement
- [ ] Voice profile: tone/formality inherit from seed; baseline's freeform preamble wins
- [ ] Workflow: inherit from seed when baseline has empty workflow
- [ ] `seeded_from: Option<String>` stamped on the produced manifest
- [ ] Existing `plan()` / `plan_with_quality()` unchanged byte-identical
- [ ] CLI `argo genesis from-vertical <charter> --seed <id>` works
- [ ] 8 unit tests + 1 integration smoke
- [ ] No new public API outside the listed exports
- [ ] `cargo build -p argo-genesis -p argo-cli` clean
- [ ] `cargo clippy -p argo-genesis -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-genesis -p argo-cli` passes (skip pre-existing flakes)
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-genesis-transfer-from-tenant`
- PR title: `genesis: seed Genesis pipeline from existing tenant vertical (mission-genesis-transfer-from-tenant)`

## Notes for the implementer

- "Baseline wins on conflict" matters: if the operator's charter specifies a stricter risk for an action than the seed's, the operator's choice must persist. The seed is a **starting point**, not an override.
- The `seeded_from` lineage field is small but high-value — downstream Forge / quality regression detection can group skills by seed lineage for cohort analysis.
- Tests against an in-tree vertical (`sales-sdr` exists per `verticals/sales-sdr/`) — don't fabricate fixture manifests; reuse what ships.
- If `argo-vertical::loader` doesn't already expose a `load_by_id` fn, add a narrow one in this brief; document as may-touch addition in the PR body.
