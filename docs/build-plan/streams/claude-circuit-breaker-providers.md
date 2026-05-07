# Per-provider circuit breaker (Anthropic / OpenAI)

**Owner**: claude. **Depends on**: none.

> SOURCE: production hardening Phase C. When Anthropic or
> OpenAI degrades (5xx storm, rate-limit lockout), every
> in-flight request retries the dead provider, exhausting our
> own resources before failing. This brief adds a per-provider
> circuit breaker: after N consecutive failures, fail fast for a
> cooldown, then half-open with a probe.

## Scope

Per-provider circuit breaker with three states:

- **Closed** — normal; failures increment a counter; on
  N=5 consecutive failures, transition to Open.
- **Open** — fail fast (return `ProviderError::CircuitOpen`)
  for `cooldown_secs` (default 30s); after cooldown, transition
  to HalfOpen.
- **HalfOpen** — admit a single probe request; on success,
  reset to Closed; on failure, back to Open and double the
  cooldown (cap at 5 min).

State is per-provider (Anthropic, OpenAI, future Bedrock, ...)
held in a `DashMap<ProviderId, BreakerState>` on the
`ProviderRegistry` (or wherever providers are dispatched —
verify file layout).

What counts as a failure: HTTP 5xx, timeout, connection refused.
**Not** a failure: HTTP 4xx (client error — that's our bug,
not theirs), rate-limit 429 (rate-limit has its own retry
semantics — don't break circuit on it).

Tunables (env):
- `ARGO_CB_FAILURE_THRESHOLD=5`
- `ARGO_CB_COOLDOWN_SECS=30`
- `ARGO_CB_COOLDOWN_MAX_SECS=300`

## Files you may touch

- `crates/argo-provider/src/circuit_breaker.rs` (NEW, ~200 LOC) —
  `Breaker`, `BreakerState`, `record_success`, `record_failure`,
  `try_acquire(provider) -> Result<Permit, CircuitOpen>`
- `crates/argo-provider/src/lib.rs` — narrowly: declare module,
  expose `Breaker` for the registry
- `crates/argo-provider/src/registry.rs` (or whatever orchestrates
  provider calls — verify; this is the dispatcher) — narrowly:
  `try_acquire` before each provider call; `record_*` based on
  the response
- `crates/argo-provider/src/circuit_breaker_tests.rs` (NEW) —
  6 tests: closed → open after threshold; open rejects fast;
  cooldown elapsed → half-open; half-open success → closed;
  half-open failure → open with doubled cooldown; cooldown cap
  enforced
- `crates/argo-cli/src/handlers/costs_anomalies.rs` (or wherever
  the `argo-errors_total{class}` counter from
  `claude-metrics-prometheus-export` lives, **only if that
  brief lands first**) — narrowly: surface circuit-open in the
  error counter as `class=circuit_open`. Skip if the metrics
  brief hasn't merged.

## Files you must NOT touch

- The HTTP client crate (reqwest) config — circuit breaker is
  above the HTTP layer.
- Rate-limit logic (per-MCP or per-route) — separate concern;
  this brief's failures intentionally exclude 429.
- Provider-specific authentication / signing — circuit breaker
  is provider-agnostic.

## Acceptance criteria

- [ ] State transitions match the documented spec exactly
- [ ] N=5 consecutive 5xx → Open
- [ ] 4xx and 429 do NOT trip the breaker
- [ ] Open returns `CircuitOpen` immediately (no API call)
- [ ] HalfOpen admits exactly one probe; concurrent calls are
      rejected as Open until the probe resolves
- [ ] Cooldown doubles on probe failure, capped at 5 min
- [ ] Env overrides honored
- [ ] 6 unit tests + 1 integration test (mock provider that
      can be toggled to 5xx, observe state machine)
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-provider` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-circuit-breaker-providers`
- PR title: `provider: per-provider circuit breaker (claude-circuit-breaker-providers)`

## Notes for the implementer

- The HalfOpen single-probe rule needs a careful concurrency
  model: an `AtomicBool` `probe_in_flight` works; or a
  `tokio::sync::Semaphore::new(1)` you `try_acquire` in HalfOpen
  state. The Semaphore is cleaner because the probe owner can
  release on completion regardless of outcome.
- DO NOT hold the `DashMap` shard lock across the actual provider
  HTTP call — read state, drop lock, call provider, re-acquire,
  update. Otherwise you serialize all calls to that provider.
- "Consecutive" failure counting: a single success resets the
  counter to 0 (don't use a sliding window — keeps the
  state machine small).
- DO NOT integrate with the OTLP metrics path in this brief.
  That's a follow-up if anyone asks.
