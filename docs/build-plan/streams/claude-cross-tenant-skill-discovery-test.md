# claude-cross-tenant-skill-discovery-test — Cross-tenant skill discovery negative test

**Owner**: codex. **Depends on**: `claude-skill-library-worker-load-wire` MERGED (the v1 SHIP-BLOCKER, currently CLAUDE-PREFERRED — author this brief NOW so it's ready to claim the moment the SHIP-BLOCKER lands).
**Authored against**: origin/main `66d07a69` on `2026-05-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-cross-tenant-skill-discovery-test`.

## Scope

Phase C audit P1.5 follow-up. Once `claude-skill-library-worker-load-wire` ships, workers will load marketplace skills via `LayeredLookup::with_library`. The integration test in that brief verifies the positive path (a worker sees a marketplace skill). This brief adds the **negative test**: a worker in tenant B does NOT see a skill published by tenant A.

Critical for v1 because cross-tenant isolation is a load-bearing security claim (multi-tenant by construction is invariant #2 in CLAUDE.md). Without this test, a future refactor could silently break tenant isolation and ship a security regression.

LOC: ~100.

## Files you may touch

- NEW `crates/argo-store/tests/skill_discovery_cross_tenant.rs` — integration test. Fixtures: tenant A publishes skill X; tenant B's worker starts up; assertion: X is NOT in B's rendered system prompt.
- `scripts/smoke-skycot.sh` — narrowly: invoke as a smoke step (skip on environments lacking the lib store). ≤10 lines.

## Files you must NOT touch

- The SHIP-BLOCKER's production code (this is a TEST brief).
- argo-skill or argo-agent-runner production code.
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] `cargo test --test skill_discovery_cross_tenant -p argo-store` passes.
- [ ] Test is deterministic (mock LLM; fixture skill data).
- [ ] Negative assertion: tenant B's rendered prompt does NOT contain tenant A's skill body.
- [ ] Positive control: tenant A's worker DOES see the skill (sanity check).
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `codex-claude-cross-tenant-skill-discovery-test`
- PR title: `test(store): cross-tenant skill discovery negative (claude-cross-tenant-skill-discovery-test)`

## Notes for the implementer

- Mission-claim alignment: production hardening (security; supports claim 4 multi-tenant integrity).
- This brief is DEPENDENCY-BLOCKED until the v1 SHIP-BLOCKER lands. Codex worker adapter should self-BLOCK on claim if the dep isn't MERGED yet; that's the correct behavior.
- Drift-prevention: test-only brief; no new pub types.
