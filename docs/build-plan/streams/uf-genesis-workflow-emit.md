# uf-genesis-workflow-emit (UF Phase-C · C1)

**Owner**: claude.
**Depends on**: A2 (`uf-genesis-emit-real-kpis-and-cadence`). tenant keystone (#2676).
**Authored against**: origin/main `58a35a9f9` on `2026-06-05`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh uf-genesis-workflow-emit`.
**BLOCKED until A2 merges** — conductor flips BLOCKED→TODO when `uf-genesis-emit-real-kpis-and-cadence` is MERGED (low-risk auto-cascade).

## Why
No generated teammate emits a `[workflows]` section. Emit a minimal `workflows/<name>.workflow.toml` per primary duty in the generated vertical, matching the hand-built reference, so a hired teammate ships with a runnable workflow.

## Grep-before-write (check overlap FIRST — per the plan)
- Read the prior genesis-refinement briefs to confirm NONE already emit workflows (avoid a parallel surface): `docs/build-plan/streams/{claude-genesis-iteration-cycle,claude-genesis-refine-split,codex-genesis-refine-cli}.md` (verified present 2026-06-05).
- Reference template shape: `verticals/receptionist/workflows/caller_greeting_check.workflow.toml`.
- Validate against `crates/argo-workflow` (load/validate) — reuse, don't fork.

## Files you may touch
- `crates/argo-genesis/src/materialize.rs` (or `materialize_writer`): emit one minimal `workflows/<duty>.workflow.toml` per primary duty, schema-matching the receptionist reference; validate the emitted TOML against `argo-workflow` before writing.

## Acceptance
A generated teammate with a primary duty emits a `workflows/<name>.workflow.toml` that `argo_workflow::load` validates; matches the receptionist reference shape; files stay <500 LOC.

## On ship (same commit)
Update `docs/build-plan/FEATURE_MATRIX.md` (genesis workflow-emit row) + regen HTML (`python3 scripts/render-feature-matrix.py`).

## Effort
~1–2 briefs (M).
