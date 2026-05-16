//! Opt-in runtime error reporting.
//!
//! This module owns the backend-neutral setup used by gateway boot.
//! The shipped default is disabled; a configured backend only starts
//! when the operator sets `[error_reporting]` and builds the matching
//! crate feature.

use crate::pii_scrubber::{pseudonymize_tenant, scrub_pii};
use std::fmt;
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::registry::LookupSpan;

/// Tracing-error layer re-export for subscribers that can install it
/// before global subscriber initialization.
pub use tracing_error::ErrorLayer as TracingErrorLayer;

/// Minimal config contract consumed by telemetry without depending on
/// the config crate. Gateway boot adapts `argo_config` into this
/// trait, avoiding a workspace dependency cycle.
pub trait ErrorReportingConfig {
    /// Backend mode: `disabled`, `otlp`, or `sentry`.
    fn mode(&self) -> &str;
    /// Sentry/Glitchtip DSN.
    fn dsn(&self) -> &str;
    /// OTLP collector base endpoint.
    fn otlp_endpoint(&self) -> &str;
    /// HMAC salt for tenant pseudonymization.
    fn pseudonym_salt(&self) -> &str;
}

/// Owned config for callers that build an error-reporting layer.
#[derive(Debug, Clone, Default)]
pub struct ErrorReportingOptions {
    /// Backend mode: `disabled`, `otlp`, or `sentry`.
    pub mode: String,
    /// Sentry/Glitchtip DSN.
    pub dsn: String,
    /// OTLP collector base endpoint.
    pub otlp_endpoint: String,
    /// HMAC salt for tenant pseudonymization.
    pub pseudonym_salt: String,
}

impl ErrorReportingConfig for ErrorReportingOptions {
    fn mode(&self) -> &str {
        &self.mode
    }
    fn dsn(&self) -> &str {
        &self.dsn
    }
    fn otlp_endpoint(&self) -> &str {
        &self.otlp_endpoint
    }
    fn pseudonym_salt(&self) -> &str {
        &self.pseudonym_salt
    }
}

/// Holds backend guards for the lifetime of the process.
#[derive(Default)]
pub struct ErrorReportingGuard {
    #[cfg(feature = "sentry")]
    _sentry: Option<sentry::ClientInitGuard>,
    #[cfg(feature = "otlp")]
    _otlp: Option<crate::otlp_exporter::ShutdownGuard>,
}

/// Initialize the configured error-reporting backend.
///
/// `disabled` returns a guard with no active exporters. `sentry` and
/// `otlp` require their corresponding Cargo feature; when a feature is
/// absent this returns [`ErrorReportingInitError::FeatureDisabled`].
///
/// # Errors
/// Returns an initialization error for missing backend config, missing
/// Cargo features, or exporter setup failures.
pub fn init_error_reporting(
    config: &impl ErrorReportingConfig,
) -> Result<ErrorReportingGuard, ErrorReportingInitError> {
    match normalize_mode(config.mode()) {
        BackendMode::Disabled => Ok(ErrorReportingGuard::default()),
        BackendMode::Sentry => init_sentry(config),
        BackendMode::Otlp => init_otlp(config),
    }
}

/// Build a layer that captures `ERROR` events for the active backend.
///
/// Callers that own subscriber construction can compose this with the
/// tracing subscriber. The gateway brief keeps the existing subscriber
/// untouched, so gateway boot only calls [`init_error_reporting`].
#[must_use]
pub fn error_reporting_layer(config: ErrorReportingOptions) -> ErrorReporterLayer {
    ErrorReporterLayer { config }
}

/// Captures `ERROR`-level tracing events and forwards scrubbed details
/// to the configured backend.
#[derive(Clone)]
pub struct ErrorReporterLayer {
    config: ErrorReportingOptions,
}

impl<S> Layer<S> for ErrorReporterLayer
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        if *event.metadata().level() != Level::ERROR {
            return;
        }
        let mut visitor = ErrorEventVisitor::default();
        event.record(&mut visitor);
        let message = if visitor.message.is_empty() {
            event.metadata().target().to_string()
        } else {
            visitor.message
        };
        capture_error_message(&self.config, &message, visitor.tenant_id.as_deref());
    }
}

/// Capture one error message through the configured backend.
pub fn capture_error_message(
    config: &impl ErrorReportingConfig,
    message: &str,
    tenant_id: Option<&str>,
) {
    if matches!(normalize_mode(config.mode()), BackendMode::Disabled) {
        return;
    }
    let scrubbed = scrub_pii(message);
    let tenant = tenant_id.map(|t| pseudonymize_tenant(t, config.pseudonym_salt()));
    capture_backend(normalize_mode(config.mode()), &scrubbed, tenant.as_deref());
}

