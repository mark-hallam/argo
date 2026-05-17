# mission-claim-4-runtime-closure-trace — prove the skills-learned loop end-to-end (P1 audit-finding)

**Owner**: claude. **Sizing hint**: judgment-required, integration test.
**Depends on**: tenant substrate (#1777 MERGED), skill-library worker-load (#1682 MERGED), F-11 selector (#1740 MERGED).
**Authored against**: origin/main `fa849bbb` on `2026-05-17`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-claim-4-runtime-closure-trace`.
**Source**: 2026-05-17 claude conductor audit finding P1-3. Mission claim 4 ("skills shared and learned") is PARTIAL — substrate exists + library-load is wired, but the CU-trajectory → SkillCandidate → propose_for_review → marketplace-publish CLOSURE is not runtime-traced end-to-end.

## Mission alignment

- **🎯 Mission claim 4** — the launch claim is "a learned skill propagates across workers." Without runtime closure proof, the claim is paper.
- **Reliability** (CLAUDE.md priority #1) — integration tests catch loop-closure regressions before they reach customers.
- **Observability** (#2) — audit chain trace for skill lifecycle (proposed → reviewed → published → re-injected).

## Scope

P1. Integration test in `crates/argo-cli/tests/skill_loop_closure_smoke.rs` that drives the full skills-learned loop end-to-end and asserts each waypoint.

### Test flow

1. **Setup.** Spawn gateway + initialize 2 tenants (A + B) via substrate APIs. Each gets 1 worker (`workerA`, `workerB`).
2. **`workerA` performs a successful action.** Stub the executor to return success + a synthetic trajectory: `{ tokens_used, success_metric, final_text }`.
3. **Assert: SkillCandidate proposed.** Per `argo-skill::learn::should_propose`, success + threshold should fire `SkillCandidate::from_cu_trajectory`. The candidate row should appear in `skill_candidate` table. Assert audit row `SkillCandidatePromoted` (or its successor variant — check current event.rs).
4. **Operator approves the candidate** (simulate via admin endpoint OR direct store call).
5. **Assert: Skill published to marketplace.** Audit row `SkillApproved` emitted; the skill row moves from `skill_candidate` to `skill` table (or whatever the canonical layer transition is); marketplace publish-row created.
6. **`workerB` (different tenant) loads library skills.** Per `LayeredLookup::with_library` + F-11 selector + relevance filter, the published skill SHOULD appear in `workerB`'s prompt context (assuming vertical / trust / tag match).
7. **Assert: Cross-tenant negative case.** A library skill from tenant A's workspace-private layer (NOT published) should NOT appear in tenant B's prompt — verify the cross-tenant boundary holds.

### Audit trail assertions

After the test, `argo audit verify-chain` should report a clean chain with the full lifecycle:
- `ToolDispatch` (workerA's action)
- `ToolResult` (success)
- `SkillCandidatePromoted` (or equivalent)
- `SkillApproved` (after operator approve)
- `ToolDispatch` (workerB's turn, with the library skill now in prompt context — assert via audit-row payload or a fresh assertion-side mechanism)

LOC: ~300.

## Files you may touch

- `crates/argo-cli/tests/skill_loop_closure_smoke.rs` (NEW) — main test. ≤280 LOC.
- `crates/argo-cli/tests/fixtures/skill_trajectory_seed.rs` (NEW or extend) — synthetic CU trajectory generator. ≤80 LOC.
- `crates/argo-cli/Cargo.toml` — narrowly: ensure dev-deps (likely already present). ≤4 modified LOC.
- `docs/skycot/operator/skill-lifecycle.md` (NEW) — operator runbook documenting the closed loop + how to debug if it breaks. ≤80 LOC.

## Files you must NOT touch

- Production code in argo-skill, argo-agent-runner, argo-audit, etc. Pure consumer of the existing API surface.
- The audit chain hash logic — read-only consumer.
- F-11 SkillRelevanceSelector (#1740 MERGED) — consume only.

## Acceptance criteria

- [ ] Test runs end-to-end via `cargo test -p argo-cli --test skill_loop_closure_smoke -- --nocapture`.
- [ ] Test takes < 3 minutes on a typical dev laptop.
- [ ] All 7 waypoints assertable + asserted.
- [ ] Cross-tenant negative case: tenant A's workspace-private skill is NOT injected into tenant B's prompt.
- [ ] Audit chain `verify-chain` reports clean after the test.
- [ ] Operator runbook documents: how to run, what each waypoint proves, debugging if any waypoint fails.
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `mission-claim-4-runtime-closure-trace`
- Title: `test(claim-4): skills-learned loop closure smoke — CU-trajectory → proposal → publish → cross-worker injection (mission-claim-4-runtime-closure-trace)`
- Worker verification: crate-scoped clippy + integration test + per SWARM_PROTOCOL.md.

## Hidden constraints

- **Use existing `should_propose` + `SkillCandidate::from_cu_trajectory` APIs.** Don't shortcut by directly inserting a skill row.
- **Dreams distillation** (V2.0 Integration B, in flight) is a separate pipeline. This brief tests the simpler "successful-action → candidate" closure. Dreams is a follow-up.
- **The library injection assertion is the load-bearing check.** Without it, "skills shared across workers" is claimed but unproven. The test should FAIL clearly if the library-load wiring regresses.
- **Per-vertical / trust / tag relevance filter (#1740 F-11)** — pick a skill + worker pair where the filter SHOULD pass and another where it SHOULD fail. Assert both.
- **Cleanup teardown:** delete the 2 test tenants + all derived skill/candidate/audit rows after the test. Idempotent re-runs.
- **Closes audit finding P1-3.** Mission claim 4 graduates from PARTIAL to PROVEN on main.
