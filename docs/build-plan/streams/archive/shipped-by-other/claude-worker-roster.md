# V1.3 — Worker rosters (groups + team policy)

**Owner**: claude. **Depends on**: E-identity (#812),
H-worker-list-cli (#837) merged.

## Scope

Today every worker is policy-resolved individually. Real
teams want **group-level policy** — "all workers in Sales
Ops auto-approve emails ≤$1, escalate ≥$5; weekend quiet
hours". This brief introduces a Roster primitive: a named
group of workers with a shared AutoApprovalPolicy + shared
NotifyChannel preferences + shared budget cap.

Per-worker policy resolution becomes:
1. Worker explicit policy (highest precedence)
2. Roster the worker belongs to
3. Workspace default

If a worker is in multiple rosters, conflict resolution is
"most restrictive wins" (smallest spend cap, fewest
auto-approve allowances, strictest channel rules).

- New `Roster { id, name, member_worker_ids: Vec<WorkerId>,
  policy: AutoApprovalPolicy, channels:
  Vec<NotifyChannelRef>, budget_cap_usd: Option<f64> }`
- New tables: `roster`, `roster_member` (many-to-many)
- `argo roster create/list/show/add-member/remove-member/
  delete`
- `argo worker show` includes resolved roster set + final
  policy lineage
- Existing `Worker::resolved_policy()` walks the new
  roster layer

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- EXHAUSTIVE-MATCH amendment applies
- `crates/argo-identity/src/roster.rs` (NEW)
- `crates/argo-identity/src/lib.rs` — narrowly: re-export
- `crates/argo-identity/src/types.rs` — narrowly: hook
  Worker::resolved_policy through roster layer
- `crates/argo-store/migrations/<next>__roster.sql` (NEW —
  2 tables)
- `crates/argo-store/src/roster.rs` (NEW)
- `crates/argo-cli/src/handlers/roster_*.rs` (5 NEW —
  create/list/show/add_member/remove_member)
- `crates/argo-cli/src/cli_roster_cmds.rs` (NEW)
- `crates/argo-cli/src/cli_subcommands.rs` — narrowly:
  register `Roster` subcommand
- `crates/argo-cli/src/handlers/worker_show.rs` — narrowly:
  surface roster lineage in worker_show output

## Acceptance criteria
- [ ] Roster CRUD persisted
- [ ] Worker.resolved_policy walks roster layer
- [ ] Multi-roster "most-restrictive" conflict resolution
- [ ] `argo worker show` includes lineage
- [ ] `--json` universal across all 5 subcommands
- [ ] 14 unit tests + 2 integration tests
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR
- Branch: `claude-worker-roster`
- PR title: `identity: worker rosters with team-level policy (claude-worker-roster)`
