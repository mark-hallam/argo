# Mission production blocker #3 — wire OTLP exporter for telemetry

**Owner**: claude. **Depends on**: none.

> SOURCE: deep-research findings tick 198, Section 2, item #30.
> argo-telemetry's tracing_sink is a span-enter-and-drop stub —
> the comment block is candid: "Real OTLP wiring lives in a
> follow-up that swaps SpanGuard for a typed handle." This brief
> is that follow-up.

## Scope

[`crates/argo-telemetry/src/tracing_sink.rs`](crates/argo-telemetry/src/tracing_sink.rs)
currently does:

```rust
let _enter = tracing::info_span!("argo", span = name, attrs = %attrs).entered();
// drop the entered guard here, which closes the span on the same line.
drop(_enter);
SpanGuard::empty()
```

This means no nested context, no parent/child spans, no real
OTLP wiring. Counters and histograms emit as `tracing::info!`
events with no aggregation.

This brief implements the typed span guard + OTLP exporter:

1. Replace `SpanGuard::empty()` with a real handle that owns
   the entered span across the caller's lifetime.
2. Add an `opentelemetry-otlp` exporter behind feature
   `otlp` (default off; when enabled, reads
   `OTEL_EXPORTER_OTLP_ENDPOINT` env var; default
   `http://localhost:4317`).
3. Wire counters + histograms to the OTLP meter when feature
   is enabled; fall back to the existing `tracing::info!` path
   when disabled.
4. Document operator setup in
   `docs/skycot/RUNBOOK.md` (one paragraph: enable feature,
   set env, point at OTel collector / Honeycomb / Datadog).

Default-off feature flag preserves zero-deps behavior for the
single-binary distribution.

## Files you may touch

- `crates/argo-telemetry/src/tracing_sink.rs` — narrowly: real
  span guard implementation (the SpanGuard type can hold an
  `Option<tracing::span::EnteredSpan>` behind a Box for the
  Send-bound case — implementer's choice)
- `crates/argo-telemetry/src/otlp_exporter.rs` (NEW, feature-gated) —
  OTLP setup + meter wiring
- `crates/argo-telemetry/src/lib.rs` — narrowly: feature gate +
  re-export
- `crates/argo-telemetry/Cargo.toml` — narrowly: optional deps
  `opentelemetry`, `opentelemetry-otlp`, `opentelemetry_sdk`;
  feature `otlp = ["dep:opentelemetry", ...]`
- `crates/argo-telemetry/src/lib_tests.rs` — narrowly: 3 tests
  (default path unchanged; feature-on path exports; nested span
  parent/child preserved)
- `docs/skycot/RUNBOOK.md` — narrowly: append a "Telemetry"
  subsection (~15 lines) explaining feature flag + env vars +
  collector pointers

## Files you must NOT touch

- Any other crate's `tracing` calls — they continue to work via
  the existing tracing-subscriber output unchanged
- `crates/argo-cli/src/handlers/gateway/metrics.rs` — that file's
  manual Prometheus aggregation is a separate concern (it's at
  545 LOC and over the ceiling; refactor is a different brief)

## Acceptance criteria

- [ ] SpanGuard is a real typed handle, not an empty drop
- [ ] Nested spans preserve parent/child relationship
- [ ] `otlp` feature default-off; build green without it
- [ ] `otlp` feature on: spans + counters export to OTLP endpoint
- [ ] env `OTEL_EXPORTER_OTLP_ENDPOINT` honored (default
      `http://localhost:4317`)
- [ ] env `OTEL_SERVICE_NAME` honored (default `argo`)
- [ ] 3 unit tests + 1 integration test using a mock collector
      (gated on env or `wiremock`)
- [ ] RUNBOOK updated with a brief operator setup section
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-telemetry` clean
- [ ] `cargo build --features otlp -p argo-telemetry` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-mission-otlp-exporter`
- PR title: `telemetry: real span guards + OTLP exporter feature (mission-otlp-exporter)`

## Notes for the implementer

- Use `opentelemetry_sdk::trace::TracerProvider` for span export;
  do not roll your own.
- The `SpanGuard` in this codebase has an opaque shape so callers
  can hold it across `await` points. If `EnteredSpan` is `!Send`,
  use `tracing::Span` plus manual entry/exit at the OTLP boundary
  (the `tracing-opentelemetry` crate provides this bridge).
- DO NOT fold the Prometheus refactor (metrics.rs over-LOC) into
  this brief. That's a separate cleanup item in MISSION_BACKLOG.md.
- After this lands, real production observability is deployable.
