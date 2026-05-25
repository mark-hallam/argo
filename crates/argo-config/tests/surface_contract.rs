use argo_config::source_tracking::{secrets_display_allowed, ConfigSourceMap};
use argo_config::{
    defaults, load_from, ArgoConfig, AuthConfig, AuthSignupProtectionConfig, CodeExecutorConfig,
    ErrorReportingConfig, ErrorReportingMode, GatewayConfig, PlannerConfig, RuntimeConfig,
    SiemConfig, TelemetryConfig, DEFAULT_CODE_EXECUTOR_PROFILE, LEGACY_AUDIT_TENANT_SENTINEL,
};
use std::fs;
use std::path::Path;
use std::sync::Mutex;
use tempfile::TempDir;

static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn public_surface_exports_defaults_for_mission_relevant_sections() {
    let cfg = ArgoConfig::default();

    let _gateway: GatewayConfig = cfg.gateway.clone();
    assert_eq!(cfg.gateway.host, defaults::DEFAULT_GATEWAY_HOST);
    assert_eq!(cfg.gateway.port, defaults::DEFAULT_GATEWAY_PORT);
    assert_eq!(cfg.gateway.url, defaults::DEFAULT_GATEWAY_URL);

    let _auth: AuthConfig = cfg.auth.clone();
    let _signup: AuthSignupProtectionConfig = cfg.auth.signup_protection.clone();
    assert!(cfg.auth.trust_proxy_header);
    assert_eq!(cfg.auth.signup_protection.ip_limit_per_hour, 5);
    assert_eq!(cfg.auth.signup_protection.email_cooldown_secs, 60);

    let _telemetry: TelemetryConfig = cfg.telemetry.clone();
    assert_eq!(cfg.telemetry.enabled, defaults::DEFAULT_TELEMETRY_ENABLED);
    assert_eq!(cfg.telemetry.exporter, defaults::DEFAULT_TELEMETRY_EXPORTER);
    assert_eq!(
        cfg.telemetry.prometheus.cardinality_cap.histogram_default,
        defaults::DEFAULT_CARDINALITY_CAP_HISTOGRAM
    );
    assert_eq!(
        cfg.telemetry
            .prometheus
            .cardinality_cap
            .counter_gauge_default,
        defaults::DEFAULT_CARDINALITY_CAP_COUNTER_GAUGE
    );

    let _siem: SiemConfig = cfg.siem.clone();
    assert!(!cfg.siem.enabled);
    assert_eq!(cfg.siem.token.expose(), "");
    assert_eq!(cfg.siem.token.to_string(), "[redacted]");

    let _executor: CodeExecutorConfig = cfg.code_executor.clone();
    assert_eq!(cfg.code_executor.profile, DEFAULT_CODE_EXECUTOR_PROFILE);
    assert!(cfg.code_executor.is_production());
    assert!(!cfg.code_executor.allow_local_in_production);

    let _planner: PlannerConfig = cfg.planner;
    assert!(cfg.planner.bias.enabled);
    assert!(cfg.planner.bias.prefer_per_vertical);
    assert_eq!(cfg.planner.bias.require_recent_within_days, 30);

    let _runtime: RuntimeConfig = cfg.runtime.clone();
    assert!(cfg.runtime.worker_resource_cage.enabled);
    assert_eq!(cfg.runtime.worker_resource_cage.memory_max_mib, 1024);
    assert_eq!(cfg.runtime.worker_resource_cage.cpu_weight, 100);

    let _error_reporting: ErrorReportingConfig = cfg.error_reporting.clone();
    assert_eq!(cfg.error_reporting.mode, ErrorReportingMode::Disabled);
    assert_eq!(ErrorReportingMode::Otlp.as_str(), "otlp");
    assert_ne!(LEGACY_AUDIT_TENANT_SENTINEL, defaults::DEFAULT_TENANT_ID);
}

