# Mission load-bearing wire — `delegate_task` inherits caller's fallback chain

**Owner**: claude. **Depends on**: `argo-tool::tools::delegate_task` exists (it does — `crates/argo-tool/src/tools/delegate_task.rs`); `EscalationChain` shape stable (it is — `argo-orchestrator::escalation`).
**Authored against**: origin/main `0742cab0` on `2026-05-10`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-delegate-task-fallback-chain`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 2 item #56 ("Add `delegate_task` inheritance of fallback chain"). Today, when worker A delegates a task to worker B via `delegate_task`, the resulting handoff carries no `escalation_chain` — so if B fails to respond before the deadline, the handoff exhausts immediately to `escalation_exhausted` instead of walking back up A's escalation policy. Mission Claim 2 ("delegation as a unit") implies the unit's accountability is preserved; that requires the chain.

## Scope

`delegate_task` (in `crates/argo-tool/src/tools/delegate_task.rs`) creates a Pending handoff to a target worker and returns the handoff id. The brief: when invoked, the tool reads the **caller's** active escalation context (the handoff or session that the calling worker is currently servicing) and inherits its `EscalationChain` + `escalation_index` into the new handoff's payload, with the new step list expanded by ONE prefix entry: `Worker(callee_id)` — so when the callee misses the deadline, escalation continues from there.

If the caller has no active escalation context (e.g. the worker is running a top-level mission with no inbound handoff), the new handoff inherits the caller's vertical-default escalation policy (read via `argo-identity::policy::resolve_effective_policy`). If neither is available, the new handoff carries an empty chain (today's behavior — preserved as the fallback).

This is NOT in scope: deep-copy of arbitrary handoff metadata; modifying `OrchestratorStore::insert_handoff` signature; changing `EscalationChain::advance` semantics; building a per-worker policy override store.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-tool/src/tools/delegate_task.rs` — extend the handoff builder: read caller's active escalation context, prefix `Worker(callee_id)` onto the chain, set `escalation_index = 0` on the new handoff. Add `fn inherit_chain(ctx: &ToolContext, callee: &WorkerId) -> Option<(EscalationChain, usize)>` helper.
- `crates/argo-tool/src/tools/delegate_task_tests.rs` (NEW if absent; if present, append) — 5 unit tests: inherits-from-active-handoff; inherits-from-active-session; falls-back-to-policy-default; falls-back-to-empty; prefix-of-Worker-callee-is-applied.
- `crates/argo-tool/src/tools/mod.rs` — narrowly scoped: ONE `pub(crate) mod delegate_task_tests;` line if a NEW tests module is added. No other edits.
- `crates/argo-tool/Cargo.toml` — narrowly scoped: confirm `argo-identity` is in `[dependencies]` (it should be, transitively). If not, add ONE line.
- `crates/argo-tool/tests/delegate_task_chain_smoke.rs` (NEW) — integration smoke: create caller worker with a 2-step chain, invoke `delegate_task`, observe the new handoff carries the prefix-extended chain.

## Files you must NOT touch

- `crates/argo-orchestrator/**` — chain semantics + walker behavior are upstream; do not modify
- `crates/argo-identity/src/policy.rs` — `resolve_effective_policy` is the upstream helper; reuse, don't modify
- `crates/argo-store/migrations/**` — no schema change
- Any other tool in `argo-tool/src/tools/`

## Acceptance criteria

- [ ] `delegate_task.rs` reads `ToolContext` for the caller's active handoff (if any) and inherits its `escalation_chain` + `escalation_index`; prefixes `Worker(callee_id)` to the chain
- [ ] When no active handoff, falls back to the caller's `resolve_effective_policy(...).escalation_chain` if any
- [ ] When neither is available, the new handoff carries an empty chain (preserving today's behavior — no regression)
- [ ] `inherit_chain` helper isolated and unit-testable; returns `Option<(EscalationChain, usize)>` so the call site stays small
- [ ] 5 unit tests in `delegate_task_tests.rs` cover the inheritance matrix
- [ ] 1 integration smoke in `tests/delegate_task_chain_smoke.rs` exercises end-to-end: caller has chain → delegate → callee handoff has prefix-extended chain
- [ ] No new public API on `argo-tool` outside the existing tool exports (`inherit_chain` is private to the module)
- [ ] `cargo build -p argo-tool` clean
- [ ] `cargo clippy -p argo-tool --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-tool` passes (skip pre-existing flakes per KNOWN_FLAKES.md)
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-delegate-task-fallback-chain`
- PR title: `tool: delegate_task inherits caller's escalation chain (mission-delegate-task-fallback-chain)`

## Notes for the implementer

- The `ToolContext` already carries enough to resolve the caller's active handoff in `argo-tool` — see the `WorkerId`/session_id/handoff_id fields. If the active-handoff lookup is not yet exposed, prefer adding a narrow `pub(crate) fn active_escalation_chain(&self) -> Option<(EscalationChain, usize)>` accessor on `ToolContext` over plumbing more state into `delegate_task`.
- Prefix semantics matter: the new chain MUST be `[Worker(callee), ...inherited_chain]`, NOT `[...inherited_chain, Worker(callee)]`. The walker advances index forward, so the prefix order ensures the first miss escalates to the next step in the inherited chain (which is what the unit promised).
- Test fixture: build the inherited chain via `EscalationChain::from(vec![EscalationStep::Worker(...)])`, not by hand-rolling JSON. The serialization round-trip is already covered upstream.
- If `resolve_effective_policy` returns None for the policy fallback path, that's fine — the empty-chain fallback handles it. Don't error.
