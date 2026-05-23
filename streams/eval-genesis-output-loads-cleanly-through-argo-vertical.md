# eval-genesis-output-loads-cleanly-through-argo-vertical

**Authored against:** `origin/main` e03680c99 on 2026-05-24 (conductor tick-131).
**Validate before claim:** `bash scripts/check-brief-freshness.sh eval-genesis-output-loads-cleanly-through-argo-vertical`
**Owner:** codex
**Priority:** 🟡 P2 — CLAIM-3 substrate (the platform builds workers)
**Source:** `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md` row `argo-vertical` (🔴 NOT AUDITED) — Mission claim 3 says Genesis-generated verticals must work; need to verify Genesis output round-trips through argo-vertical's loader.

## What to do

Drive `argo-genesis::pipeline::plan + write_vertical` on **5 distinct role descriptions** (cover diverse verbs: sales / support / ops / engineering / analyst — distinct enough that the planner emits 5 distinct vertical trees). For each one:

1. Write the tree to a `tempfile::tempdir()`.
2. Call `argo_vertical::load_dir(<tempdir>)` and assert `Ok(_)` (no parse errors, no missing-file errors).
3. Assert the loaded `Vertical` has the expected shape: `worker.toml` parsed, at least one prompt loaded, at least one eval seed parsed.
4. Assert the loaded vertical contains the **same set of action capabilities** as the planner's input charter (no silent stripping in write → load).

Write a single integration test file `crates/argo-genesis/tests/genesis_output_loads_through_vertical_test.rs`. Use mock LLM client (see `pipeline_end_to_end_test.rs` for the pattern — already mocks `genesis_judge`).

Also: write a short report `docs/skycot/evals/scheduled/2026-05-23/eval-genesis-output-loads-cleanly-through-argo-vertical.md` covering:
- Round-trip verdict per archetype (PASS / FAIL).
- Any silently-stripped fields (planner → vertical writer → vertical loader).
- Recommended fix-briefs if any 🔴 cells emerge.

## May-touch

- `crates/argo-genesis/tests/genesis_output_loads_through_vertical_test.rs` (new)
- `docs/skycot/evals/scheduled/2026-05-23/eval-genesis-output-loads-cleanly-through-argo-vertical.md` (new)
- `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md` (flip the `argo-vertical` row icon + add citation if PASS)

## Acceptance

- 5 round-trip tests pass (one per archetype) — happy path.
- Crate-scoped clippy clean (`cargo clippy -p argo-genesis --all-targets`).
- Workspace clippy clean.
- Report committed.
- No production source touched (audit-only).
- If any 🔴 cells emerge: corresponding fix-brief stub appended to ledger (TODO) + cited in the report.

## LOC budget

~250-450 LOC (test file + report). Sibling pattern: `crates/argo-genesis/tests/pipeline_end_to_end_test.rs`.
