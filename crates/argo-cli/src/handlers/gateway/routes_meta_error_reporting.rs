//! `GET /api/v1/meta/error-reporting` — browser-safe error
//! reporting config for the SPA and marketing site.

use argo_config::{ArgoConfig, ErrorReportingMode};
use serde_json::json;

pub(super) fn response() -> (&'static str, String) {
    let cfg = load_cfg();
    let er = cfg.error_reporting;
    let body = match er.mode {
        ErrorReportingMode::Disabled => json!({
            "enabled": false,
            "mode": "disabled",
        }),
        ErrorReportingMode::Sentry if !er.dsn.trim().is_empty() => json!({
            "enabled": true,
            "mode": "sentry",
            "dsn": er.dsn,
        }),
        ErrorReportingMode::Otlp if !er.otlp_endpoint.trim().is_empty() => json!({
            "enabled": true,
            "mode": "otlp",
            "otlp_endpoint": er.otlp_endpoint,
        }),
        mode => json!({
            "enabled": false,
            "mode": mode.as_str(),
        }),
    };
    ("200 OK", body.to_string())
}

fn load_cfg() -> ArgoConfig {
    let cwd = match std::env::current_dir() {
        Ok(c) => c,
        Err(_) => return ArgoConfig::default(),
    };
    let mut cfg = ArgoConfig::load(&cwd.join("argo.toml")).unwrap_or_default();
    if let Ok(env_cfg) = ArgoConfig::from_env() {
        cfg.merge(env_cfg);
    }
    cfg
}
