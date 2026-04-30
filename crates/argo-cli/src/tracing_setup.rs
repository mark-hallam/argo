//! `tracing-subscriber` init for the argo binary.
//!
//! Default level is `warn` so users see hard errors but no
//! verbose chatter. Override via `RUST_LOG=info` (or any standard
//! env-filter directive) to surface the in-loop spans/metrics
//! emitted by [`argo_telemetry::TracingTelemetry`].
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Initialise the global subscriber. Idempotent — safe to call
/// from multiple entry points (the binary's `main` and tests).
pub fn init() {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn,argo=info"));
    let _ = tracing_subscriber::registry()
        .with(fmt::layer().with_target(false))
        .with(filter)
        .try_init();
}
