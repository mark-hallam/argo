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

> **Mid-flight clarification (claude/mark-opus-4d5e8a)** —
> the brief originally pointed at `candidate.rs` and
> `cli_skill_cmds.rs` which do not exist; `SkillCandidate`
> lives in `learn.rs` and the skill subcommand enum lives
> in `cli_subcommands_skill.rs`. Paths corrected below to
> match the real layout (no scope expansion).

- `crates/argo-skill/src/dreams_distillation.rs` (NEW)
- `crates/argo-skill/src/dreams_distillation_tests.rs` (NEW
  sibling — `#[cfg(test)] #[path] mod tests;`)
- `crates/argo-skill/src/learn.rs` — narrowly: add
  `body_source: Option<String>` field on `SkillCandidate`
  (was: `candidate.rs` in stale brief; `SkillCandidate`
  lives in `learn.rs`)
- `crates/argo-skill/src/lib.rs` — narrowly: re-export
- `crates/argo-skill/Cargo.toml` — narrowly: feature flag
  + dep on `argo-memory` (re-uses `DailyCapTracker`)
- `crates/argo-cli/src/handlers/skill_distill_now.rs` (NEW)
- `crates/argo-cli/src/cli_subcommands_skill.rs` — narrowly:
  `DistillNow` subcommand variant (was: `cli_skill_cmds.rs`
  in stale brief; the enum lives in `cli_subcommands_skill.rs`)
- `crates/argo-cli/src/handlers/skill.rs` — narrowly: dispatch
  the `DistillNow` arm to the new handler
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: declare
  the new module
- `crates/argo-cli/tests/skill_dreams_smoke.rs` (NEW)
- BLANKET CLI ALLOWLIST permits `cli.rs` / `Cargo.toml` /
  `tests/` / `handlers/mod.rs` under strict scope.

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
