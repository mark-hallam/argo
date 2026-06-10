# fix-delegate-handoff-test-provision-spawn-capable-worker

Authored against: origin/main 3ec2c9ec81407714454212b825ea7eae82883e1d on 2026-06-10
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-delegate-handoff-test-provision-spawn-capable-worker

## Goal

Restore the claim-2 ("Delegation is the unit") regression coverage. Two tests in
`crates/argo-agent-runner/tests/delegate_task_handoff_chain_runtime.rs` fail
DETERMINISTICALLY on clean `origin/main` and have since the capability fence
tightened — they need to provision a Spawn-capable worker so `delegate_task` is
allowed under the fence, not denied.

## Verified root cause (triage 2026-06-10, conductor)

Both `delegate_task_without_inbound_handoff_leaves_chain_unset` and
`delegate_task_inherits_active_inbound_handoff_chain` panic at
`delegate_task_handoff_chain_runtime.rs:219` (`.expect("delegated handoff")`).
Instrumented run on clean main: the lookup finds **0 handoffs** under the test's
(tenant, session) — the handoff is never written.

NOT a flake (deterministic, single-threaded, with `SKYCOT_DB_KEY_BASE64` set).
NOT a code regression. The chain:

- The fixture creates a **worker-bound** session (`worker_id: Some(receiver)`,
  `vertical_id: None`) and provisions **no** capability profile.
- `dispatch.rs:146-166` — the `None if is_worker_session` branch (DEC-041 (D),
  commit `107b4e6b1` 2026-06-07; deny-by-default profile `60b80e7d6` 2026-05-31)
  fences a profile-less worker session to a **deny-all** `WorkerCapabilityProfile`
  (empty allow-sets). Both commits landed AFTER the test (#2256, 2026-05-25), so
  it passed when authored — a profile-less worker then fell through to the
  `None =>` branch and bound `owner` (wide-open).
- `delegate_task` requires `Capability::Spawn { spawn_kind: Subagent }`
  (`crates/argo-tool/src/tools/delegate_task.rs:116-120`). The deny-all profile
  grants none, so the tool is denied; the mock still returns "delegated" on the
  2nd call (so `result.text == "delegated"` and `calls == 2` pass), but
  `insert_handoff` never runs → 0 handoffs → panic.

The fence is correct-by-design: under DEC-041 (D) a worker may delegate (spawn a
subagent) ONLY if its profile explicitly grants `Spawn`. The TEST is stale.

## Fix (interpretation A — the principled reading; see design note)

Provision the fixture's worker session with a capability profile that grants
`Spawn { Subagent }`, mirroring how a real delegating worker is set up:

- Resolve a profile via the existing genesis-fence path: write a genesis vertical
  manifest under the test tenant's `.genesis/<sanitize_tenant(tenant)>/<vid>/`
  whose capability spec grants `spawn` (see how
  `turn_vertical_fence::resolve_worker_profile_for_session` /
  `resolve_worker_profile_in` derive a `WorkerCapabilityProfile` from the
  manifest), and create the session with that `vertical_id: Some(vid)` +
  genesis provenance so `resolve_worker_profile_for_session` returns a
  Spawn-granting profile instead of `None`.
- Keep the two tests' assertions otherwise unchanged (chain unset vs. inherited).
  The point is only to lift the worker out of deny-all so the handoff is written.

Grep before write: do NOT add a new parallel capability/profile helper — reuse
`WorkerCapabilityProfile` + the genesis manifest resolution already used in
`turn_vertical_fence.rs`.

## May touch

- `crates/argo-agent-runner/tests/delegate_task_handoff_chain_runtime.rs`
- a test-only manifest-writer helper IF one is not already reusable
- `docs/build-plan/KNOWN_FLAKES.md` (remove the two-test entry when this lands)

## Acceptance / verifier

1. `cargo test -p argo-agent-runner --test delegate_task_handoff_chain_runtime`
   → both tests PASS on clean main (the handoff is written; chain-unset and
   chain-inherited assertions hold).
2. The fix provisions the capability via the REAL fence path (a Spawn-granting
   profile), NOT by disabling/bypassing the DEC-041 (D) fence — demonstrate the
   worker is genuinely fenced-but-Spawn-capable, so the test re-proves claim-2
   under the fence rather than around it.
3. Remove the KNOWN_FLAKES.md entry in the same PR.

## Design note for the operator (first-principles)

This brief assumes **interpretation A**: the fence is right, and "delegation is
the unit" means delegation is a capability-gated primitive — a worker delegates
only when granted `Spawn`. That is consistent with DEC-041 (D)'s purpose (stop a
worker from spawning subagents / reaching mutating tools by default).
**Interpretation B** would be that a worker should delegate by default and the
deny-all is too broad — that would be a CODE change (add delegate to the minimal
teammate toolset, or grant Spawn to bare workers) and weakens the fence. Recommend
A. If the operator chooses B, re-scope this brief to the fence/minimal-toolset.

## Size / owner

~40-100 LOC test change. Owner: claude. 🟡 P2 — claim-2 ("Delegation is the unit")
is badged LIVE in MISSION.md / FEATURE_MATRIX.md but its regression tests are
currently RED; this restores honest coverage (NO THEATER / DEC-029).
