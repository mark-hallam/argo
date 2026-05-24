# docs-contributing-agent-live-ops-refresh

**Owner**: codex.
**Depends on**: none.
**Authored against**: origin/main `a9675bad2` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh docs-contributing-agent-live-ops-refresh`.

P3 docs hygiene. `docs/build-plan/CONTRIBUTING_AGENT.md` is the short
worker/conductor companion, but it has not been refreshed since before
the current conductor scripts and handoff checks landed. Keep it short;
do not turn it into a copy of `SWARM_PROTOCOL.md`.

## Scope

Refresh `CONTRIBUTING_AGENT.md` so a fresh worker or reviewer sees the
current live-ops shape: freshness checks before claiming, codex
pre-push gate, conductor-only mutation authority, unread handoffs,
heartbeats, gateway-freshness, findings fix-by, todo aging, and brief
block-history guard. The doc must still say that `SWARM_PROTOCOL.md`
and `BUILD_STATUS.md` are authoritative when anything conflicts.

This is documentation-only. Do not edit the live protocol text except
through a separate conductor/process brief.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `docs/build-plan/CONTRIBUTING_AGENT.md` - refresh the short companion only; keep it under 150 LOC if practical.

## Files you must NOT touch

- `docs/build-plan/SWARM_PROTOCOL.md` - source of truth, not part of this doc refresh.
- `docs/build-plan/PROTOCOL_LOOP_PROMPT.md` - conductor prompt changes need their own process brief.
- `docs/build-plan/BUILD_STATUS.md` except the standard PR-number row update.
- Anything not on the may-touch list above.

## Acceptance criteria

- [ ] The doc names `BUILD_STATUS.md` + `SWARM_PROTOCOL.md` as authoritative over this companion.
- [ ] The worker flow mentions `scripts/check-brief-freshness.sh` before implementation and the codex pre-push gate where applicable.
- [ ] The conductor role mentions unread handoffs, heartbeat refresh, gateway-freshness, fix-by, todo-aging, BLOCKED triage, queue replenishment, and PR review/merge.
- [ ] The queue-replenishment section mentions `scripts/check-brief-block-history.sh` before authoring/inserting a row.
- [ ] No new operational rule conflicts with `SWARM_PROTOCOL.md` or `PROTOCOL_LOOP_PROMPT.md`.
- [ ] `rg -n 'check-brief-freshness|check-unread-handoffs|check-agent-heartbeats|check-gateway-freshness|check-findings-only-fix-by|check-todo-aging|check-brief-block-history' docs/build-plan/CONTRIBUTING_AGENT.md` finds all expected script names.
- [ ] `bash scripts/check-brief-freshness.sh docs-contributing-agent-live-ops-refresh` passes.
- [ ] Every modified file remains under 500 LOC.

## Branch + PR

- Branch: `codex-docs-contributing-agent-live-ops-refresh`
- PR title: `docs(build-plan): refresh contributor companion for live ops (docs-contributing-agent-live-ops-refresh)`

## Notes for the implementer

- Prefer concise bullets over protocol duplication.
- If a rule seems stale, cite the live protocol rather than rewriting it here.
