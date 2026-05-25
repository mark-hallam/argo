# eval-argo-telemetry-crate-surface

Date: 2026-05-24
Brief: `eval-argo-telemetry-crate-surface`
Verdict: PASS

## Scope

Audited the public surface exported from `crates/argo-telemetry/src/lib.rs`:

- Public sink seam: `TelemetrySink`, `SpanGuard`, `NoopTelemetry`, `InMemoryTelemetry`, `TracingTelemetry`, and recorded event structs.
- Mission taxonomy: `spans::*`, `metrics::*`, `spans::all()`, and `metrics::all()`.
- Public modules: `cardinality`, `error_reporting`, `http_target`, `metrics_prompt_cache`, `pii_scrubber`, `prometheus_registry`, and `span_context`.
- Public Prometheus helper surface and production metric families used by gateway/operator dashboards.
- Optional OTLP and Sentry surfaces were checked as feature-gated exports, but this audit did not require a live collector or network.

## Evidence Added

Added `crates/argo-telemetry/tests/surface_contract.rs` as an external integration test that compiles only through the crate's public API. It pins:

- Exported modules, types, traits, function reachability, and constants from `lib.rs`.
- The exact sorted contents of `spans::all()` and `metrics::all()`.
- OpenMetrics registration for the production gateway/operator metric families.
- `NoopTelemetry` no-panic sink behavior and `InMemoryTelemetry` recording behavior for spans, counters, and histograms.
- Same-metric-family cardinality overflow behavior with a local `CardinalityTracker`.
- Public PII scrubbing and tenant pseudonymization behavior.

## Findings

No contract gaps were found in this pass. The crate already had useful internal unit coverage for the same subsystems; the new integration test pins the externally visible API so downstream crates cannot silently lose the observability surface.

## Follow-Ups

None. No real finding was identified that warrants a new brief.

## Verification

- `bash scripts/check-brief-freshness.sh eval-argo-telemetry-crate-surface` reported only authored-SHA distance drift; current crate scope still matched the brief.
- `rustfmt --check crates/argo-telemetry/tests/surface_contract.rs`
- `cargo test -p argo-telemetry --tests`
- `cargo clippy -p argo-telemetry --all-targets -- -D warnings`

Boundary note: `cargo fmt --check -p argo-telemetry` also reports pre-existing rustfmt drift in `crates/argo-telemetry/src/cardinality.rs` and `crates/argo-telemetry/src/prometheus_registry_tests.rs`, both outside this brief's may-touch list.
