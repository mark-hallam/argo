# ops-unread-handoffs-action-lint

**Owner**: codex.
**Depends on**: none.
**Authored against**: origin/main `52dc49988` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh ops-unread-handoffs-action-lint`.

🟡 **P2 conductor-noise reduction.** Source: this tick's unread evaluator handoffs mixed true new work, already-landed work, and operator-only follow-through. The existing unread-handoff checker only blocks; it does not help the conductor classify what is already actioned.

## Scope

Add a helper that reads unread SHARED handoff entries after the current conductor cutoff and prints an action summary:

- `already-landed` when the handoff's cited brief/PR is now MERGED or IN_REVIEW
- `queue-needed` when it surfaces a new brief id with no ledger row
- `operator-only` when the write target is explicitly outside the repo
- `informational` otherwise

This is a read-only classifier layered on top of the existing blocker script.

## Files you may touch

- `scripts/classify-unread-handoffs.sh` (NEW, ≤280 LOC)
- `scripts/tests/classify-unread-handoffs.bats` (NEW, ≤220 LOC)
- `docs/build-plan/PROTOCOL_LOOP_PROMPT.md` — narrowly: mention the helper after `check-unread-handoffs.sh`

## Files you must NOT touch

- `scripts/check-unread-handoffs.sh`
- `docs/skycot/agents/SHARED.md` except via test fixtures
- Feature crates under `crates/`

## Acceptance criteria

- [ ] Helper consumes the same conductor cutoff timestamp used by the unread checker
- [ ] Helper recognizes ledger/PR keywords in handoff text and classifies at least the three categories above
- [ ] Tests cover landed, queue-needed, operator-only, and informational cases
- [ ] Prompt docs mention the helper as a follow-on to the blocking check
- [ ] Every authored file <500 LOC

## Branch + PR

- Branch: `codex-ops-unread-handoffs-action-lint`
- PR title: `ops: classify unread handoff action states (ops-unread-handoffs-action-lint)`