#[test]
fn from_toml_parses_gateway_auth_telemetry_siem_executor_planner_and_worker_resource() {
    let cfg = ArgoConfig::from_toml(
        r#"
[gateway]
enabled = true
host = "0.0.0.0"
port = 9443
url = "https://gateway.example.test"

[auth.signup_protection]
ip_limit_per_hour = 11
email_cooldown_secs = 120

[auth.signup_protection.turnstile]
enabled = true
secret_key = "turnstile-secret"

[telemetry]
enabled = false
exporter = "otlp"
endpoint = "http://collector:4318"

[telemetry.prometheus.cardinality_cap]
histogram_default = 200
counter_gauge_default = 400

[telemetry.prometheus.cardinality_cap.per_metric]
provider_call_duration_seconds = 55

[siem]
enabled = true
backend = "splunk"
endpoint = "https://splunk.example.test"
token = "splunk-token"
sourcetype = "argo:audit"
interval_secs = 9

[code_executor]
profile = "dev"
allow_local_in_production = true

[planner.bias]
enabled = false
prefer_per_vertical = false
require_recent_within_days = 7

[runtime.worker_resource_cage]
enabled = false
memory_max_mib = 2048
cpu_weight = 250
"#,
    )
    .expect("config should parse");

    assert!(cfg.gateway.enabled);
    assert_eq!(cfg.gateway.host, "0.0.0.0");
    assert_eq!(cfg.gateway.port, 9443);
    assert_eq!(cfg.gateway.url, "https://gateway.example.test");

    assert_eq!(cfg.auth.signup_protection.ip_limit_per_hour, 11);
    assert_eq!(cfg.auth.signup_protection.email_cooldown_secs, 120);
    assert!(cfg.auth.signup_protection.turnstile.enabled);

    assert!(!cfg.telemetry.enabled);
    assert_eq!(cfg.telemetry.exporter, "otlp");
    assert_eq!(cfg.telemetry.endpoint, "http://collector:4318");
    assert_eq!(
        cfg.telemetry.prometheus.cardinality_cap.histogram_default,
        200
    );
    assert_eq!(
        cfg.telemetry
            .prometheus
            .cardinality_cap
            .per_metric
            .get("provider_call_duration_seconds"),
        Some(&55)
    );

    assert!(cfg.siem.enabled);
    assert_eq!(cfg.siem.backend, "splunk");
    assert_eq!(cfg.siem.token.expose(), "splunk-token");
    assert_eq!(cfg.siem.sourcetype, "argo:audit");

    assert_eq!(cfg.code_executor.profile, "dev");
    assert!(!cfg.code_executor.is_production());
    assert!(cfg.code_executor.allow_local_in_production);

    assert!(!cfg.planner.bias.enabled);
    assert!(!cfg.planner.bias.prefer_per_vertical);
    assert_eq!(cfg.planner.bias.require_recent_within_days, 7);

    assert!(!cfg.runtime.worker_resource_cage.enabled);
    assert_eq!(cfg.runtime.worker_resource_cage.memory_max_mib, 2048);
    assert_eq!(cfg.runtime.worker_resource_cage.cpu_weight, 250);
}

