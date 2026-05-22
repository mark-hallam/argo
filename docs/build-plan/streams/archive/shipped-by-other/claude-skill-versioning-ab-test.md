# V2.0 — Skill versioning + A/B test framework

**Owner**: claude. **Depends on**: F-skills-self-learning,
codex-skill-marketplace-search merged.

## Scope

Multiple versions of the same skill stored side-by-side.
`argo skill ab-test <skill> --variants vN,vM --runs 100`
deploys a 50/50 split for the next N invocations + tracks
per-version success rate; once stat-significant, promotes
the winner.

Mission claim: "workers grow new skills from successful
trajectories" — v1 ships `SkillCandidate::propose_for_review`
but no versioning + no comparative testing. v2.0 closes this.

- New `skill_version(skill_id, version, manifest, body,
  trust_grade, created_at)` table
- New `skill_ab_test_run(id, skill_id, variant_a, variant_b,
  target_runs, started_at, status)` table
- New `skill_ab_test_outcome(run_id, invocation_id,
  variant_chosen, succeeded, latency_ms)` table
- A/B selection logic: deterministic hash of invocation ID
  → 50/50 split
- Significance check: chi-square or 2-prop z-test; default
  α=0.05; min sample 30 per variant
- New CLI: `argo skill versions <skill>`,
  `argo skill ab-test <skill> --variants ... --runs ...`,
  `argo skill ab-status <run-id>`,
  `argo skill ab-promote <run-id> --winner <variant>`
- Auto-promote when significant + min-sample reached

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-skill-store/src/version.rs` (NEW)
- `crates/argo-skill-store/src/ab_test.rs` (NEW)
- `crates/argo-store/migrations/<next>__skill_versions.sql`
  (NEW)
- `crates/argo-store/migrations/<next+1>__skill_ab_tests.sql`
  (NEW)
- `crates/argo-runtime/src/skill_dispatch.rs` — narrowly:
  variant-selection hook
- `crates/argo-cli/src/handlers/skill_ab_*.rs` (4 NEW)
- `crates/argo-cli/src/cli_skill_cmds.rs` — narrowly: 4
  variants

## Acceptance criteria
- [ ] Skill versions stored + listed
- [ ] A/B test creates run + routes 50/50
- [ ] Stat-significance check works (3 known cases
      verified)
- [ ] Auto-promote when significant + min-sample reached
- [ ] `--json` universal output
- [ ] 10 unit tests + 2 integration tests
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR
- Branch: `claude-skill-versioning-ab-test`
- PR title: `skill: versioning + A/B test framework (claude-skill-versioning-ab-test)`
