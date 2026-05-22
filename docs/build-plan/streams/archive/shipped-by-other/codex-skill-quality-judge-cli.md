# V2.0 — Skill quality judge CLI

**Owner**: codex. **Depends on**: claude-skill-quality-judge
(queued) merged. Companion CLI surface to the architectural
brief.

## Scope

The architectural brief (claude-skill-quality-judge) ships
the score formula + persistence + recompute tick. This brief
ships the operator surface:

- `argo skill quality <skill-name>` — show current score +
  inputs snapshot + score history (last 30 days)
- `argo skill quality-list [--threshold <n>] [--worst]` —
  rank skills by quality
- `argo skill quality-recompute <skill-name>` — force
  immediate recompute (otherwise nightly)
- `argo skill quality-trend <skill-name>` — print ASCII
  spark chart of last 30 daily scores

Plus extending `argo skill list` with a `--quality-min <n>`
filter (already in claude-skill-quality-judge brief, but the
CLI surface there is just the basic 3 subcommands; this
brief adds the trend + extends list).

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-cli/src/handlers/skill_quality.rs` (NEW or
  extend if claude-skill-quality-judge ships a stub)
- `crates/argo-cli/src/handlers/skill_quality_list.rs` (NEW)
- `crates/argo-cli/src/handlers/skill_quality_trend.rs` (NEW —
  ASCII sparkline)
- `crates/argo-cli/src/handlers/skill_quality_recompute.rs` (NEW)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 4 pub mods
- `crates/argo-cli/src/cli_skill_quality_cmds.rs` (NEW or
  extend)
- `crates/argo-cli/src/cli_skill_cmds.rs` — narrowly: extend
  `list` with `--quality-min` flag
- `crates/argo-cli/tests/skill_quality_smoke.rs` (NEW)

## Acceptance criteria
- [ ] All 4 subcommands work end-to-end
- [ ] Sparkline renders 30 daily scores in 30 chars
- [ ] `--quality-min` filter on `argo skill list` works
- [ ] `--json` envelope universal
- [ ] 6 unit tests + 1 CLI smoke
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `codex-skill-quality-judge-cli`
- PR title: `cli: argo skill quality + quality-list + quality-trend + quality-recompute (codex-skill-quality-judge-cli)`
