# Mission load-bearing wire — wire `BreakerTransport` into provider client construction

**Owner**: claude. **Depends on**: `argo-provider::transport::BreakerTransport` SHIPPED (verified at `crates/argo-provider/src/transport.rs:216`); `argo-provider::Breaker + BreakerConfig` SHIPPED; `argo-provider::circuit::is_breaker_failure` SHIPPED.
**Authored against**: origin/main `f8f51df4` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-circuit-breaker-wire-providers`.

> SOURCE: MISSION_BACKLOG.md Section 1 #36 — "Wire circuit breaker to provider transport calls." The `BreakerTransport<T>` wrapper is implemented but **never actually constructed** (verified: only doc-comment references in `transport.rs`). Provider clients use the bare transport so transient outages can cascade. Mission promise: every provider client wraps its HTTP transport with a `BreakerTransport` so HTTP-5xx/network-layer failures trip the breaker.

## Scope

1. **Locate provider client constructors**. First 5min task: trace `argo-provider::anthropic::{AnthropicClient::new, ...}` + any other provider impls (search `Transport for ` in `crates/argo-provider/src/`). Cite line numbers in PR body.

2. **Wrap every concrete transport** with `BreakerTransport::new(inner, Breaker::new(BreakerConfig::default()))` at the construction site. Each provider client gets its OWN breaker (don't share — one provider's outage shouldn't trip the others).

3. **Add config knob**: NEW optional `[provider.<name>.breaker]` block in `argo.toml`:
   ```toml
   [provider.anthropic.breaker]
   failure_threshold = 5
   half_open_probe_interval_ms = 30000
   ```
   Missing block → use `BreakerConfig::default()`. Parser change in `argo-config`.

4. **Observability**: emit `tracing::warn!(provider, "breaker_tripped")` when `try_acquire` returns Err. Single event per state transition (not per call).

5. **Operator inspection**: NEW `argo provider breakers [--json]` CLI lists each provider + breaker state (Closed/Open/HalfOpen) + last-transition timestamp. Uses existing `Breaker::state()` (verify name in first 5min).

This is NOT in scope: cross-region failover; circuit state persistence across restarts (in-memory only is fine); per-model breakers within a provider (per-provider only); changing `BreakerConfig` defaults.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-provider/src/anthropic/mod.rs` (verify path) — narrowly scoped: ONE call-site change at client construction, wrap inner with `BreakerTransport::new`. ≤ 30 LOC delta.
- Other provider impls under `crates/argo-provider/src/<vendor>/` — same pattern, per vendor.
- `crates/argo-provider/src/breaker_observability.rs` (NEW, ≤ 100 LOC) — `tracing::warn!` emit helper + state-transition guard.
- `crates/argo-provider/src/breaker_observability_tests.rs` (NEW, sibling) — 3 unit tests.
- `crates/argo-config/src/provider.rs` (verify path) — narrowly scoped: ADD `[provider.<name>.breaker]` parser. ≤ 40 LOC delta.
- `crates/argo-config/src/provider_tests.rs` — 3 NEW tests: parse-default; parse-custom; parse-rejects-bad-value.
- `crates/argo-cli/src/handlers/provider_breakers.rs` (NEW, ≤ 150 LOC) — CLI handler.
- `crates/argo-cli/src/cli_subcommands_provider.rs` (verify path) — narrowly scoped: ADD `Breakers` variant.

## Files you must NOT touch

- `crates/argo-provider/src/transport.rs` — `BreakerTransport` impl stable; reuse only
- `crates/argo-provider/src/circuit.rs` — `Breaker` + `BreakerConfig` + `is_breaker_failure` stable; reuse only
- `crates/argo-store/migrations/**` — no schema change
- Any non-provider/non-config/non-cli crate

## Acceptance criteria

- [ ] Every provider client's transport is wrapped in `BreakerTransport`
- [ ] Each provider gets its own `Breaker` instance (no sharing)
- [ ] `[provider.<name>.breaker]` config parses; missing → default
- [ ] `tracing::warn!` emits exactly once per state transition (Closed→Open, HalfOpen→Open, HalfOpen→Closed)
- [ ] `argo provider breakers --json` returns structured envelope with per-provider state
- [ ] 3+3+3 unit tests across observability + config + CLI
- [ ] Existing provider tests still pass byte-identical
- [ ] `cargo build -p argo-provider -p argo-config -p argo-cli` clean
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-circuit-breaker-wire-providers`
- PR title: `provider: wire BreakerTransport at every provider client + argo provider breakers CLI (mission-circuit-breaker-wire-providers)`

## Notes for the implementer

- The breaker wrap is **load-bearing for reliability**: do not skip the integration test that verifies a 5xx flood actually trips the breaker.
- For the state-transition log: a simple `AtomicU8 last_state` per breaker that you compare-and-swap on each `try_acquire` works. Document the chosen approach in PR body.
- The CLI must not crash if a provider isn't configured — show "not configured" or omit silently. Document choice.
- BLOCK and request revision rather than touch transport.rs / circuit.rs internals.
