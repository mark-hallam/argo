# Stream H (follow-up) — `argo worker list` + enhanced `show`

**Owner**: codex. **Depends on**: E-identity (#781),
claude-worker-instance-policy (#815) merged.

## Scope

Workers exist (Stream E) and can have per-instance policies
(#815). But operators have no CLI to LIST all workers in a
tenant or SEE their resolved policy at a glance.

Subcommands:
- `argo worker list --tenant <id> [--persona <name>]` →
  table of workers with id, persona, role.charter (truncated),
  auto_approval.enabled, created_at
- `argo worker show <worker_id>` → full Worker JSON + the
  resolved policy (per `Worker::resolved_policy(workspace_default)`)
- `argo worker policy show <worker_id>` → just the resolved
  policy, suitable for piping into `jq`

`--json` on every subcommand.

## Files you may touch
- `crates/argo-cli/src/cli_worker_cmds.rs` — narrowly:
  extend with `List`, `Show`, `Policy(...)` variants
- `crates/argo-cli/src/handlers/worker.rs` — extend (do NOT
  replace existing handlers; ADD new ones)
- `crates/argo-cli/src/handlers/worker_list.rs` (NEW)

## Files you must NOT touch
- `crates/argo-identity/**` — read-only consumer
- The worker_update handler (from #815)

## Acceptance criteria
- [ ] All 3 subcommands wired
- [ ] `--tenant <id>` required (operators must scope queries)
- [ ] `show` resolves the worker's policy against the
      workspace default (loads `.skycot/auto_approval.toml`)
- [ ] `policy show` outputs the resolved policy as JSON ready
      to pipe into `argo worker update --auto-approval-json -`
- [ ] `--json` flag on every subcommand
- [ ] Integration test seeding 3 workers + filtering by
      persona
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean
- [ ] Every authored file < 500 LOC

## Branch + PR
- Branch: `codex-H-worker-list-cli`
- PR title: `cli: argo worker list/show + policy show (H-worker-list-cli)`

## Notes
- Mirror `argo budget show <worker_id>` from #820 for the
  table shape
- Truncate role.charter to 60 chars in list view; full text in show
