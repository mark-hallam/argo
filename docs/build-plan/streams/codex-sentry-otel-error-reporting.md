# codex-sentry-otel-error-reporting — gateway + SPA error reporting (Wave 4 P1)

**Owner**: codex. **Depends on**: `codex-ci-workflows-reactivate` MERGED (#1732).
**Authored against**: origin/main `054d8b0a` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-sentry-otel-error-reporting`.
**Source**: §4 weekend plan Wave 4 — production-grade error visibility before public launch.

## Mission alignment

- **Observability** (CLAUDE.md priority #2) — exceptions today silently log + die; need aggregation + alerting.
- **Reliability** (#1) — find issues before users report them.

## Scope

P1. Two error-reporting surfaces, both gated on operator config (no auto-send to third party without explicit opt-in).

1. **Gateway-side error reporting.**
   - Hook into existing `tracing` subscriber via a new `tracing-error` layer.
   - On `ERROR`-level events: capture event + span context + tenant_id + worker_id (if in context) + error chain (`std::error::Error::source` walk).
   - Three exporter backends, picked via `[error_reporting]` config block:
     - `disabled` (default) — no-op.
     - `otlp` — OpenTelemetry OTLP/gRPC exporter to operator-specified endpoint (works with Grafana Cloud, Honeycomb, etc.).
     - `sentry` — Sentry SDK via `sentry-tracing` crate; operator provides DSN.
   - Each exporter is feature-flagged in Cargo.toml so the binary doesn't pull SDK deps unless enabled.
   - PII scrubbing: tenant_id pseudonymized via HMAC before send (operator-configurable salt). Email / phone / token-shaped strings redacted via regex.

2. **SPA-side error reporting.**
   - Next.js error boundary in `web/app/error.tsx` and `marketing/app/error.tsx` (from `claude-error-page-design-system`) wired to report.
   - Same backend choices: disabled / otlp / sentry. Config served via `/api/v1/meta/error-reporting` (returns `{ enabled: bool, dsn?: string, otlp_endpoint?: string }`).
   - Source-map upload: build step uploads sourcemaps to chosen backend so stacktraces are de-minified.
   - Tracks unhandled promise rejections + global errors.

LOC: ~350.

## Files you may touch

**Backend:**
- `crates/argo-telemetry/Cargo.toml` — add `sentry`, `sentry-tracing`, `opentelemetry-otlp` as optional dependencies behind `sentry` + `otlp` features. ≤10 modified LOC.
- `crates/argo-telemetry/src/error_reporting.rs` (NEW) — public `init_error_reporting(config: &ErrorReportingConfig)` + the tracing-error layer. ≤180 LOC.
- `crates/argo-telemetry/src/pii_scrubber.rs` (NEW) — regex-based redaction. ≤80 LOC.
- `crates/argo-config/src/lib.rs` (or wherever the `[error_reporting]` schema lives) — add config struct with `mode`, `dsn`, `otlp_endpoint`, `pseudonym_salt`. ≤40 modified LOC.
- `crates/argo-cli/src/handlers/gateway/boot.rs` — call `init_error_reporting()` early in boot path. ≤10 modified LOC.
- `crates/argo-cli/src/handlers/gateway/routes_meta_error_reporting.rs` (NEW) — `/api/v1/meta/error-reporting` GET. ≤40 LOC.

**SPA:**
- `web/lib/error-reporting/init.ts` (NEW) — initializes Sentry / OTLP client based on `/api/v1/meta/error-reporting` response. ≤90 LOC.
- `web/lib/error-reporting/init.test.ts` (NEW). ≤40 LOC.
- `web/app/layout.tsx` — wire init at root. ≤8 modified LOC.
- `web/next.config.js` — sourcemap upload config (conditional on env). ≤30 modified LOC.
- `marketing/app/layout.tsx` — same init wiring. ≤8 modified LOC.

**Tests:**
- `crates/argo-telemetry/src/pii_scrubber_tests.rs` (NEW) — verify redaction rules. ≤80 LOC.

## Files you must NOT touch

- Existing `tracing-subscriber` setup — the error layer composes with what's already there.
- Audit chain (`argo-audit`) — that's tamper-evident operational audit, different concern from runtime error reporting.
- `argo-store` migrations — error reporting is push-out, no persistence.
- The capability gate or auth — error reporting is operator infra, not user-facing.

## Acceptance criteria

- [ ] `[error_reporting]` config block parses `mode = "disabled" | "otlp" | "sentry"`.
- [ ] Default config: `mode = "disabled"` — zero data leaves the binary.
- [ ] Building with `--features otlp` includes OTLP exporter; without it, the OTLP path is `unreachable!()` with compile-time guard.
- [ ] Same for `--features sentry`.
- [ ] Errors raised in a request handler with tenant context include the (pseudonymized) tenant_id in the captured event.
- [ ] PII scrubber redacts: email addresses, phone numbers, things matching `eyJ[A-Za-z0-9_-]{20,}` (JWTs), `sk-[A-Za-z0-9]{20,}` (provider keys).
- [ ] Pseudonymization is consistent: same tenant_id → same pseudonym (HMAC stable).
- [ ] SPA error boundary captures + reports unhandled errors.
- [ ] Unhandled promise rejection captured.
- [ ] Sourcemap upload runs only in CI build (`process.env.CI === 'true'`).
- [ ] `cargo test -p argo-telemetry` passes.
- [ ] `cargo clippy -p argo-telemetry --all-targets -- -D warnings` clean.
- [ ] `cargo build --features otlp` and `cargo build --features sentry` both succeed.
- [ ] `pnpm test && pnpm typecheck` clean for web + marketing.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `codex-sentry-otel-error-reporting`
- Title: `feat(telemetry): opt-in OTLP + Sentry error reporting with PII scrubbing (codex-sentry-otel-error-reporting)`
- Worker verification: crate-scoped clippy + tests + feature-flag builds + workspace clippy per SWARM_PROTOCOL.md.

## Hidden constraints

- **Default off.** Anthropic's pattern is don't-phone-home-without-opt-in. Operator config gates everything.
- **Cardinality limits.** OTLP backends bill on cardinality. Don't include high-cardinality dimensions (raw worker_id, request_id) as labels — those go in event body, not span attributes.
- **PII regex is conservative.** Better to over-redact than leak. Document the regex set in the brief — operator can review.
- **Feature flags real.** If the operator doesn't enable Sentry, the `sentry` crate shouldn't be in the resulting binary. Test via `cargo bloat` or `cargo tree --no-default-features`.
- **Sentry DSN is not a secret per Sentry's threat model**, but treat it like one anyway — env var, not committed config.
- **Sourcemap upload needs an auth token (Sentry / Glitchtip)** — env var, not committed. The build step skips upload if token absent (and logs a warning).
