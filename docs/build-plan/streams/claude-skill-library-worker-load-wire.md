# claude-skill-library-worker-load-wire — Wire marketplace library skills into runtime worker context (v1 SHIP-BLOCKER)

**Owner**: claude. **Depends on**: nothing (existing library infrastructure is shipped — just unwired at runtime).
**Authored against**: origin/main `29b798be` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-skill-library-worker-load-wire`.

## Scope

**🔴 V1 SHIP-BLOCKER discovered by Phase A.5 audit 2026-05-14.**

Phase A.5 traced the Claim 4 skill loop and found that **workers do not load marketplace skills at runtime**. `argo-agent-runner/src/turn.rs` (around line 180) builds skill context by calling `SkillStore::new(store).skills_for_session(…)` — but the store it constructs is **workspace-only**. The `LayeredLookup::with_library` constructor (in `argo-skill/src/library.rs` ~line 50) exists and is ready to use, but no production runtime caller invokes it.

**Impact**: Success scenario #2 in MISSION.md — "A learned skill propagates across workers" — silently fails. Maya learns a skill, it gets published to the marketplace, and a different worker can never load it because the runtime never consults the marketplace. The mission claim is real-code-broken, not just under-implemented.

**Fix**: ~150 LOC in `argo-agent-runner/src/turn.rs` (and possibly its context-builder) to:
1. Accept (or construct) a second `SkillStore` handle for the library/marketplace.
2. Replace `SkillStore.skills_for_session(...)` with `LayeredLookup::with_library(workspace_store, library_store).skills_for_session(...)`.
3. Pass through tenant_id properly so cross-tenant skill leak doesn't happen.
4. Add an integration test that publishes a skill, reloads the agent, and confirms the skill is visible in the worker's system prompt.

The library SkillStore handle source is operator-decisional but the simplest v1 path is: read from the same SQLite store the gateway already uses for marketplace skills (see `argo-cli/src/handlers/skill_catalog.rs` and the H-4b follow-up TODO comment in `crates/argo-cli/src/handlers/gateway/routes_skills.rs:30`).

LOC: ~150.

## Files you may touch

- `crates/argo-agent-runner/src/turn.rs` — narrowly: replace workspace-only skill load with LayeredLookup.with_library. ≤80 modified lines.
- `crates/argo-agent-runner/src/turn_helpers.rs` (or the context-builder sibling that owns AgentDeps construction) — narrowly: thread library_store handle through. ≤40 modified lines.
- `crates/argo-agent-runner/src/lib.rs` — narrowly: re-export only if needed. ≤5 modified lines.
- `crates/argo-agent-runner/Cargo.toml` — confirm `argo-skill = { workspace = true }` is already present (it is). No change expected.
- NEW or extended test: `crates/argo-agent-runner/src/turn_library_integration_tests.rs` (≤200 LOC) — smoke that publishes a fixture skill + reloads agent + asserts skill is in the rendered system prompt.
- `crates/argo-cli/src/handlers/gateway/routes_skills.rs` — narrowly: remove the H-4b TODO comment at line 30 once with_library is wired. ≤10 modified lines.

## Files you must NOT touch

- `argo-skill/src/library.rs` — LayeredLookup is already correct; this brief is the CONSUMER, not the producer.
- `argo-skill/src/marketplace.rs` — discovery surface; separate concern.
- Multi-tenant isolation in argo-store — make sure tenant_id flows through, but don't refactor the isolation primitives.
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] `argo-agent-runner/src/turn.rs` constructs skills via `LayeredLookup::with_library(...)`, not `SkillStore.skills_for_session()` alone.
- [ ] Library store handle is plumbed through AgentDeps (or equivalent injection point).
- [ ] Tenant-isolation preserved: a tenant's worker NEVER loads another tenant's private skills via the library path. Test fixture proves this.
- [ ] Integration test: publish a fixture marketplace skill → start a worker → confirm the skill appears in the system prompt rendered by the worker.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] `cargo test --workspace` passes including the new integration test.
- [ ] H-4b TODO comment in `routes_skills.rs:30` removed (the follow-up is now landed).
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `claude-skill-library-worker-load-wire`
- PR title: `feat(skill): wire marketplace library skills into runtime worker context (v1 SHIP-BLOCKER, claude-skill-library-worker-load-wire)`

## Notes for the implementer

- **Mission-claim alignment: claim 4 (Skills shared and learned). This is the load-bearing fix for success scenario #2.**
- Read Phase A.5 audit report at `docs/skycot/audit/2026-05-14-phase-A.5-skills-loop.md` (will be created alongside this brief) for full context.
- Library store source for v1: the same store the gateway already reads from in `skill_catalog.rs`. Pass-through is fine; do not invent a new store layer.
- **Tenant isolation is critical here.** Read `crates/argo-store/src/tenant.rs` (or wherever tenant scoping is enforced) before wiring. The test fixture MUST include a cross-tenant negative case.
- Drift-prevention rule applied: brief adds zero new pub types — only wires an existing producer (LayeredLookup::with_library) to its missing consumer.
