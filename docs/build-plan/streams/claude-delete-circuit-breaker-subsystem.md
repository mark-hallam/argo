# claude-delete-circuit-breaker-subsystem — delete decorative ~600 LOC breaker code

**Owner**: claude. **Depends on**: nothing.
**Authored against**: origin/main `6e3e31d8` on `2026-05-14`.

> **Phase 3 wire-or-delete decision (CEO-authorized 2026-05-14): DELETE.** Audit cluster 1 D-1 found the entire circuit-breaker subsystem (`Breaker`, `BreakerConfig`, `CircuitState`, `Permit`, `BreakerTransport`, `ObservedBreakerTransport`, `StateObserver`, `is_breaker_failure` — ~600 LOC across `circuit.rs`, `breaker_observability.rs`, `from_manifest_with_breaker` constructors) is never invoked. `argo-cli/handlers/provider_breakers.rs:8-15` admits: "Breakers are constructed fresh on every CLI invocation, so the reported state is always Closed." Decorative.

## Scope

Delete the breaker subsystem. Revive in a future brief if a real customer needs it.

## Files you may touch
- `crates/argo-provider/src/circuit.rs` — `git rm`.
- `crates/argo-provider/src/breaker_observability.rs` — `git rm`.
- `crates/argo-provider/src/transport.rs` — narrowly: remove `BreakerTransport` + `ObservedBreakerTransport` impl blocks + `from_manifest_with_breaker` constructors. ≤80 LOC removed.
- `crates/argo-provider/src/lib.rs` — narrowly: remove re-exports.
- `crates/argo-cli/src/handlers/provider_breakers.rs` — `git rm` (the inspector CLI; pointless without the breaker).
- `crates/argo-cli/src/cli_subcommands_provider.rs` — narrowly: remove `breakers` subcommand variant.
- Any test referencing the removed types — delete.

## Acceptance
- [ ] All Breaker* types removed from argo-provider.
- [ ] `argo provider breakers` subcommand removed.
- [ ] `cargo build --workspace` clean.
- [ ] Workspace clippy clean.
- [ ] Net LOC ~-600.

## Branch
- `claude-delete-circuit-breaker-subsystem`
