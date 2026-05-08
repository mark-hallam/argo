# Agent Coordination Rules

This file is a short companion for agents and humans. The live protocol
for every coding platform lives in `SWARM_PROTOCOL.md` (in this same
directory). If this file conflicts with `SWARM_PROTOCOL.md`, follow
`SWARM_PROTOCOL.md`.

## Shared Model

- `BUILD_STATUS.md` is the global ledger for parent work.
- `streams/<brief-id>.md` is the parent brief spec.
- `work-packages/<brief-id>.md` is optional child state for a
  manager-worker brief.
- PRs carry implementation review and final integration discussion.
- The conductor is the only role that merges and arbitrates global
  conflicts.

## Agent Families

`agent_family` is provenance and sizing signal, not authority. Current
families are `claude` and `codex`; future families should use their own
branch prefix and the same protocol roles.

## Roles

- **conductor** — global priority, PR review/merge, queue
  replenishment, stale package sweeps, and BLOCKED triage.
- **worker** — one normal parent brief through PR creation.
- **manager_worker** — one parent brief, bounded child decomposition,
  integration, tests, and one final parent PR.
- **subworker** — one child task inside a manager package; no global
  ledger edits and no final PR.
- **reviewer** — read-only diff/package checker.

## Default Worker Flow

1. Sync main and read `SWARM_PROTOCOL.md`.
2. Read `BUILD_STATUS.md`.
3. Claim a TODO parent row according to the live protocol.
4. Read `streams/<brief-id>.md`.
5. Implement inside the may-touch list.
6. Run the required verification from `SWARM_PROTOCOL.md` and the brief.
7. Open a PR.
8. Mark the parent row `IN_REVIEW`.

## Manager-Worker Flow

Use only when the parent brief says `Manager-worker allowed: yes`.

1. Claim the parent row.
2. Create `work-packages/<brief-id>.md` from
   `PROTOCOL_MANAGER_PACKAGE_TEMPLATE.md`.
3. Keep child files inside the parent may-touch list.
4. Keep child state out of `BUILD_STATUS.md`.
5. If the platform supports subagents, satisfy `spawn_requests`
   automatically; otherwise leave them pending for a wrapper or human.
6. Integrate child outputs into one parent branch and one final PR.
7. Link the package from the PR body.

## Non-Negotiables

- Never force-push to main.
- Never modify another agent's branch.
- Never expand may-touch without conductor approval.
- Never let subworkers edit `BUILD_STATUS.md`.
- Never skip the acceptance checklist.
- Never merge your own work.
