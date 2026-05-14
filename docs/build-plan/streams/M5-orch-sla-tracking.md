# M5-orch-sla-tracking - Track M (M-Orch), Phase 1, Tier 1

**Owner**: claude. **Depends on**: nothing.
**Authored against**: origin/main `5a5d3970` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh M5-orch-sla-tracking`.

## Scope

Sla definition: target latency + escalation policy + breach action. Attaches to a duty / skill / worker. Tracker watches in-flight runs; emits breach events on threshold. Multi-tenant.

LOC: ~900. Branch: `claude-M5-orch-sla-tracking`. PR title: `feat(orch): SLA tracking + breach events (M5)`.

## Files you may touch

- NEW crates/argo-orchestrator/src/sla.rs
- NEW crates/argo-orchestrator/src/sla_tests.rs
- NEW crates/argo-orchestrator/migrations/NNNN_sla.sql - additive (pick next-free slot at claim time)

## Files you must NOT touch

- Existing escalation/handoff machinery (extend, dont refactor).
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] SLA definition CRUD via gateway endpoint.
- [ ] Breach event fires when synthetic run exceeds threshold.
- [ ] Escalation policy honored (notify / reroute / fail).
- [ ] 6 unit tests.
- [ ] No file > 500 LOC.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (Rust-side).
- [ ] `pnpm typecheck` clean (web-side).
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-M5-orch-sla-tracking`
- PR title: `feat(orch): SLA tracking + breach events (M5)`

## Notes for the implementer

- See `streams/M0-enterprise-parity-track-overview.md` for the authoritative spec for this brief.
- Mission-claim alignment: claim 2 (Delegation contract terms).


## Amendment 2026-05-14 (may-touch widening)

Worker self-BLOCKED on initial claim noting that may-touch was too narrow to ship a real (non-scaffold) implementation. Conductor adds these load-bearing edits to the allowed surface:

- The containing crate's `lib.rs` — narrowly: add `pub mod <new_module>;` declaration. **≤10 modified lines.**
- The containing crate's `Cargo.toml` — narrowly: add any required `[dependencies]` line for sibling argo-* crates that this brief consumes. **≤5 modified lines.**
- If a gateway endpoint is in scope: register the route in the appropriate `routes_dispatch_*.rs` sibling (post-#1601 split). **≤10 modified lines.**

These edits are mechanical and required for the new code to compile + be reachable. Do NOT widen beyond the brief's original scope (no business logic in lib.rs, no new dependencies beyond what the brief names).
