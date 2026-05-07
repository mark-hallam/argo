# V1.4 — Skill runs CLI

**Owner**: codex. **Depends on**: claude-cu-skill-learn-wire (#807),
H-skill-review-cli (#823) merged.

## Scope

`argo skill review` shows skill candidates pending approval.
`argo skill quality` (#886) shows aggregate scores. Missing: a
chronological view of WHEN each skill was actually invoked, which
worker invoked it, and what happened.

Add:
- `argo skill runs <skill-name> [--worker <id>] [--since <duration>]
  [--limit N] [--include-failed] [--json]` — chronological list of
  recent skill invocations newest-first; columns: timestamp, worker,
  decision (success/fail/skipped), latency_ms, action_id.
- `argo skill runs --tenant <id> [--limit N] [--json]` — flat
  cross-skill view for a tenant operator.

Reuses existing `worker_action` / `spend_ledger` joins (no new tables
needed). Pure read-side query helpers.

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-cli/src/handlers/skill_runs.rs` (NEW)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 1 pub mod
- `crates/argo-cli/src/cli_skill_search_cmds.rs` (or sibling) —
  narrowly: add `Runs` variant to existing `Skill` subcommand enum
- `crates/argo-store/src/skill_runs_query.rs` (NEW) — pure query
  helpers (runs_for_skill / runs_for_tenant)
- `crates/argo-store/src/lib.rs` — narrowly: re-export
- `crates/argo-cli/tests/skill_runs_smoke.rs` (NEW)

## Acceptance criteria
- [ ] `argo skill runs my-skill --limit 10` returns 10 newest
      invocations of my-skill, newest-first
- [ ] `--since 24h` filters correctly (wall-clock relative)
- [ ] `--include-failed` shows skipped + failed runs (default
      excludes them)
- [ ] `--worker <id>` filters to a single worker
- [ ] `argo skill runs --tenant t-1 --limit 50` cross-skill view
- [ ] `--json` envelope universal
- [ ] 5 unit tests + 1 CLI smoke
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `codex-skill-runs-cli`
- PR title: `skill: argo skill runs — chronological invocation history (codex-skill-runs-cli)`
