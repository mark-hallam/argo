# M7-orch-queue-management-ui - Track M (M-Orch), Phase 1, Tier 2

**Owner**: claude. **Depends on**: M5 MERGED; M6 MERGED; E0 MERGED.
**Authored against**: origin/main `5a5d3970` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh M7-orch-queue-management-ui`.

## Scope

/orchestration route: list of in-flight + queued runs across workers. Filters: worker / skill / SLA state / status. SLA cards: live latency vs target with breach indicator. Retry badges. Operator actions: cancel run, reset retry, force-escalate.

LOC: ~1100. Branch: `claude-M7-orch-queue-management-ui`. PR title: `feat(web/orch): queue management UI (M7)`.

## Files you may touch

- NEW web/app/(app)/orchestration/page.tsx (<=350)
- NEW web/components/skycot/orch/QueueTable.tsx (<=350)
- NEW web/components/skycot/orch/SlaCard.tsx (<=250)
- NEW web/components/skycot/orch/RetryBadge.tsx (<=100)
- NEW web/lib/api/orchestration.ts
- NEW crates/argo-cli/src/handlers/gateway/routes_orchestration.rs - gateway endpoints

## Files you must NOT touch

- M5/M6 backend (consume APIs, dont change).
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] Route loads, shows >= 50 mock runs cleanly.
- [ ] All filters work.
- [ ] Operator actions hit gateway.
- [ ] 8 unit tests + 1 e2e.
- [ ] No file > 500 LOC.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (Rust-side).
- [ ] `pnpm typecheck` clean (web-side).
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-M7-orch-queue-management-ui`
- PR title: `feat(web/orch): queue management UI (M7)`

## Notes for the implementer

- See `streams/M0-enterprise-parity-track-overview.md` for the authoritative spec for this brief.
- Mission-claim alignment: claim 2 (Delegation observability).


## Amendment 2026-05-14 (may-touch widening)

Pre-empted from the same self-BLOCK pattern that hit M1/M5/M9: workers reasonably reported the original may-touch was too narrow to ship a real (non-scaffold) implementation. Conductor adds these load-bearing edits to the allowed surface:

- The containing crate's `lib.rs` — narrowly: add `pub mod <new_module>;` declaration. **≤10 modified lines.**
- The containing crate's `Cargo.toml` — narrowly: add any required `[dependencies]` line for sibling argo-* crates that this brief consumes. **≤5 modified lines.**
- If a gateway endpoint is in scope: register the route in the appropriate `routes_dispatch_*.rs` sibling (post-#1601 split). **≤10 modified lines.**

These edits are mechanical and required for the new code to compile + be reachable. Do NOT widen beyond the brief's original scope (no business logic in lib.rs, no new dependencies beyond what the brief names).
