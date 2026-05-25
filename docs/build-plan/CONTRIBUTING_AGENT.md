# Agent Coordination Rules

This is the short companion for agents and humans. `BUILD_STATUS.md`
is the live ledger, and `SWARM_PROTOCOL.md` is the live operating
protocol. If this file conflicts with either, follow those sources.
For conductor ticks, `PROTOCOL_LOOP_PROMPT.md` is the detailed prompt.

## Shared Model

- `BUILD_STATUS.md` tracks parent brief state and current operator
  amendments.
- `streams/<brief-id>.md` is the parent brief spec and acceptance
  contract.
- `work-packages/<brief-id>.md` is optional child state for a
  manager-worker brief.
- PRs carry implementation review and final integration discussion.
- The conductor is the only role that merges, arbitrates global
  conflicts, authors queue rows, and owns BLOCKED triage.

## Agent Families

`agent_family` is provenance and sizing signal, not authority. Current
families are `claude` and `codex`; future families should use their own
branch prefix and the same protocol roles.

## Roles

- **conductor** — global priority, PR review/merge, BLOCKED triage,
  queue replenishment, stale package sweeps, and live-ops checks.
- **worker** — one normal parent brief through PR creation and its own
  ledger row update.
- **manager_worker** — one parent brief, bounded child decomposition,
  integration, tests, and one final parent PR.
- **subworker** — one child task inside a manager package; no global
  ledger edits and no final PR.
- **reviewer** — read-only diff/package checker.

## Default Worker Flow

1. Sync `main`, read `BUILD_STATUS.md`, then read `SWARM_PROTOCOL.md`.
2. Pick the first eligible `TODO` row whose dependencies are satisfied.
3. Run `bash scripts/check-brief-freshness.sh <brief-id>` before
   claiming and handle any drift before implementation.
4. Claim through the live protocol, read `streams/<brief-id>.md`, then
   implement only inside the brief's may-touch list.
5. Run the brief checks plus the crate-scoped verification required by
   `SWARM_PROTOCOL.md`.
6. Codex workers must also run `bash scripts/codex-pre-push.sh` before
   pushing a feature branch.
7. Open the PR, then mark the parent row `IN_REVIEW` with the PR
   number.

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

## Conductor Live Ops

The conductor follows `PROTOCOL_LOOP_PROMPT.md`; this list is only a
pointer to the current checks, not a replacement for that prompt.

- Start from current `main` and the live ledger.
- Check unread handoffs with `scripts/check-unread-handoffs.sh`.
- Refresh and inspect agent heartbeat state with
  `scripts/check-agent-heartbeats.sh`.
- Check running-gateway drift with `scripts/check-gateway-freshness.sh`.
- Check findings deadlines with `scripts/check-findings-only-fix-by.sh`.
- Check stale TODOs with `scripts/check-todo-aging.sh`.
- Review and merge/route open PRs; never ask workers to merge their own
  work.
- Triage `BLOCKED` rows and stale claims according to the live protocol.
- Replenish queues when required by `BUILD_STATUS.md`.

## Queue Replenishment

Before authoring or inserting a new row, run
`scripts/check-brief-block-history.sh <brief-id>` and do not recreate a
currently blocked or already resolved brief. New queue rows still need
a stream brief, may-touch list, freshness command, acceptance checklist,
and dependency statement.

## Non-Negotiables

- Never force-push to main.
- Never modify another agent's branch.
- Never expand may-touch without conductor approval.
- Never let subworkers edit `BUILD_STATUS.md`.
- Never skip the acceptance checklist.
- Never merge your own work.
