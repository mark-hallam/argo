# claude-skill-marketplace-end-to-end-test — Smoke test the full claim-4 loop (teach → propose → publish → reload → visible)

**Owner**: claude. **Depends on**: claude-skill-library-worker-load-wire MERGED (without it, the test will fail by design).
**Authored against**: origin/main `29b798be` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-skill-marketplace-end-to-end-test`.

## Scope

Phase A.5 audit confirmed the individual edges of the claim-4 skill loop (capture → distill → eval → publish → discovery) are each implemented, but **no test exercises the FULL loop end-to-end**. We need a smoke that:

1. Teaches a worker a fixture skill via the CU recording / mechanical path.
2. Skill candidate proposes for review.
3. Eval gate runs + passes (deterministic fixture).
4. Skill promotes to published state.
5. Skill bundles into OCI tar + lands in marketplace.
6. A DIFFERENT worker (different worker_id, same tenant) starts up.
7. The new worker's system prompt INCLUDES the marketplace-loaded skill.
8. Assertion: success scenario #2 ("learned skill propagates across workers") is demonstrably working.

Without this test, regressions in the loop are silent — exactly the failure mode Phase A.5 caught.

LOC: ~250 (one integration test file + fixture data).

## Files you may touch

- NEW `crates/argo-skill/tests/end_to_end_claim4_loop.rs` — the integration test.
- NEW `crates/argo-skill/tests/fixtures/claim4_e2e_skill.md` — fixture skill content.
- NEW `crates/argo-skill/tests/fixtures/claim4_e2e_trajectory.json` — fixture CU trajectory.
- `scripts/smoke-skycot.sh` — narrowly: invoke the new test as a smoke step. ≤15 modified lines.
- `crates/argo-skill/Cargo.toml` — narrowly: add `[[test]] name = "end_to_end_claim4_loop"` if not auto-discovered. ≤5 modified lines.

## Files you must NOT touch

- The skill loop production code (this is a TEST brief — no production changes).
- argo-marketplace production code.
- argo-agent-runner production code.
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] `cargo test --test end_to_end_claim4_loop -p argo-skill` passes.
- [ ] Test exercises all 8 stages above; failure of any stage produces a clear assertion message.
- [ ] Test is deterministic (no LLM calls; uses fixture data + mocked judge).
- [ ] Test runs in <10s wall-clock.
- [ ] `bash scripts/smoke-skycot.sh` includes this as a step + still exits 0.
- [ ] Cross-tenant negative case: another tenant's worker does NOT load the fixture skill.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `claude-skill-marketplace-end-to-end-test`
- PR title: `test(skill): end-to-end claim-4 loop smoke (claude-skill-marketplace-end-to-end-test)`

## Notes for the implementer

- Mission-claim alignment: claim 4 verification + regression-prevention for success scenario #2.
- **This test will FAIL until claude-skill-library-worker-load-wire MERGES.** That's by design — the test exists to prove Brief A's claim. If the test passes without Brief A, the test is wrong (likely missed the cross-tenant or workspace-only check).
- Use the existing fixture patterns in `crates/argo-skill/tests/` for consistency.
- Drift-prevention: brief adds NEW test files only. No new pub types.
