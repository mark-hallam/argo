# fix-routing-decision-persist-wire

**Owner**: codex.
**Depends on**: `fix-routing-decision-tenant-scoping` MERGED (this brief threads the now-tenant-aware context through the sole production caller).
**Authored against**: origin/main `0bf75c1a1` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-routing-decision-persist-wire`.

🟡 **P1 CLAIM-5 RUNTIME.** Wrap `TieredRouter::new(classifier)` in `PersistingRouter` at the only production caller so `routing_decision` rows actually land in the database. Source: `eval-argo-routing-crate-surface` audit (#2115) — pinned FINDINGS-ONLY (b).

## Why this is P1

Per gap manifest line 84:
> (b) only production caller `argo_agent_runner::turn_routing::maybe_route_turn_with_transport:87` builds `TieredRouter::new` bare and never wraps in `PersistingRouter` → `routing_decision` table receives no writes.

The `PersistingRouter` decorator exists (`crates/argo-routing/src/persist.rs`) and is exported. The substrate is complete. The runtime path just never wraps. Result: `routing_decision` table is permanently empty, the §24.4 acceptance criterion is met by audit log only, and operator analytics on routing behaviour are blocked.

## Scope

**In:**
- `crates/argo-agent-runner/src/turn_routing.rs` around line 87 — wrap `TieredRouter::new(classifier)` in `PersistingRouter::new(tiered, store, tenant_id)` (signature TBD post-#-fix-routing-decision-tenant-scoping).
- Thread `tenant_id` from caller's `Session` / `AgentDeps` into the wrapper.
- Update any in-scope tests in `crates/argo-agent-runner/tests/` that exercise this path.
- 1 NEW integration-test binary `crates/argo-agent-runner/tests/routing_decision_persisted_test.rs` (or extend existing): drive one routed turn, assert `routing_decision` table has exactly 1 row tagged with the correct tenant_id + non-Fixed decision kind.

**Out:**
- `argo-routing` crate surface (locked by sibling brief).
- Any new public API.
- Backfilling existing rows.

## Acceptance

- `cargo test -p argo-agent-runner --test routing_decision_persisted_test` green.
- `cargo clippy -p argo-agent-runner --all-targets -- -D warnings` clean.
- §24.4 of the build plan flips from "audit-log only" → "live SQL row".
- Gap-manifest row updated (one-line edit to mark `(b)` SHIPPED).

## Branch + PR

- Branch: `codex-fix-routing-decision-persist-wire`
- PR title: `fix(routing): wrap TieredRouter in PersistingRouter at turn_routing callsite`

## Notes for the implementer

- ~50-100 LOC expected. Mechanical wrap + 1 integration test.
- Cite the gap manifest row verbatim in the PR body.
- DO NOT claim until `fix-routing-decision-tenant-scoping` is MERGED — the `PersistingRouter` constructor signature changes in that PR.
- If sibling brief isn't merged yet, BLOCK + leave a note for the conductor.
