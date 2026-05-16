//! Gateway telemetry boot helpers kept out of `boot.rs` so the
//! lifecycle file stays below the 500-LOC ceiling.

use argo_config::{ArgoConfig, ErrorReportingMode};
use argo_telemetry::error_reporting::{
    init_error_reporting, ErrorReportingConfig, ErrorReportingGuard,
};

pub(super) fn init_gateway_error_reporting(cfg: &ArgoConfig) -> Option<ErrorReportingGuard> {
    if matches!(cfg.error_reporting.mode, ErrorReportingMode::Disabled) {
        return None;
    }
    match init_error_reporting(&GatewayErrorReportingConfig(&cfg.error_reporting)) {
        Ok(guard) => Some(guard),
        Err(e) => {
            tracing::warn!(target: "gateway::error_reporting", error = %e, "error reporting disabled");
            None
        }
    }
}

struct GatewayErrorReportingConfig<'a>(&'a argo_config::ErrorReportingConfig);

impl ErrorReportingConfig for GatewayErrorReportingConfig<'_> {
    fn mode(&self) -> &str {
        self.0.mode.as_str()
    }

    fn dsn(&self) -> &str {
        &self.0.dsn
    }

    fn otlp_endpoint(&self) -> &str {
        &self.0.otlp_endpoint
    }

    fn pseudonym_salt(&self) -> &str {
        &self.0.pseudonym_salt
    }
}
