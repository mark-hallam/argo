# mission-spend-gate-wire-transport — wire SpendGate into argo-agent-runner::build_transport

**Owner**: claude. **Depends on**: `argo_provider::SpendGate` (shipped); `argo_store::spend_ceiling` (shipped).
**Authored against**: origin/main `6e3e31d8` on `2026-05-14`.

> **Phase 3 wire-or-delete decision (CEO-authorized 2026-05-14): WIRE.** Audit cluster 1 D-2 / P-2 found `SpendGate` + `BudgetEnforcer` (~250 LOC) built but un-wired. `argo-store::spend_ceiling` (the enforcement source) IS wired.

## Scope

In `argo-agent-runner::transport::build_transport`, wrap the constructed `Transport` with `SpendGate::new(inner, store_clone)` when `[budget.ceiling]` config is non-empty. Read the ceiling at transport-construction time; pass to gate.

Audit emission: gate emits `AuditKind::Provider` (or new `BudgetExhausted` variant if appropriate) on every refuse-to-dispatch.

## Files you may touch
- `crates/argo-agent-runner/src/transport.rs` — narrowly: ≤30 modified lines at the build site.
- `crates/argo-provider/src/spend_gate.rs` — narrowly: ≤20 modified lines if API needs minor tweak.
- `crates/argo-agent-runner/tests/transport_spend_gate_smoke.rs` (NEW, ≤150 LOC) — assert: turn that would exceed ceiling returns `BudgetExhausted` instead of provider response.

## Acceptance
- [ ] Transport stack with `[budget.ceiling] per_session_usd = 0.01` refuses any call past the ceiling.
- [ ] Audit row emitted on every refused dispatch.
- [ ] Pre-brief callers (no ceiling configured) see no behavior change.
- [ ] Workspace clippy clean.

## Branch
- `claude-mission-spend-gate-wire-transport`
