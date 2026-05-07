# Prometheus /metrics endpoint on gateway

**Owner**: claude. **Depends on**: none.

> SOURCE: production hardening Phase C. The OTLP path exports
> tracing data, but Prometheus is the dominant scrape format for
> ops-team dashboards (Grafana, Alertmanager, kube-prometheus).
> This brief adds a `/metrics` endpoint to the gateway in
> Prometheus text format, populated from a small handful of
> counters and one histogram.

## Scope

Expose a `/metrics` HTTP endpoint on the gateway in Prometheus
text exposition format (RFC). Initial metrics:

**Counters** (monotonic, labeled by `tenant_id` where relevant):
- `argo_sessions_total` — sessions created
- `argo_messages_total{role}` — messages added per role
- `argo_audit_rows_total` — audit ledger rows written
- `argo_errors_total{class}` — error count by class
  (provider, schema, store, network, internal)

**Histogram**:
- `argo_action_latency_seconds{action}` — wall time per
  gateway action (e.g. `session.create`, `message.add`,
  `forge.run`); buckets `[0.01, 0.05, 0.1, 0.5, 1, 5, 10]`

Metrics live in a process-global `prometheus::Registry` and the
endpoint serializes via `prometheus::TextEncoder`. Feature flag
`prometheus` (default OFF — keeps single-binary distribution
zero-deps; ops users opt in via `--features prometheus`).

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_metrics.rs` (NEW) —
  ~140 LOC; defines the `Metrics` struct (Lazy global), the
  `record_*` helpers callers invoke, and the
  `prometheus_metrics_handler` axum route
- `crates/argo-cli/src/handlers/gateway/mod.rs` — narrowly:
  feature-gated `pub mod routes_metrics;` + route mount under
  feature `prometheus`
- `crates/argo-cli/src/handlers/gateway/dispatch_route.rs` —
  narrowly: 4-6 call sites to `Metrics::record_action_latency`
  and `record_error` (feature-gated zero-cost when off)
- `crates/argo-cli/Cargo.toml` — narrowly: optional dep
  `prometheus = "0.13"`; new feature
  `prometheus = ["dep:prometheus"]`
- `crates/argo-cli/src/handlers/gateway/routes_metrics_tests.rs`
  (NEW) — 4 tests: text format parses, counters increment,
  histogram observes, no-feature build path is a no-op

## Files you must NOT touch

- `argo-tracing` / `argo-telemetry` — those are tracing-side;
  this brief is metrics-side. Keep them separate.
- The OTLP bridge from #938 / claude-otlp-tracing-bridge —
  Prometheus is a parallel exposition path, not a replacement.
- Any provider crate — provider-specific metrics belong in a
  later brief once we've validated the global pattern.

## Acceptance criteria

- [ ] Feature `prometheus` default-off; build green without it
- [ ] `/metrics` returns 200 with text/plain `version=0.0.4`
      content type when feature on
- [ ] All four counters increment from real call sites in
      `dispatch_route.rs`
- [ ] Histogram observes latency on at least one action
- [ ] Output passes `promtool check metrics` (or equivalent
      static parse) in CI
- [ ] 4 unit tests + 1 integration test (curl `/metrics`,
      assert each counter present)
- [ ] RUNBOOK extended with scrape config example
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli` clean
- [ ] `cargo build --features prometheus -p argo-cli` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-metrics-prometheus-export`
- PR title: `gateway: Prometheus /metrics endpoint behind prometheus feature (claude-metrics-prometheus-export)`

## Notes for the implementer

- Use `prometheus::register_int_counter_vec_with_registry!` for
  labeled counters; pin labels to a small allowlist.
- Do NOT use unbounded label values (e.g. tenant_id is bounded;
  request URL is not — never label by URL).
- The `prometheus` crate's `Encoder::encode_to_string` is the
  cleanest path; avoid manual format string assembly.
- DO NOT fold OTLP metrics export into this brief — separate
  concern, separate PR.
