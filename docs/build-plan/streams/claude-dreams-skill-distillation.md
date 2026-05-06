# V2.0 — Dreams skill distillation (Integration B)

**Owner**: claude. **Depends on**: claude-cu-skill-learn-wire (#807),
claude-skill-success-metric (#813),
claude-dreams-memory-summarize (queued) merged. From dreams
investigation `docs/skycot/DREAMS_AND_RUNTIME_SWARM.md` Integration B.

## Scope

Today `maybe_propose` mechanically generates a SkillCandidate
body from a successful CU trajectory. Quality varies — some
candidates are great, some are noisy. Use Claude Dreams to
**distill skill candidates from cohorts of successful
trajectories**: feed N similar trajectories through Dreams and
get back a polished candidate body.

- New `argo-skill::dreams_distillation` module
- Trigger: nightly tick scans recent trajectories grouped by
  (worker, action_kind), fires Dreams once per cohort meeting
  thresholds (≥3 successful trajectories with similar
  trajectories cosine sim > 0.7)
- Output: a SkillCandidate with `body_source: "dreams_distilled"`
  + cohort size + cohort sample IDs
- Cost guardrail: same daily cap pattern as memory-summarize
- The dream output replaces the mechanical body when both
  exist; mechanical fallback remains for cohorts < 3

## Files you may touch
- `crates/argo-skill/src/dreams_distillation.rs` (NEW)
- `crates/argo-skill/src/candidate.rs` — narrowly: hook
  Dreams path before falling back to mechanical
- `crates/argo-skill/src/lib.rs` — narrowly: re-export
- `crates/argo-skill/Cargo.toml` — narrowly: feature flag
- `crates/argo-cli/src/handlers/skill_distill_now.rs` (NEW)
- `crates/argo-cli/src/cli_skill_cmds.rs` — narrowly:
  `DistillNow` subcommand variant
- `crates/argo-cli/tests/skill_dreams_smoke.rs` (NEW)

## Acceptance criteria
- [ ] Cohort grouping works (cosine sim > 0.7 threshold)
- [ ] Dreams API call with cohort produces candidate body
- [ ] body_source recorded as dreams_distilled
- [ ] Mechanical fallback when cohort < 3
- [ ] Daily cap shared with memory-summarize
- [ ] 8 unit tests + 1 integration smoke
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `claude-dreams-skill-distillation`
- PR title: `skill: Dreams distillation for cohort-based skill candidates (claude-dreams-skill-distillation)`
