# M6-orch-retry-policy - Track M (M-Orch), Phase 1, Tier 1

**Owner**: claude. **Depends on**: M5 MERGED.
**Authored against**: origin/main `5a5d3970` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh M6-orch-retry-policy`.

## Scope

Per-skill retry policy: max attempts, backoff strategy, on-which-errors. Stored alongside skill IR (annotation, not separate object). Existing argo-mcp-connector shared retry-with-backoff (PR 1301) is the primitive; this brief wires it into the orchestrator at duty level.

LOC: ~700. Branch: `claude-M6-orch-retry-policy`. PR title: `feat(orch): per-skill retry policy + backoff (M6)`.

## Files you may touch

- NEW crates/argo-orchestrator/src/retry.rs
- NEW crates/argo-orchestrator/src/retry_tests.rs
- NEW crates/argo-orchestrator/migrations/NNNN_retry.sql - additive

## Files you must NOT touch

- argo-mcp-connector backoff primitive (wrap, dont refactor).
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] Policy definition + storage.
- [ ] Synthetic failing skill retries per policy.
- [ ] Honors argo-mcp-connector shared backoff.
- [ ] 5 unit tests.
- [ ] No file > 500 LOC.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (Rust-side).
- [ ] `pnpm typecheck` clean (web-side).
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-M6-orch-retry-policy`
- PR title: `feat(orch): per-skill retry policy + backoff (M6)`

## Notes for the implementer

- See `streams/M0-enterprise-parity-track-overview.md` for the authoritative spec for this brief.
- Mission-claim alignment: claim 2.


## Amendment 2026-05-14 (may-touch widening)

Pre-empted from the same self-BLOCK pattern that hit M1/M5/M9: workers reasonably reported the original may-touch was too narrow to ship a real (non-scaffold) implementation. Conductor adds these load-bearing edits to the allowed surface:

- The containing crate's `lib.rs` — narrowly: add `pub mod <new_module>;` declaration. **≤10 modified lines.**
- The containing crate's `Cargo.toml` — narrowly: add any required `[dependencies]` line for sibling argo-* crates that this brief consumes. **≤5 modified lines.**
- If a gateway endpoint is in scope: register the route in the appropriate `routes_dispatch_*.rs` sibling (post-#1601 split). **≤10 modified lines.**

These edits are mechanical and required for the new code to compile + be reachable. Do NOT widen beyond the brief's original scope (no business logic in lib.rs, no new dependencies beyond what the brief names).
