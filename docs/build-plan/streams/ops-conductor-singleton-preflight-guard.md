# ops-conductor-singleton-preflight-guard

**Owner**: codex.
**Depends on**: none.
**Authored against**: origin/main `52dc49988` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh ops-conductor-singleton-preflight-guard`.

🔴 **P1 conductor-safety guard.** Source: conductor hard rule + this tick's live heartbeat check. The prompt says "ensure exactly one conductor across machines and stop/escalate if another conductor heartbeat younger than yours exists", but the repo has no dedicated preflight helper to enforce that before BUILD_STATUS or merge mutations.

## Scope

Add a small repo-tracked preflight helper that the conductor can run before mutations:

1. Read `docs/skycot/agents/heartbeats/conductor.json`.
2. Compare `last_run_utc` against current UTC and an optional `--candidate-now <iso>`.
3. Exit nonzero when another conductor heartbeat is still within cadence/grace OR is newer than the candidate run.
4. Print a short machine-readable reason (`alive`, `newer`, `stale`, `missing`, `invalid`).
5. Document the callsite in the conductor loop prompt so future ticks use the helper instead of ad-hoc JSON parsing.

## Files you may touch

- `scripts/check-conductor-singleton.sh` (NEW, ≤220 LOC)
- `scripts/tests/check-conductor-singleton.bats` (NEW, ≤220 LOC)
- `docs/build-plan/PROTOCOL_LOOP_PROMPT.md` — narrowly: conductor preflight invocation note
- `docs/build-plan/SWARM_PROTOCOL.md` — narrowly: mention the helper in conductor role guidance

## Files you must NOT touch

- `docs/build-plan/BUILD_STATUS.md` except the standard row flip
- Any worker prompt adapters
- Feature crates under `crates/`

## Acceptance criteria

- [ ] Helper exits `0` for missing/stale heartbeat and `1` for live/newer heartbeat
- [ ] `--candidate-now` makes the check testable without wall-clock sleeps
- [ ] Invalid JSON / timestamp prints a clear failure reason and exits nonzero
- [ ] Bats coverage includes stale, live, newer-than-candidate, missing, and malformed cases
- [ ] Prompt docs mention the helper before conductor mutations
- [ ] Every authored file <500 LOC

## Branch + PR

- Branch: `codex-ops-conductor-singleton-preflight-guard`
- PR title: `ops: add conductor singleton preflight guard (ops-conductor-singleton-preflight-guard)`