#[cfg(feature = "sentry")]
fn init_sentry(
    config: &impl ErrorReportingConfig,
) -> Result<ErrorReportingGuard, ErrorReportingInitError> {
    let dsn = config.dsn().trim();
    if dsn.is_empty() {
        return Err(ErrorReportingInitError::MissingConfig("dsn"));
    }
    let guard = sentry::init((
        dsn.to_string(),
        sentry::ClientOptions {
            attach_stacktrace: true,
            ..Default::default()
        },
    ));
    Ok(ErrorReportingGuard {
        _sentry: Some(guard),
        #[cfg(feature = "otlp")]
        _otlp: None,
    })
}

#[cfg(not(feature = "sentry"))]
fn init_sentry(
    _config: &impl ErrorReportingConfig,
) -> Result<ErrorReportingGuard, ErrorReportingInitError> {
    Err(ErrorReportingInitError::FeatureDisabled("sentry"))
}

#[cfg(feature = "otlp")]
fn init_otlp(
    config: &impl ErrorReportingConfig,
) -> Result<ErrorReportingGuard, ErrorReportingInitError> {
    let endpoint = config.otlp_endpoint().trim();
    if endpoint.is_empty() {
        return Err(ErrorReportingInitError::MissingConfig("otlp_endpoint"));
    }
    std::env::set_var("OTEL_EXPORTER_OTLP_ENDPOINT", endpoint);
    let guard = crate::otlp_exporter::init()
        .map_err(|e| ErrorReportingInitError::Exporter(e.to_string()))?;
    Ok(ErrorReportingGuard {
        #[cfg(feature = "sentry")]
        _sentry: None,
        _otlp: Some(guard),
    })
}

#[cfg(not(feature = "otlp"))]
fn init_otlp(
    _config: &impl ErrorReportingConfig,
) -> Result<ErrorReportingGuard, ErrorReportingInitError> {
    Err(ErrorReportingInitError::FeatureDisabled("otlp"))
}

fn capture_backend(mode: BackendMode, message: &str, tenant: Option<&str>) {
    match mode {
        BackendMode::Sentry => capture_sentry(message, tenant),
        BackendMode::Otlp => capture_otlp(message, tenant),
        BackendMode::Disabled => {}
    }
}

#[cfg(feature = "sentry")]
fn capture_sentry(message: &str, tenant: Option<&str>) {
    sentry::with_scope(
        |scope| {
            if let Some(t) = tenant {
                scope.set_tag("tenant", t);
            }
        },
        || {
            sentry::capture_message(message, sentry::Level::Error);
        },
    );
}

#[cfg(not(feature = "sentry"))]
fn capture_sentry(_message: &str, _tenant: Option<&str>) {}

#[cfg(feature = "otlp")]
fn capture_otlp(message: &str, tenant: Option<&str>) {
    use opentelemetry::trace::{Span as _, Status, Tracer as _};
    use opentelemetry::{global, KeyValue};

    let tracer = global::tracer(crate::otlp_exporter::TRACER_NAME);
    let mut span = tracer.start("error.reporting");
    span.set_status(Status::error(message.to_string()));
    span.set_attribute(KeyValue::new("error.message", message.to_string()));
    if let Some(t) = tenant {
        span.set_attribute(KeyValue::new("tenant_id", t.to_string()));
    }
    span.add_event(
        "exception",
        vec![KeyValue::new("exception.message", message.to_string())],
    );
}

#[cfg(not(feature = "otlp"))]
fn capture_otlp(_message: &str, _tenant: Option<&str>) {}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BackendMode {
    Disabled,
    Otlp,
    Sentry,
}

fn normalize_mode(mode: &str) -> BackendMode {
    match mode {
        "otlp" => BackendMode::Otlp,
        "sentry" => BackendMode::Sentry,
        _ => BackendMode::Disabled,
    }
}

#[derive(Default)]
struct ErrorEventVisitor {
    message: String,
    tenant_id: Option<String>,
}

impl ErrorEventVisitor {
    fn store(&mut self, field: &Field, value: String) {
        match field.name() {
            "message" => self.message = value,
            "tenant_id" if !value.is_empty() => self.tenant_id = Some(value),
            _ => {}
        }
    }
}

impl Visit for ErrorEventVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.store(field, value.to_string());
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.store(field, format!("{value:?}"));
    }
}

/// Error-reporting initialization failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorReportingInitError {
    /// Backend was configured but the binary was built without the
    /// matching Cargo feature.
    FeatureDisabled(&'static str),
    /// Required backend field is empty.
    MissingConfig(&'static str),
    /// Backend exporter initialization failed.
    Exporter(String),
}

impl fmt::Display for ErrorReportingInitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FeatureDisabled(feature) => {
                write!(f, "error reporting feature '{feature}' is not enabled")
            }
            Self::MissingConfig(field) => write!(f, "[error_reporting].{field} is required"),
            Self::Exporter(msg) => write!(f, "error reporting exporter init failed: {msg}"),
        }
    }
}

impl std::error::Error for ErrorReportingInitError {}
