# mission-throttle-registry-wire-dispatch — wire ThrottleRegistry into action dispatch path

**Owner**: claude. **Depends on**: `argo_runtime::action_throttle::ThrottleRegistry` (shipped); `argo_config::ActionThrottleConfig` (shipped).
**Authored against**: origin/main `6e3e31d8` on `2026-05-14`.

> **Phase 3 wire-or-delete decision (CEO-authorized 2026-05-14): WIRE.** Audit cluster 3 O-6 found `ThrottleRegistry::try_acquire` exists but the agent dispatch hot path doesn't rate-limit through it. Operator config knobs ship; behavior doesn't.

## Scope

In `argo-cli/src/handlers/worker_actions_*` (the action-dispatch hot path) — wrap every action with a `throttle_registry.try_acquire(tenant_id, action_type).await` call. If `Wait{delay}`, sleep + retry. If `Proceed`, dispatch.

Per CLAUDE.md per-tenant isolation: throttle key is `(tenant_id, action_type)`. Per-action-type throttling means one tenant's `send_email` rate-limit doesn't affect their `code_execute` budget.

## Files you may touch
- `crates/argo-cli/src/handlers/worker_actions_executor.rs` (or wherever the central dispatcher lives) — narrowly: ≤50 LOC at the per-action entry.
- `crates/argo-runtime/src/action_throttle.rs` — narrowly: minor API tweaks if needed.
- `crates/argo-cli/tests/throttle_smoke.rs` (NEW, ≤150 LOC) — assert: 10 rapid send_email actions on one tenant gate at the configured rate; different action types interleave freely.

## Acceptance
- [ ] Per-(tenant, action) throttle enforced.
- [ ] Disabled-config path preserves pre-brief behavior.
- [ ] Audit row on every `Wait` > 10s (already a defined `AuditKind` variant).
- [ ] Workspace clippy clean.

## Branch
- `claude-mission-throttle-registry-wire-dispatch`
