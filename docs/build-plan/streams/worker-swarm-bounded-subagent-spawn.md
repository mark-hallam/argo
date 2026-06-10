# worker-swarm-bounded-subagent-spawn

**Authored against**: origin/main `523b2cf45` on `2026-06-11`
**Validate before claim**: bash scripts/check-brief-freshness.sh worker-swarm-bounded-subagent-spawn

## Goal

Implement DEC-049 Stage 1: let a worker spawn a BOUNDED subagent swarm. Grant
`Spawn{Subagent}` (and only that) to workers whose manifest opts in, behind a
default-off flag, with depth + fan-out bounds that make the swarm finite. Closes
the superseded `fix-delegate-handoff-test-provision-spawn-capable-worker` (claim-2
delegate tests RED).

## Why (DEC-049)

Claim-2 delegation is impossible today: `WorkerCapabilityProfile::to_capability_set()`
only emits `ToolDispatch`, never `Spawn`, so no worker can pass `delegate_task`'s
`Spawn{Subagent}` requirement (`crates/argo-tool/src/tools/delegate_task.rs:116`).
DEC-041(D) deny-all fence is correct; the fix is a NARROW, bounded grant — see
`docs/skycot/decisions/DEC-049-bounded-worker-subagent-swarm.md`.

## Fix (Stage 1 — bounds + grant, default-off)

1. **`crates/argo-capability/src/worker_profile.rs`**
   - Add field `can_spawn_subagent: bool` (default false) to `WorkerCapabilityProfile`.
   - Add builder `with_subagent_spawn(mut self, yes: bool) -> Self`.
   - `to_capability_set()` pushes `Capability::Spawn { spawn_kind: SpawnKind::Subagent }`
     iff `self.can_spawn_subagent`. NEVER Subprocess/Container — those are not
     reachable from any profile.
   - `derive()` sets `can_spawn_subagent: false`; the field is set only via the
     builder (keep `derive`'s 4-arg signature — additive, no caller break).

2. **Flag + bounds constants** (small new module, e.g.
   `crates/argo-agent-runner/src/swarm_bounds.rs`, or in delegate_task):
   - `swarm_enabled() -> bool` reads `ARGO_WORKER_SWARM` (mirror
     `boot_spawns_duty::duty_autofire_enabled`; default OFF, accept 1/true/yes).
   - `pub const MAX_DELEGATION_DEPTH: usize = 4;`
   - `pub const MAX_DELEGATION_FANOUT: usize = 8;`

3. **`crates/argo-agent-runner/src/turn_vertical_fence.rs` →
   `derive_profile_from_manifest`**: set the spawn opt-in —
   `.with_subagent_spawn(swarm_enabled() && allowed.contains("delegate_task"))`.
   (Manifest grants delegation by listing `delegate_task` in `[tools].allowed`;
   the flag must ALSO be on. Both required → grant is doubly default-off.)

4. **`crates/argo-tool/src/tools/delegate_task.rs` → `dispatch`** — bounds run
   ALWAYS (independent of the grant), AFTER `inherit_active_chain` builds the
   chain, BEFORE `insert_handoff`:
   - **Depth**: the handoff's escalation chain length > `MAX_DELEGATION_DEPTH`
     → `error_text` refusal ("delegation depth N exceeds max 4").
   - **Fan-out**: count active (Pending/InProgress) outbound handoffs for
     `ctx.session_id` via the store; >= `MAX_DELEGATION_FANOUT` → refusal.

## May touch
- `crates/argo-capability/src/worker_profile.rs` (+ `worker_profile_tests.rs`)
- `crates/argo-agent-runner/src/turn_vertical_fence.rs`, new `swarm_bounds.rs`
- `crates/argo-tool/src/tools/delegate_task.rs` (+ its tests)
- `crates/argo-agent-runner/tests/delegate_task_handoff_chain_runtime.rs`
  (provision a spawn-capable worker so the claim-2 tests pass)
- `docs/build-plan/KNOWN_FLAKES.md` (remove the 2-delegate-test entry)
- `docs/build-plan/BUILD_STATUS.md` (flip the superseded brief)

## Acceptance / verifier
1. Profile whose `allowed_tools` includes `delegate_task`, built with
   `ARGO_WORKER_SWARM=1`, emits `Spawn{Subagent}` (a unit assert on
   `to_capability_set().covers(Spawn{Subagent})`); WITHOUT the flag → does NOT.
2. No profile EVER emits `Spawn{Subprocess}` or `Spawn{Container}` (unit assert).
3. Depth > 4 and fan-out >= 8 each yield a `delegate_task` refusal (unit/integration).
4. `cargo test -p argo-agent-runner --test delegate_task_handoff_chain_runtime`
   PASSES (claim-2 restored via the real grant path, NOT a fence bypass).
5. Flag default-off: with `ARGO_WORKER_SWARM` unset, a delegate-capable manifest
   still yields NO `Spawn` — delegation denied (regression that the grant cannot
   leak on by default).
6. Full pre-merge gate green; KNOWN_FLAKES entry removed.

## Size / owner
~200-300 LOC across capability + agent-runner + tool + tests. Owner: claude
(conductor-built, operator-directed 2026-06-11). 🟢 P1 — closes claim-2's RED
regression + ships the operator-requested swarm capability, safely bounded +
default-off.
