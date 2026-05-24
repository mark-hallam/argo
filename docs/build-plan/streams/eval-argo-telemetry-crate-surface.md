# eval-argo-telemetry-crate-surface

**Owner**: claude
**Priority**: P2 - observability substrate audit
**Authored against**: origin/main `2860f85e1` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-argo-telemetry-crate-surface`.

## Source

The mission-vs-code gap manifest still marks `argo-telemetry` as `NOT AUDITED`. The crate owns span/metric taxonomy, telemetry sinks, cardinality protection, Prometheus registry helpers, OTLP feature gates, PII scrubbers, and structured tracing context.

## Goal

Audit and pin the `argo-telemetry` public surface so observability names, sink behavior, label-cardinality defense, and optional OTLP exports cannot drift silently.

## May Touch

- `crates/argo-telemetry/tests/surface_contract.rs` (NEW)
- `crates/argo-telemetry/tests/` (NEW sibling tests if needed)
- `docs/skycot/evals/scheduled/2026-05-24/eval-argo-telemetry-crate-surface.md` (NEW report)
- `docs/build-plan/BUILD_STATUS.md`

## Must Not Touch

- Do not change production `argo-telemetry/src/**` code unless a contract test exposes an obvious compile-only drift and the fix is tiny.
- Do not change gateway `/metrics` or `/openmetrics` handlers.
- Do not require a live OTLP collector or network.

## Acceptance

- [ ] Pin exported telemetry types, traits, constants, and public modules from `crates/argo-telemetry/src/lib.rs`.
- [ ] Assert `spans::all()` and metric constants include the mission taxonomy names workers and gateway code rely on.
- [ ] Cover `NoopTelemetry` and `InMemoryTelemetry` sink behavior for counters, histograms, and spans.
- [ ] Cover cardinality cap behavior with a same-label-family overflow case.
- [ ] Cover PII scrubber behavior on at least one secret/email-like sample if the API exposes it publicly.
- [ ] Write an eval report with verdict, gaps found, and follow-up brief names only for real findings.
- [ ] `cargo test -p argo-telemetry --tests` passes, or the PR body records any unrelated clean-main failure with a focused green command.

## Notes

This is a surface-contract audit, not a route-wiring brief. Leave any gateway handler changes to a separate fix brief.
