# V1.x — Cost-by-skill CLI

**Owner**: codex. **Depends on**: H-budget-cli (#820),
F-skills-self-learning merged.

## Scope

`argo budget` shows spend by worker + by day. Operators
also want **cost-by-skill** — which skills are most
expensive to run, which are cheapest. Useful for skill
selection during AB tests + budget allocation.

- `argo budget by-skill [--since <duration>]
  [--limit <n>] [--vertical <id>]` — top N skills by spend
- `argo budget by-skill <skill-name> --history` — per-day
  spend trend for a single skill
- `--json` envelope per universal convention

Backed by joining `worker_action` / `spend_ledger` /
`skill` tables (skill_id should already exist on
worker_action rows from H-spend-persistence).

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-cli/src/handlers/budget_by_skill.rs` (NEW)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: pub mod
- `crates/argo-cli/src/cli_budget_cmds.rs` — narrowly: add
  `BySkill` subcommand variant
- `crates/argo-store/src/budget.rs` (or extend) — narrowly:
  add `spend_by_skill_top_n` + `spend_by_skill_history`
  query helpers
- `crates/argo-cli/tests/budget_by_skill_smoke.rs` (NEW)

## Acceptance criteria
- [x] `argo budget by-skill` returns ranked list
- [x] `--since 7d/30d/90d` honored
- [x] `<skill-name> --history` returns per-day rows
- [x] `--json` universal envelope
- [x] 4 unit tests (query helpers) + 1 CLI smoke test
- [x] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `codex-cost-by-skill-cli`
- PR title: `cli: argo budget by-skill — top-N + history (codex-cost-by-skill-cli)`
