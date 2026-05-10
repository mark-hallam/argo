# mission-routing-skillmatch-rolematch — RoutingEngine SkillMatch + RoleMatch implementations

**Owner**: claude. **Depends on**: nothing (existing handoff + skill + role tables). **Tier M — Mission MVP critical path.**

**Authored against**: origin/main 7785708b on 2026-05-10.

## Why

Mission claim: workers hand off to each other automatically; Worker B picks up where
Worker A left off without re-asking the same questions. Today `RoutingEngine::route`
returns `None` for the SkillMatch and RoleMatch routing modes (per
MISSION_BACKLOG #51) — every handoff falls through to the human escalation path,
breaking the autonomous-handoff claim. This brief implements those two cases and
proves the chain end-to-end (shared-memory namespace visible to the receiving worker).

## Approach (concrete)

1. **Modify** `crates/argo-orchestrator/src/routing.rs`:
   - Implement `RoutingEngine::route` SkillMatch arm: query the skill registry for
     workers whose available skill set contains `required_skill`, rank by
     trust_grade desc + last-used recency, return the top match.
   - Implement RoleMatch arm: query the worker registry for workers whose
     `role` matches `required_role`, rank the same way, return the top match.
   - NoMatch fallback: return `None` (existing behavior — escalation handler picks up).
2. **NEW or extend** `crates/argo-orchestrator/src/routing_skill_match.rs` and
   `routing_role_match.rs` (split files if `routing.rs` would exceed 500 LOC; the
   500-LOC invariant is hard).
3. **Modify** `crates/argo-orchestrator/src/lib.rs` — narrowly: export the new modules
   if split.
4. **Reuse** the existing handoff record + shared-memory namespace primitives —
   no new schema. The handoff record already carries `required_skill` and
   `required_role` Option fields.
5. **Capability**: routing is read-only against the worker + skill registries —
   no new capability surface; document in rustdoc that the engine consults
   `WorkerRegistry::list_for_tenant` + `SkillRegistry::workers_with_skill`.
6. **Rustdoc**: every `pub` item carries doc + at least one example.

## Files you may touch

- `crates/argo-orchestrator/src/routing.rs` — narrowly: implement SkillMatch + RoleMatch
  arms. If file would exceed 500 LOC, split per step 2.
- `crates/argo-orchestrator/src/routing_skill_match.rs` (NEW if split, ≤ 250 LOC)
- `crates/argo-orchestrator/src/routing_role_match.rs` (NEW if split, ≤ 250 LOC)
- `crates/argo-orchestrator/src/routing_tests.rs` (NEW or extend, ≤ 350 LOC) — 3+ unit tests:
  - SkillMatch returns highest-trust-grade worker with the skill.
  - RoleMatch returns highest-trust-grade worker with the role.
  - NoMatch fallback returns None.
- `crates/argo-orchestrator/src/lib.rs` — narrowly: pub mod lines if files split.
- `crates/argo-orchestrator/tests/handoff_routing_e2e.rs` (NEW, ≤ 250 LOC) —
  1 integration test: Worker A completes duty → emits handoff with
  `required_skill = "marketplace_listing"` → RoutingEngine selects Worker B who has
  that skill → shared memory namespace from A is visible to B → assertion that the
  questions A asked do not need to be re-asked (asserted by checking the memory
  namespace contains the prior Q/A records).

## Files you must NOT touch

- Skill registry implementation itself — read-only consumer.
- Worker registry implementation itself — read-only consumer.
- Handoff schema or shared-memory schema — reuse existing tables.
- Escalation handler — it already handles `None` returns from `route`.

## Acceptance criteria

- [ ] `RoutingEngine::route` SkillMatch returns `Some(WorkerId)` when at least one
      worker has the required skill; ranks by trust_grade desc.
- [ ] `RoutingEngine::route` RoleMatch returns `Some(WorkerId)` when at least one
      worker has the required role; ranks by trust_grade desc.
- [ ] NoMatch path still returns `None` (regression test).
- [ ] Integration test demonstrates A → B handover with shared memory visible to B
      and no re-asking of A's questions.
- [ ] 3+ unit tests + 1 integration test.
- [ ] Rustdoc on every pub item; `cargo clippy -p argo-orchestrator --all-targets -- -D warnings` clean.
- [ ] Every modified file < 500 LOC.

## Branch + PR

- Branch: `claude-mission-routing-skillmatch-rolematch`
- PR title: `routing: implement SkillMatch + RoleMatch with end-to-end handoff test (mission-routing-skillmatch-rolematch)`
