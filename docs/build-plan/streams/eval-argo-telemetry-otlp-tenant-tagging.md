# eval-argo-telemetry-otlp-tenant-tagging — audit argo-telemetry OTLP exporter + tenant-id tagging

**Owner**: claude or codex. **Depends on**: nothing.
**Authored against**: origin/main `289d3077f` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-argo-telemetry-otlp-tenant-tagging`.

## Why this is P2

Mission gap manifest row `argo-telemetry` is 🔴 NOT AUDITED. The OTLP (OpenTelemetry Protocol — vendor-neutral format for metrics, traces, and logs) exporter was added in #938. Multi-tenant requirement: every emitted span/metric must carry `tenant_id` as a resource attribute so downstream observability can attribute traffic correctly. Need to verify.

## Scope (read-only audit)

1. **Surface inventory** of `crates/argo-telemetry/src/lib.rs`.
2. **Tenant tagging check** — find the OTLP exporter setup; assert every span builder or metric counter accepts and binds `tenant_id` as a resource attribute.
3. **Boot wiring** — find where the telemetry exporter is initialised in the binary boot path; confirm it's called.
4. **Pin tests** at `crates/argo-telemetry/tests/surface_pin_test.rs` (~100-150 LOC) covering surface + tenant-bind contract.
5. **Report** at `docs/skycot/evals/2026-05-25-argo-telemetry-otlp-tenant-tagging.md` (~500-800 words).

## Files you may touch

- `crates/argo-telemetry/tests/surface_pin_test.rs` — new.
- `docs/skycot/evals/2026-05-25-argo-telemetry-otlp-tenant-tagging.md` — new.

## Files you must NOT touch

- `crates/argo-telemetry/src/**`.

## Acceptance criteria

- [ ] Surface table.
- [ ] Yes/No verdict on tenant-tagging with citations.
- [ ] Pin tests pass.
- [ ] ≤5 follow-up briefs.

## Branch + PR

- Branch: `claude-eval-argo-telemetry-otlp-tenant-tagging` or `codex-…`.
- PR title: `eval(telemetry): audit argo-telemetry OTLP + tenant tagging (eval-argo-telemetry-otlp-tenant-tagging)`.
