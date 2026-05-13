//! `tracing-subscriber` init for the argo binary.
//!
//! Default level is `warn` so users see hard errors but no
//! verbose chatter. Override via `RUST_LOG=info` (or any standard
//! env-filter directive) to surface the in-loop spans/metrics
//! emitted by [`argo_telemetry::TracingTelemetry`].
//!
//! By default, the secrets-scrubbing layer
//! ([`crate::secrets_scrubber::SecretsScrubberLayer`]) is wired into
//! the chain so bearer tokens, API keys, passwords, and JWTs cannot
//! leak through `tracing::*!` call sites. Set
//! `ARGO_TRACING_SCRUB=0` for local debug only — it falls back to
//! plain `fmt::layer()` output.
//!
//! When `[telemetry.context].enabled = true` (env override
//! `ARGO_TELEMETRY_CONTEXT=1`), the
//! [`argo_telemetry::span_context::ContextInjectorLayer`] is layered
//! ABOVE the scrubber so every event also produces a structured
//! `tenant_id=… worker_id=… session_id=…` line for SRE log-grep.
//! When `enabled = false` (default), the Layer is not constructed —
//! true zero-overhead path per the brief.
use argo_telemetry::span_context::ContextInjectorLayer;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use crate::secrets_scrubber::{self, SecretsScrubberLayer};

/// Env override for `[telemetry.context].enabled` — `1`/`true`
/// enables the [`ContextInjectorLayer`] without needing an
/// `argo.toml` change (useful for tests + ad-hoc grepping).
pub const ENV_CONTEXT_ENABLED: &str = "ARGO_TELEMETRY_CONTEXT";

/// Env override for `[telemetry.context].emit_anon_tenant_as`.
pub const ENV_CONTEXT_ANON: &str = "ARGO_TELEMETRY_CONTEXT_ANON";

fn context_enabled_from_env() -> bool {
    matches!(
        std::env::var(ENV_CONTEXT_ENABLED).ok().as_deref(),
        Some("1" | "true" | "TRUE" | "yes")
    )
}

fn context_anon_from_env() -> String {
    std::env::var(ENV_CONTEXT_ANON).unwrap_or_else(|_| "anon".to_string())
}

/// Initialise the global subscriber. Idempotent — safe to call
/// from multiple entry points (the binary's `main` and tests).
pub fn init() {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn,argo=info"));
    let ctx_layer = context_enabled_from_env()
        .then(|| ContextInjectorLayer::new(context_anon_from_env()));
    if secrets_scrubber::enabled() {
        let _ = tracing_subscriber::registry()
            .with(ctx_layer)
            .with(SecretsScrubberLayer::new())
            .with(filter)
            .try_init();
    } else {
        eprintln!("argo: ARGO_TRACING_SCRUB=0 — secrets scrubbing DISABLED");
        let _ = tracing_subscriber::registry()
            .with(ctx_layer)
            .with(fmt::layer().with_target(false))
            .with(filter)
            .try_init();
    }
}
