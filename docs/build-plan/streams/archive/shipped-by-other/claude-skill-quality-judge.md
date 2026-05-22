# V2.0 — Skill quality judge

**Owner**: claude. **Depends on**: H-skill-review-cli (#823),
claude-skill-success-metric (#813),
claude-skill-versioning-ab-test (#863) merged.

## Scope

Per-skill quality score, computed across recent invocations.
Inputs: success_rate, avg_latency, retry_count, operator
thumbs (deferred via separate brief), p99 latency. Outputs a
0-100 quality score that operators can use to:
- Filter skill listings (`argo skill list --quality-min 80`)
- Trigger automatic A/B tests when score drops
- Demote skills below a threshold

- New table `skill_quality_score(skill_id, score, computed_at,
  inputs_snapshot)` — one row per skill per nightly compute
- Background tick (reuse existing tokio interval pattern)
  recomputes scores every N hours
- New CLI: `argo skill quality <skill>`,
  `argo skill quality-list [--threshold <n>] [--worst]`,
  `argo skill quality-recompute <skill>` (force)

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-skill/src/quality.rs` (NEW)
- `crates/argo-skill/src/lib.rs` — narrowly: re-export
- `crates/argo-store/migrations/<next>__skill_quality_score.sql`
  (NEW)
- `crates/argo-store/src/skill_quality.rs` (NEW)
- `crates/argo-cli/src/handlers/skill_quality_*.rs` (3 NEW)
- `crates/argo-cli/src/cli_skill_quality_cmds.rs` (NEW —
  sibling-file pattern alongside existing cli_skill_*)
- `crates/argo-cli/src/cli_subcommands.rs` — narrowly: add
  `Quality` subcommand under `Skill`

## Acceptance criteria
- [ ] Score formula documented in module-level comment
- [ ] Score persists per skill per compute cycle
- [ ] Recompute job runs at startup + on N-hour interval
- [ ] `--quality-min` filter on `argo skill list`
- [ ] All 3 new subcommands work + `--json` universal
- [ ] 10 unit tests (formula edge cases) + 2 integration
      (full recompute cycle)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `claude-skill-quality-judge`
- PR title: `skill: per-skill quality score + auto-recompute (claude-skill-quality-judge)`
