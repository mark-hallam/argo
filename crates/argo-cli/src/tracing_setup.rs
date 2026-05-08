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
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use crate::secrets_scrubber::{self, SecretsScrubberLayer};

/// Initialise the global subscriber. Idempotent — safe to call
/// from multiple entry points (the binary's `main` and tests).
pub fn init() {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn,argo=info"));
    if secrets_scrubber::enabled() {
        let _ = tracing_subscriber::registry()
            .with(SecretsScrubberLayer::new())
            .with(filter)
            .try_init();
    } else {
        eprintln!("argo: ARGO_TRACING_SCRUB=0 — secrets scrubbing DISABLED");
        let _ = tracing_subscriber::registry()
            .with(fmt::layer().with_target(false))
            .with(filter)
            .try_init();
    }
}
