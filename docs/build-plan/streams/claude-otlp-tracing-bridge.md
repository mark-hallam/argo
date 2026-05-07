# OTLP tracing bridge — Phase C follow-up to #938

**Owner**: claude. **Depends on**: #938 (mission-otlp-exporter, merged).

> SOURCE: production hardening Phase C. PR #938 wired the OTLP
> exporter behind feature `otlp` and proved spans + counters can
> reach a collector. But the rest of the codebase emits
> `#[instrument]` spans through `tracing`, and those spans are
> recorded but NOT exported by #938's setup — there's no bridge
> from the `tracing` ecosystem to the `opentelemetry` ecosystem.
> This brief is that bridge.

## Scope

Wire `tracing-opentelemetry` as a feature-flagged subscriber so
that every existing `#[instrument]` span across the workspace
flows through the OTLP exporter automatically when feature
`otlp` is enabled.

After this lands:

- Operators flip `--features otlp` and set
  `OTEL_EXPORTER_OTLP_ENDPOINT`, and **all** existing
  `#[instrument]`-decorated functions (hundreds across the
  workspace) export to their collector with no source changes.
- Default-off behavior preserved: feature off → zero overhead,
  zero new deps in the binary.

Concretely:

1. Add `tracing-opentelemetry` as an optional dep in
   `argo-tracing/Cargo.toml`, gated by feature `otlp`.
2. New module `crates/argo-tracing/src/otlp_bridge.rs` exports a
   `init_otlp_layer()` function that builds the
   `OpenTelemetryLayer` wrapping the tracer from #938's
   exporter and returns it as a `Layer<Registry>` for the
   subscriber.
3. The init path in `argo-tracing/src/lib.rs` registers this
   layer alongside the existing fmt layer when feature is on.

## Files you may touch

- `crates/argo-tracing/src/otlp_bridge.rs` (NEW) — bridge module
  with `init_otlp_layer()`; ~80 LOC
- `crates/argo-tracing/src/lib.rs` — narrowly: feature-gated
  module include + subscriber registration
- `crates/argo-tracing/Cargo.toml` — narrowly: add optional dep
  `tracing-opentelemetry`; extend feature `otlp = [..., "dep:tracing-opentelemetry"]`
- `crates/argo-tracing/src/otlp_bridge_tests.rs` (NEW) —
  3 tests: bridge no-op when feature off, layer attaches when
  feature on, span attributes propagate
- `docs/skycot/RUNBOOK.md` — narrowly: extend the existing
  Telemetry section (added by #938) with one paragraph noting
  that `#[instrument]` spans are now auto-exported

## Files you must NOT touch

- `argo-telemetry` crate — #938's exporter is the source of the
  tracer; this brief consumes it, doesn't replace it.
- Any `#[instrument]` call site across the workspace — they
  already work via the bridge for free.

## Acceptance criteria

- [ ] `tracing-opentelemetry` is an optional dep gated by `otlp`
- [ ] `init_otlp_layer()` returns a `Layer<Registry>`
- [ ] Default build (`cargo build`) does not pull
      `tracing-opentelemetry` into the dependency graph
- [ ] `cargo build --features otlp -p argo-tracing` clean
- [ ] An `#[instrument]` span emitted from any test crate
      reaches a mock collector when feature is on (integration
      test using the same mock harness as #938)
- [ ] 3 unit tests + 1 integration test
- [ ] RUNBOOK Telemetry section extended (~5 lines)
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-tracing` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-otlp-tracing-bridge`
- PR title: `tracing: bridge #[instrument] spans to OTLP exporter (claude-otlp-tracing-bridge)`

## Notes for the implementer

- `OpenTelemetryLayer::new(tracer)` is the standard pattern; the
  tracer comes from #938's `TracerProvider`.
- Do NOT introduce a global `Lazy` tracer here — `argo-tracing`
  already has the init path; just add the layer.
- Span attributes from `#[instrument(fields(...))]` should
  propagate as OTLP attributes — verify in the integration test
  with at least one custom field.
