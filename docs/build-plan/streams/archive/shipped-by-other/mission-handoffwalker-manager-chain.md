# Mission load-bearing wire — HandoffWalker consults org_chart manager_id chain on chain-exhaust

**Owner**: claude. **Depends on**: org chart store + reports_to_chain helper merged (already on main; see `argo-identity::org::OrgStore::reports_to_chain`).
**Authored against**: origin/main `0742cab0` on `2026-05-10`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-handoffwalker-manager-chain`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 2 item #52 ("Wire org chart `manager_id` chain into HandoffWalker escalation"). The walker today only walks the explicit `EscalationChain` declared on a handoff payload; when that chain exhausts, the parent session goes to `SessionStatus::Expired` with an `escalation_exhausted` audit row and the human is never reached. Org-chart fallback closes the Claim 2 ("delegation as a unit") promise that escalation walks toward a real manager when no explicit chain exists.

## Scope

`HandoffWalker::sweep_once` in `crates/argo-orchestrator/src/walker.rs` advances overdue handoffs along the `EscalationChain` carried in the handoff payload. When the chain is exhausted (or absent), it writes an `escalation_exhausted` audit row and flips the session to Expired. This brief teaches the walker to consult the org chart **before giving up**: if the originating worker has an `org_member` row with a non-null `manager_id`, the walker walks the manager chain via `OrgStore::reports_to_chain` and emits a fresh Pending handoff to the next manager up. Only when the manager chain is also exhausted (no parent, or the topmost member) does the walker write `escalation_exhausted`.

This is a wire, not a redesign: no schema change, no new audit kind. The existing `EscalationStep::Worker(id)` shape is reused — when the manager chain produces a step, the walker emits the same shape it would for a declared chain, so downstream handlers and audit semantics are identical.

This is NOT in scope: building a UI for the manager chain, changing the deadline policy, supporting `Human(addr)` nodes injected from the org chart (those still come from the explicit chain), or modifying `OrgStore::reports_to_chain` itself.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-orchestrator/src/walker.rs` — extend `sweep_once`'s exhaust path: before writing `escalation_exhausted`, attempt `try_advance_via_org_chart(...)`. If it produces a step, emit the new handoff and return; if not, fall through to existing exhausted behavior.
- `crates/argo-orchestrator/src/walker_helpers.rs` — add `pub(crate) async fn try_advance_via_org_chart(pool: &SqlitePool, tenant: &TenantId, originator: &WorkerId, current_index: usize) -> ArgoResult<Option<EscalationStep>>` that calls `OrgStore::find_member_by_worker(...)` then `OrgStore::reports_to_chain(...)` and maps the `current_index`-th manager into `EscalationStep::Worker(WorkerId(member.worker_id))`.
- `crates/argo-orchestrator/src/walker_tests.rs` — narrowly scoped: add 4 new tests beneath the existing test module (org-chart fallback emits to manager; double-fallback walks two levels; topmost-manager exhausts cleanly; non-org-member worker falls straight through to existing exhaust path).
- `crates/argo-orchestrator/Cargo.toml` — narrowly scoped: confirm `argo-identity` is already in `[dependencies]`; if not, add ONE line. No other Cargo edits.
- `crates/argo-orchestrator/tests/handoff_policy_smoke.rs` — narrowly scoped: ONE new integration test asserting the org-chart fallback path end-to-end (seed 3-level org chart, exhaust explicit chain, observe handoff emitted to grand-manager).

## Files you must NOT touch

- `crates/argo-identity/src/org.rs` — `OrgStore::reports_to_chain` is the upstream helper; reuse, don't modify
- `crates/argo-store/migrations/**` — no schema change
- Any `argo-channels` file — `Human(addr)` notification path is unchanged
- Any `argo-cli` handler

## Acceptance criteria

- [ ] `try_advance_via_org_chart` lives in `walker_helpers.rs` and returns `Ok(Some(EscalationStep::Worker(...)))` when the originator has an org_member row with a chain to walk
- [ ] Returns `Ok(None)` when (a) the originator is not an org_member, or (b) the chain index is past the topmost manager
- [ ] `walker.rs` calls it on the chain-exhaust path BEFORE writing `escalation_exhausted` and emits a new Pending handoff if a step is produced (reusing the existing emission code path)
- [ ] When the org-chart chain produces a step, the new Pending handoff carries `escalation_index` advanced by 1 (so subsequent sweeps continue walking)
- [ ] When org-chart fallback produces a step, NO `escalation_exhausted` audit is written; only when both explicit AND org-chart chains are exhausted does `escalation_exhausted` fire
- [ ] 4 new unit tests in `walker_tests.rs` cover: fallback emits to direct manager; fallback walks past direct manager (index=1) to grand-manager; topmost-manager (root of org) writes exhausted; worker without org_member row writes exhausted with no probe to OrgStore
- [ ] 1 integration test in `tests/handoff_policy_smoke.rs` exercises the full path against a seeded 3-level org chart
- [ ] `cargo build -p argo-orchestrator` clean
- [ ] `cargo clippy -p argo-orchestrator --all-targets -- -D warnings` clean (worker runs; conductor verifies via Tier-1 PR-body claim)
- [ ] `cargo test -p argo-orchestrator` passes (skip pre-existing flakes per KNOWN_FLAKES.md)
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-handoffwalker-manager-chain`
- PR title: `orchestrator: HandoffWalker walks org_chart manager_id chain on exhaust (mission-handoffwalker-manager-chain)`

## Notes for the implementer

- The org-chart probe should be lazy: if `OrgStore::find_member_by_worker` returns None, skip `reports_to_chain` entirely (the `Ok(None)` early-return). This avoids per-tick load on tenants that never adopted the org chart.
- `escalation_index` is a usize already carried in the handoff payload via `parse_index_from_payload`. When emitting a fallback step, advance by 1 just like the explicit-chain path does — that's the invariant subsequent sweeps rely on.
- Audit shape: when the fallback fires, the new handoff's audit row should look identical to an explicit-chain advance (same `escalation_advance` kind, same tenant/session/handoff IDs). Don't introduce a new audit kind for org-chart fallback — it's an implementation detail.
- Test fixture pattern: `OrgStore::insert_member` exists; build a 3-level chain in test setup with `worker_a → manager_b → manager_c`, originate a handoff from `worker_a` with an exhausted explicit chain, sweep, assert a new Pending handoff to `manager_b`, sweep again, assert one to `manager_c`, sweep a third time, assert `escalation_exhausted`.
- If during implementation you find that `OrgStore::reports_to_chain` does NOT take a `current_index` parameter (it just returns the full chain), call it once and slice — no upstream change needed.
