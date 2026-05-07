# V1.1 — Org-chart-aware team roster CLI

**Owner**: claude. **Depends on**: org_chart migration 21 + roster
migration 22 present on main.

## Why

Migrations 21 (`org_chart`) and 22 (`roster`) shipped during the swarm
but neither has a CLI surface — the tables are populated by genesis
templates only. Operators can't see who's on a team, can't add a new
hire to a team, and can't restructure as the org grows.

Mission claim 2 is about delegation. Delegation needs *teams*: a
"finance team" of (AR clerk, AP clerk, accountant) lets the operator
say "the finance team handles vendor onboarding" instead of routing to
individual workers.

## Scope

- CLI:
  - `argo team list [--format markdown|json]` — all teams + member
    counts
  - `argo team show <team-id> [--format markdown|json]` — team
    metadata + nested members + each member's level + current charter
    summary
  - `argo team add <team-id> <worker-id> [--role <role>]` — insert
    into roster
  - `argo team remove <team-id> <worker-id>` — soft delete
    (`removed_at` timestamp; preserves history)
  - `argo team rename <team-id> <new-name>`
  - `argo team create <name> [--parent <team-id>]` — new team in
    org_chart
- All commands respect existing migration 21/22 schemas — read existing
  table shapes via `sqlite3 .argo/store.db .schema org_chart` first.

## Approach

1. `crates/argo-store/src/team_roster.rs` (NEW) — typed wrappers
   around existing `org_chart` + `roster` tables. ~200 LOC.
2. `crates/argo-cli/src/handlers/team_roster.rs` (NEW, ≤ 420 LOC) —
   handlers for all 6 subcommands + markdown formatter.
3. Soft-delete via `removed_at` column — if not present in current
   `roster`, add via migration `42__roster_removed_at.sql`.

## Files you may touch

- BLANKET CLI ALLOWLIST applies
- EXHAUSTIVE-MATCH amendment applies
- `crates/argo-store/migrations/42__roster_removed_at.sql` (NEW —
  IF the column doesn't already exist; check the migration 22 file
  before adding)
- `crates/argo-store/src/team_roster.rs` (NEW)
- `crates/argo-store/src/lib.rs` — narrowly: pub mod
- `crates/argo-cli/src/handlers/team_roster.rs` (NEW)
- `crates/argo-cli/src/cli.rs` — narrowly: `Team` top-level subcmd
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 1 pub mod line

## Files you must NOT touch

- Existing migrations 21/22 — additive only via NEW migration if
  needed
- Worker struct — team membership lives in roster, not on worker
- Genesis templates — no template changes; CLI is operator-facing

## Acceptance criteria

- [ ] `argo team list` shows team name, parent, member count
- [ ] `argo team show <id>` markdown lists members + level + charter
      one-liner per member
- [ ] `argo team add` inserts roster row; idempotent on existing
      (worker, team) pair → updates role only
- [ ] `argo team remove` sets `removed_at` (not hard delete)
- [ ] `argo team rename` updates org_chart row
- [ ] `argo team create --parent <id>` validates parent exists
- [ ] 12 unit tests + 4 CLI smoke + 1 markdown snapshot
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-worker-team-roster-cli`
- PR title: `team: roster + org-chart CLI (claude-worker-team-roster-cli)`

## Notes

- **First step for the implementer**: read migration 21 + 22 source to
  pin down the exact column names and types before writing the
  `team_roster.rs` queries.
- Charter one-liner in `team show`: read first 80 chars of `charter_md`
  from the worker, ellipsis if longer.
- Soft delete is intentional — reviews / audit / KPI history reference
  worker_id, hard-delete would create dangling FKs.
