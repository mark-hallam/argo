//! Telemetry config sections: `[telemetry]` and
//! `[telemetry.log_shipping]`. Extracted from [`crate::sections`] to
//! keep that module under the 500-LOC ceiling. Re-exported through
//! `sections` so the public API is unchanged.
use crate::defaults;
use serde::{Deserialize, Serialize};

/// `[telemetry]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TelemetryConfig {
    /// Master enable.
    pub enabled: bool,
    /// `stdout`, `otlp`, or `none`.
    pub exporter: String,
    /// Exporter endpoint (e.g. OTLP HTTP URL).
    pub endpoint: String,
    /// `[telemetry.log_shipping]` — optional HTTP log-shipping target
    /// (Stream `mission-log-shipping-target-docs`). Off by default.
    pub log_shipping: TelemetryLogShippingConfig,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: defaults::DEFAULT_TELEMETRY_ENABLED,
            exporter: defaults::DEFAULT_TELEMETRY_EXPORTER.into(),
            endpoint: String::new(),
            log_shipping: TelemetryLogShippingConfig::default(),
        }
    }
}

/// `[telemetry.log_shipping]` — direct HTTP log shipping target for
/// operators who cannot run a sidecar log-agent (heroku-style
/// hosting, FaaS). For sidecar deployments (Loki + Promtail,
/// Datadog-agent, CloudWatch-agent, Splunk forwarder, Vector.dev),
/// see `docs/skycot/runbook/log-shipping.md` instead.
///
/// Best-effort by design: HTTP errors warn-not-crash, queue overflow
/// drops the OLDEST event (newest events matter more for "what's
/// happening now"), no on-disk spool, no compression. Operators with
/// no-loss / high-volume requirements should use the sidecar recipes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TelemetryLogShippingConfig {
    /// Master enable for the HTTP log-shipping Layer. Off by default;
    /// when off the Layer is not installed and adds zero overhead.
    pub enabled: bool,
    /// Shipping mode. `http_target` is the only mode in v1; sidecar
    /// shipping is "file logging + agent tailing" and does not pass
    /// through this section.
    pub mode: String,
    /// HTTPS endpoint that accepts a POSTed JSON array of events.
    pub endpoint: String,
    /// Env var name holding the `Authorization` header value
    /// (`Bearer …`, `Splunk …`, etc.). Env-indirect so operators
    /// rotate without restart-and-edit-config.
    pub header_authorization_env: String,
    /// Max events per POST. Smaller batches reduce per-event
    /// latency; larger batches reduce HTTP overhead.
    pub batch_size: usize,
    /// Worst-case latency a buffered event sees before shipping,
    /// in seconds (whichever comes first: batch cap or interval).
    pub flush_interval_seconds: u64,
    /// Per-request timeout, in seconds. Hangs at the endpoint cannot
    /// stall the calling tracing-event path.
    pub timeout_seconds: u64,
    /// In-memory queue bound; when full, OLDEST events drop.
    pub queue_max: usize,
}

impl Default for TelemetryLogShippingConfig {
    fn default() -> Self {
        Self {
            enabled: defaults::DEFAULT_LOG_SHIPPING_ENABLED,
            mode: defaults::DEFAULT_LOG_SHIPPING_MODE.into(),
            endpoint: String::new(),
            header_authorization_env: defaults::DEFAULT_LOG_SHIPPING_AUTH_ENV.into(),
            batch_size: defaults::DEFAULT_LOG_SHIPPING_BATCH_SIZE,
            flush_interval_seconds: defaults::DEFAULT_LOG_SHIPPING_FLUSH_INTERVAL_SECS,
            timeout_seconds: defaults::DEFAULT_LOG_SHIPPING_TIMEOUT_SECS,
            queue_max: defaults::DEFAULT_LOG_SHIPPING_QUEUE_MAX,
        }
    }
}
