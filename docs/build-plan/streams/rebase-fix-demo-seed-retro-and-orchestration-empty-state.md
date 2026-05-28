# rebase-fix-demo-seed-retro-and-orchestration-empty-state

**Owner**: codex.
**Authored against**: origin/main `a5df90c41` on `2026-05-29`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh rebase-fix-demo-seed-retro-and-orchestration-empty-state`.
**Supersedes**: PR #2587 (closed for re-author 2026-05-29) + original brief `streams/fix-demo-seed-retro-and-orchestration-empty-state.md`.

🟡 **P1 — `/retro` and `/orchestration` render empty-state dead-ends in the authed demo session. Investor walkthroughs beyond the 6-scene runbook land on the dead-end. Re-implement against current main; PR #2587's 750-LOC seed touched lib.rs/seed.rs/plan.rs which #2584 reshaped.**

## What changed on main since PR #2587 was authored

PR #2584 (`766f6b0d0` — "fix(demo-seed): add pitch-agent + 3 dept-filler personas for /cohorts coverage") landed and reshaped:
- `crates/argo-demo-seed/src/lib.rs` — new module declarations + SeedReport shape
- `crates/argo-demo-seed/src/seed.rs` — new persona wiring path
- `crates/argo-demo-seed/src/plan.rs` — new plan rows
- Added new modules: `dept_filler.rs`, `personas.rs` (canonical persona registry)

The current `crates/argo-demo-seed/src/` layout (verified 2026-05-29):
- `dept_filler.rs`, `lib.rs`, `marketplace.rs`, `personas.rs`, `plan.rs`, `seed.rs`, `skills.rs`, `workflow.rs`

PR #2587's new `retros.rs` and `orchestration.rs` modules do NOT exist on main. The feature is genuinely unshipped.

## What this brief MUST ship

Re-implement PR #2587's full posture against current `origin/main` HEAD:

1. **3 retros** seeded under the demo tenant via new `crates/argo-demo-seed/src/retros.rs`:
   - "Sprint 28 retro — marketplace pricing experiment" (Mariah-led, mixed verdict)
   - "Q1 review — candidate funnel improvements" (Priya-led, positive verdict)
   - "Standup digest reliability post-mortem" (Peta-led, negative verdict + 3 action items)
2. **2 orchestration sessions** seeded via new `crates/argo-demo-seed/src/orchestration.rs` (`provider = 'orchestrator'`):
   - "Marketplace listing → re-price weekly" (Mariah)
   - "New customer onboarding → handoff to project-coordination" (Peta)
3. Each retro carries metrics, the 5-section narrative body referencing the seeded `customer_research_followup` workflow, and a real `worker_id` so the SPA card preview renders.
4. `SeedReport` gains `retros_created` + `orchestration_runs_created` fields; CLI handler prints both in the Pass B block.
5. **Integration test** at `crates/argo-cli/tests/demo_seed_retro_orchestration_coverage.rs`:
   - `GET /api/v1/retros` → `RetroStore::list_all` (≥3 rows + each row has a non-empty section for the card preview).
   - `GET /api/v1/orchestration/runs` → `SELECT … WHERE provider = 'orchestrator'` (≥2 rows + expected titles).
   - Sibling-tenant invisibility (0 rows under a different tenant), mirroring the canonical Scene-6 guard.

**Personas**: per #2584 the canonical roster is now in `personas.rs`. Confirm Mariah / Peta / Priya are still the leads named there. The original brief listed Patrick for the standup-digest retro; Patrick is NOT in the canonical demo persona list. Swap to Peta who covers Project & Ops.

**Wiring path**: in current `crates/argo-demo-seed/src/lib.rs`, follow the same pattern PR #2584 used for `dept_filler` + `personas` — declare the new module, plumb the seed call from `seed.rs::seed_demo_workspace()`, surface counts in `SeedReport`.

## Acceptance criteria

- [ ] `bash scripts/check-brief-freshness.sh rebase-fix-demo-seed-retro-and-orchestration-empty-state` passes.
- [ ] `cargo clippy -p argo-demo-seed -p argo-cli --all-targets -- -D warnings` clean.
- [ ] `cargo test -p argo-demo-seed -p argo-cli --test demo_seed_retro_orchestration_coverage --lib` all pass.
- [ ] `bash scripts/workspace-clippy-fast-fail.sh` clean.
- [ ] After running `argo demo seed --tenant <id> --skip-pass-a-turns`, `/retro` shows ≥3 cards and `/orchestration` shows ≥2 rows.

## May-touch

- `crates/argo-demo-seed/src/lib.rs` (module declarations + SeedReport)
- `crates/argo-demo-seed/src/seed.rs` (wire the new seed calls into the Pass B path that #2584 established)
- `crates/argo-demo-seed/src/plan.rs` (Pass B plan rows for retros + orchestration)
- `crates/argo-demo-seed/src/retros.rs` (NEW)
- `crates/argo-demo-seed/src/orchestration.rs` (NEW)
- `crates/argo-demo-seed/Cargo.toml` (likely no change; verify RetroStore + OrchestratorStore are already deps)
- `crates/argo-cli/src/handlers/demo_seed.rs` (Pass B count printing)
- `crates/argo-cli/tests/demo_seed_retro_orchestration_coverage.rs` (NEW integration test)

## Out of scope

- Touching `personas.rs` / `dept_filler.rs` (out of scope — those are #2584's territory).
- Wiring Patrick into the canonical roster (separate brief if operator wants him added).
- Gateway-side endpoint shape for `/api/v1/retros` or `/api/v1/orchestration/runs` (no change needed; the data path already exists).

## Effort

~750 LOC across two new modules + integration test + plan/seed wiring. ~3-4 hours.

## Source

- Original brief: `streams/fix-demo-seed-retro-and-orchestration-empty-state.md`
- Closed PR: #2587 (closed 2026-05-29 as REAL-CONFLICT-NEEDS-REAUTHOR; 750 LOC across argo-demo-seed modules reshaped by #2584)
- UX audit: `docs/skycot/ux-audits/2026-05-26-2152/whole-site-mvp-claims-coverage.md` F08
