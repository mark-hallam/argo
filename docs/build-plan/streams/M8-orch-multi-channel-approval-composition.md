# M8-orch-multi-channel-approval-composition - Track M (M-Orch), Phase 1, Tier 2

**Owner**: claude. **Depends on**: M7 MERGED; existing approvals infra (E1-* MERGED).
**Authored against**: origin/main `5a5d3970` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh M8-orch-multi-channel-approval-composition`.

## Scope

Operator defines approval routing: e.g. this skills high-value approvals go to Slack channel ceo-approvals, fall back to SMS to CFO if no response in 30min, fall back to email if no response in 4h. Composition language stored as part of skill IR (annotation). Runtime walks the routing tree per policy.

LOC: ~1000. Branch: `claude-M8-orch-multi-channel-approval-composition`. PR title: `feat(orch): multi-channel approval routing composition (M8)`.

## Files you may touch

- NEW crates/argo-orchestrator/src/approval_routing.rs
- NEW crates/argo-orchestrator/src/approval_routing_tests.rs
- NEW web/components/skycot/orch/ApprovalRoutingEditor.tsx

## Files you must NOT touch

- E1-* approvals UI; existing channel adapters.
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] Routing tree: 3-channel fallback with timeouts works end-to-end.
- [ ] Editor renders the tree visually.
- [ ] 6 unit tests.
- [ ] No file > 500 LOC.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (Rust-side).
- [ ] `pnpm typecheck` clean (web-side).
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-M8-orch-multi-channel-approval-composition`
- PR title: `feat(orch): multi-channel approval routing composition (M8)`

## Notes for the implementer

- See `streams/M0-enterprise-parity-track-overview.md` for the authoritative spec for this brief.
- Mission-claim alignment: claim 6 (Multi-channel approval gates).


## Amendment 2026-05-14 (may-touch widening)

Pre-empted from the same self-BLOCK pattern that hit M1/M5/M9: workers reasonably reported the original may-touch was too narrow to ship a real (non-scaffold) implementation. Conductor adds these load-bearing edits to the allowed surface:

- The containing crate's `lib.rs` — narrowly: add `pub mod <new_module>;` declaration. **≤10 modified lines.**
- The containing crate's `Cargo.toml` — narrowly: add any required `[dependencies]` line for sibling argo-* crates that this brief consumes. **≤5 modified lines.**
- If a gateway endpoint is in scope: register the route in the appropriate `routes_dispatch_*.rs` sibling (post-#1601 split). **≤10 modified lines.**

These edits are mechanical and required for the new code to compile + be reachable. Do NOT widen beyond the brief's original scope (no business logic in lib.rs, no new dependencies beyond what the brief names).