#[test]
fn layered_loader_precedence_and_source_tracking_are_stable() {
    with_scoped_env(
        &[
            ("ARGO_GATEWAY_PORT", Some("3333")),
            ("ARGO_TELEMETRY_EXPORTER", Some("env-exporter")),
            ("ARGO_TELEMETRY_ENABLED", None),
            ("ARGO_GATEWAY_URL", None),
            ("ARGO_DEFAULT_TENANT", None),
            ("ARGO_SHOW_SECRETS", Some("1")),
        ],
        |workspace, xdg| {
            write_user_config(
                xdg.path(),
                r#"
[gateway]
port = 1111
url = "http://user.example.test"

[telemetry]
exporter = "user-exporter"
"#,
            );
            fs::create_dir_all(workspace.path().join(".skycot")).expect("create .skycot");
            fs::write(
                workspace.path().join(".skycot/argo.toml"),
                r#"
[gateway]
port = 2222

[auth.signup_protection]
ip_limit_per_hour = 17

[telemetry]
enabled = false
"#,
            )
            .expect("write project config");
            fs::write(
                workspace.path().join("argo.toml"),
                "[gateway]\nport = 9999\n",
            )
            .expect("write legacy config");

            let cfg = load_from(workspace.path()).expect("load layered config");
            assert_eq!(cfg.gateway.port, 3333, "env beats project and user");
            assert_eq!(
                cfg.gateway.url, "http://user.example.test",
                "lower user file survives when project and env omit the field"
            );
            assert_eq!(cfg.auth.signup_protection.ip_limit_per_hour, 17);
            assert!(!cfg.telemetry.enabled);
            assert_eq!(cfg.telemetry.exporter, "env-exporter");

            let sources = ConfigSourceMap::detect_from(workspace.path());
            assert_eq!(sources.source_for("gateway.port"), "env ARGO_GATEWAY_PORT");
            assert!(sources.source_for("gateway.url").contains("user config"));
            assert!(sources
                .source_for("auth.signup_protection.ip_limit_per_hour")
                .contains("project config"));
            assert_eq!(
                sources.source_for("tenant.default_id"),
                "default",
                "unmentioned keys report built-in source"
            );
            assert!(secrets_display_allowed(true));
        },
    );
}

#[test]
fn explicit_override_merges_preserve_unmentioned_base_fields() {
    let mut base = ArgoConfig::from_toml_with_overrides(
        r#"
[gateway]
port = 1111
url = "http://base.example.test"

[telemetry]
exporter = "base-exporter"
enabled = true
"#,
    )
    .expect("base config");
    let overlay = ArgoConfig::from_toml_with_overrides(
        r#"
[gateway]
port = 2222

[telemetry]
enabled = false
"#,
    )
    .expect("overlay config");

    base.merge(overlay);
    assert_eq!(base.gateway.port, 2222);
    assert_eq!(base.gateway.url, "http://base.example.test");
    assert_eq!(base.telemetry.exporter, "base-exporter");
    assert!(!base.telemetry.enabled);
}

fn write_user_config(xdg: &Path, body: &str) {
    let path = xdg.join(defaults::USER_CONFIG_RELATIVE);
    fs::create_dir_all(path.parent().expect("user config parent")).expect("create user config dir");
    fs::write(path, body).expect("write user config");
}

fn with_scoped_env<F>(pairs: &[(&str, Option<&str>)], f: F)
where
    F: FnOnce(&TempDir, &TempDir),
{
    let _lock = ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let xdg = tempfile::tempdir().expect("xdg tempdir");
    let mut full_pairs = vec![
        ("HOME", None),
        ("XDG_CONFIG_HOME", xdg.path().to_str()),
        ("ARGO_GATEWAY_HOST", None),
        ("ARGO_GATEWAY_PORT", None),
        ("ARGO_GATEWAY_URL", None),
        ("ARGO_TELEMETRY_ENABLED", None),
        ("ARGO_TELEMETRY_EXPORTER", None),
        ("ARGO_TELEMETRY_ENDPOINT", None),
        ("ARGO_DEFAULT_TENANT", None),
        ("ARGO_SHOW_SECRETS", None),
    ];
    full_pairs.extend_from_slice(pairs);

    let originals: Vec<(String, Option<String>)> = full_pairs
        .iter()
        .map(|(key, _)| ((*key).to_string(), std::env::var(key).ok()))
        .collect();
    for (key, value) in &full_pairs {
        match value {
            Some(value) => std::env::set_var(key, value),
            None => std::env::remove_var(key),
        }
    }
    let _guard = ScopedEnvGuard { originals };
    f(&workspace, &xdg);
}

struct ScopedEnvGuard {
    originals: Vec<(String, Option<String>)>,
}

impl Drop for ScopedEnvGuard {
    fn drop(&mut self) {
        for (key, value) in &self.originals {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}
