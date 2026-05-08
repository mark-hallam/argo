//! Layered banner-config loader for `argo gateway start`.
//!
//! Precedence (highest → lowest):
//!
//!   1. CLI flag (`--quiet`)
//!   2. Env vars (`ARGO_BANNER_QUIET`, `ARGO_BANNER_ENABLED`,
//!      `ARGO_BANNER_HEADER`, `ARGO_BANNER_FOOTER`)
//!   3. `[startup_banner]` section of `argo.toml`
//!   4. Built-in `BannerConfig::default()` — byte-identical to the
//!      pre-config route catalog.
//!
//! Sibling of `start_banner.rs`: this module reads inputs and
//! produces a `BannerConfig`; `start_banner.rs` renders it. Reading
//! the TOML file directly (rather than extending `argo-config`) is
//! deliberate — banner config is gateway-scoped and the brief
//! explicitly forbids touching `argo-config` core.

use super::start_banner::{BannerConfig, BannerMode};
use serde::Deserialize;
use std::path::Path;

/// Wire form of the `[startup_banner]` TOML section. All fields
/// optional so a partial section produces a partial override.
#[derive(Debug, Default, Deserialize)]
struct StartupBannerToml {
    enabled: Option<bool>,
    header: Option<String>,
    footer: Option<String>,
    mode: Option<String>,
    #[serde(default)]
    extra_fields: Vec<ExtraFieldToml>,
}

#[derive(Debug, Deserialize)]
struct ExtraFieldToml {
    key: String,
    value: String,
}

#[derive(Debug, Default, Deserialize)]
struct TomlRoot {
    startup_banner: Option<StartupBannerToml>,
}

/// Read `[startup_banner]` from `argo.toml` (or any TOML doc with the
/// same shape). Missing file / parse error / missing section all
/// fall back to `BannerConfig::default()`.
#[must_use]
pub fn load_from_toml_file(path: &Path) -> BannerConfig {
    std::fs::read_to_string(path)
        .ok()
        .map(|s| load_from_toml_str(&s))
        .unwrap_or_default()
}

/// Same as `load_from_toml_file` but takes the raw TOML text. Useful
/// for tests and for callers that have already loaded the file.
#[must_use]
pub fn load_from_toml_str(toml_text: &str) -> BannerConfig {
    let parsed: TomlRoot = toml::from_str(toml_text).unwrap_or_default();
    parsed
        .startup_banner
        .map(toml_to_config)
        .unwrap_or_default()
}

fn toml_to_config(t: StartupBannerToml) -> BannerConfig {
    let mut cfg = BannerConfig::default();
    if let Some(enabled) = t.enabled {
        cfg.enabled = enabled;
    }
    if let Some(mode) = t.mode {
        cfg.mode = parse_mode(&mode);
    }
    if t.header.is_some() {
        cfg.header = t.header;
    }
    if t.footer.is_some() {
        cfg.footer = t.footer;
    }
    if !t.extra_fields.is_empty() {
        cfg.extra_fields = t
            .extra_fields
            .into_iter()
            .map(|f| (f.key, f.value))
            .collect();
    }
    if header_or_footer_set(&cfg) || !cfg.extra_fields.is_empty() {
        cfg.mode = upgrade_to_custom(cfg.mode);
    }
    cfg
}

fn header_or_footer_set(cfg: &BannerConfig) -> bool {
    cfg.header.is_some() || cfg.footer.is_some()
}

fn upgrade_to_custom(mode: BannerMode) -> BannerMode {
    match mode {
        BannerMode::Default => BannerMode::Custom,
        other => other,
    }
}

fn parse_mode(s: &str) -> BannerMode {
    match s.to_ascii_lowercase().as_str() {
        "quiet" | "off" | "disabled" => BannerMode::Quiet,
        "custom" => BannerMode::Custom,
        _ => BannerMode::Default,
    }
}

/// Read banner config from the process environment via a caller-
/// supplied getter. The getter form keeps tests isolated from the
/// real process env.
#[must_use]
pub fn load_from_env_with<F>(getter: F) -> BannerConfig
where
    F: Fn(&str) -> Option<String>,
{
    let mut cfg = BannerConfig::default();
    if let Some(quiet) = getter("ARGO_BANNER_QUIET") {
        if env_truthy(&quiet) {
            cfg.enabled = false;
            cfg.mode = BannerMode::Quiet;
            return cfg;
        }
    }
    if let Some(enabled) = getter("ARGO_BANNER_ENABLED") {
        cfg.enabled = env_truthy(&enabled);
        if !cfg.enabled {
            cfg.mode = BannerMode::Quiet;
            return cfg;
        }
    }
    let header = getter("ARGO_BANNER_HEADER").filter(|s| !s.is_empty());
    let footer = getter("ARGO_BANNER_FOOTER").filter(|s| !s.is_empty());
    if header.is_some() || footer.is_some() {
        cfg.header = header;
        cfg.footer = footer;
        cfg.mode = upgrade_to_custom(cfg.mode);
    }
    cfg
}

/// Production env-var loader — reads the real process environment.
#[must_use]
pub fn load_from_env() -> BannerConfig {
    load_from_env_with(|k| std::env::var(k).ok())
}

fn env_truthy(s: &str) -> bool {
    matches!(
        s.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

/// Right-biased merge: any non-default field on `over` overrides
/// `base`. `Quiet` mode and `enabled=false` always propagate so the
/// override-wins precedence is consistent.
#[must_use]
pub fn merge(base: BannerConfig, over: BannerConfig) -> BannerConfig {
    let default = BannerConfig::default();
    let mut out = base;
    if !over.enabled {
        out.enabled = false;
        out.mode = BannerMode::Quiet;
        return out;
    }
    if over.mode != default.mode {
        out.mode = over.mode;
    }
    if over.header.is_some() {
        out.header = over.header;
    }
    if over.footer.is_some() {
        out.footer = over.footer;
    }
    if !over.extra_fields.is_empty() {
        out.extra_fields = over.extra_fields;
    }
    if (out.header.is_some() || out.footer.is_some() || !out.extra_fields.is_empty())
        && out.mode == BannerMode::Default
    {
        out.mode = BannerMode::Custom;
    }
    out
}

/// Resolve the effective banner config from all four layers.
///
/// `argo_toml_path`: optional path to `argo.toml`. When absent the
/// TOML layer is skipped. `cli_quiet`: top-precedence override
/// matching the `--quiet` CLI flag.
#[must_use]
pub fn load_layered(argo_toml_path: Option<&Path>, cli_quiet: bool) -> BannerConfig {
    let mut cfg = BannerConfig::default();
    if let Some(p) = argo_toml_path {
        cfg = merge(cfg, load_from_toml_file(p));
    }
    cfg = merge(cfg, load_from_env());
    if cli_quiet {
        cfg.enabled = false;
        cfg.mode = BannerMode::Quiet;
    }
    cfg
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_getter(pairs: &[(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        move |k: &str| map.get(k).cloned()
    }

    #[test]
    fn env_quiet_yields_quiet_mode() {
        // Acceptance #2: ARGO_BANNER_QUIET=1 → suppressed banner.
        let cfg = load_from_env_with(make_getter(&[("ARGO_BANNER_QUIET", "1")]));
        assert!(!cfg.enabled);
        assert_eq!(cfg.mode, BannerMode::Quiet);
    }

    #[test]
    fn env_quiet_accepts_truthy_aliases() {
        for v in ["1", "true", "YES", "on", "True"] {
            let cfg = load_from_env_with(make_getter(&[("ARGO_BANNER_QUIET", v)]));
            assert_eq!(cfg.mode, BannerMode::Quiet, "value {v}");
        }
        // Falsy values do not flip to quiet.
        let cfg = load_from_env_with(make_getter(&[("ARGO_BANNER_QUIET", "0")]));
        assert_eq!(cfg.mode, BannerMode::Default);
    }

    #[test]
    fn env_custom_header_promotes_to_custom_mode() {
        // Acceptance #3: custom header via env var reflected in cfg.
        let cfg = load_from_env_with(make_getter(&[(
            "ARGO_BANNER_HEADER",
            "=== prod / us-east-1 ===",
        )]));
        assert_eq!(cfg.mode, BannerMode::Custom);
        assert_eq!(cfg.header.as_deref(), Some("=== prod / us-east-1 ==="));
        assert!(cfg.enabled);
    }

    #[test]
    fn empty_env_yields_default_config() {
        let cfg = load_from_env_with(make_getter(&[]));
        assert_eq!(cfg, BannerConfig::default());
    }

    #[test]
    fn toml_section_drives_custom_render() {
        // Acceptance #4: [startup_banner] TOML overrides individual fields.
        let toml_text = r#"
[startup_banner]
header = "deploy=staging"
footer = "build=feedface"
[[startup_banner.extra_fields]]
key = "region"
value = "eu-west-2"
"#;
        let cfg = load_from_toml_str(toml_text);
        assert_eq!(cfg.mode, BannerMode::Custom);
        assert_eq!(cfg.header.as_deref(), Some("deploy=staging"));
        assert_eq!(cfg.footer.as_deref(), Some("build=feedface"));
        assert_eq!(
            cfg.extra_fields,
            vec![("region".to_string(), "eu-west-2".to_string())]
        );
    }

    #[test]
    fn toml_quiet_mode_round_trips() {
        let cfg = load_from_toml_str("[startup_banner]\nmode = \"quiet\"\n");
        assert_eq!(cfg.mode, BannerMode::Quiet);
    }

    #[test]
    fn toml_missing_section_is_default() {
        assert_eq!(load_from_toml_str(""), BannerConfig::default());
        assert_eq!(load_from_toml_str("[other]\n"), BannerConfig::default());
    }

    #[test]
    fn toml_malformed_falls_back_to_default() {
        // Brief safety: bad TOML must not panic the gateway.
        assert_eq!(
            load_from_toml_str("not valid toml ===="),
            BannerConfig::default()
        );
    }

    #[test]
    fn merge_precedence_env_over_toml() {
        let toml_layer = load_from_toml_str(
            "[startup_banner]\nheader = \"toml-header\"\nfooter = \"toml-footer\"\n",
        );
        let env_layer = load_from_env_with(make_getter(&[("ARGO_BANNER_HEADER", "env-header")]));
        let merged = merge(toml_layer, env_layer);
        assert_eq!(merged.header.as_deref(), Some("env-header"));
        // Footer was only in TOML; survives.
        assert_eq!(merged.footer.as_deref(), Some("toml-footer"));
        assert_eq!(merged.mode, BannerMode::Custom);
    }

    #[test]
    fn merge_disabled_propagates_to_quiet() {
        let base = BannerConfig {
            mode: BannerMode::Custom,
            header: Some("kept".into()),
            ..BannerConfig::default()
        };
        let over = BannerConfig {
            enabled: false,
            ..BannerConfig::default()
        };
        let merged = merge(base, over);
        assert!(!merged.enabled);
        assert_eq!(merged.mode, BannerMode::Quiet);
    }

    #[test]
    fn cli_quiet_flag_overrides_env_and_toml() {
        // Acceptance #5: --quiet overrides env (and TOML).
        let dir = tempfile::tempdir().expect("tempdir");
        let toml_path = dir.path().join("argo.toml");
        std::fs::write(
            &toml_path,
            "[startup_banner]\nheader = \"toml-keeps-trying\"\n",
        )
        .expect("write toml");
        // load_layered uses real env via load_from_env(); since there
        // is no env-var injection seam at load_layered, this test
        // only asserts the CLI flag wins over everything else by
        // ignoring whatever env state happens to be set. It's safe
        // because the cli_quiet=true branch unconditionally forces
        // Quiet mode at the end of load_layered.
        let cfg = load_layered(Some(&toml_path), true);
        assert!(!cfg.enabled);
        assert_eq!(cfg.mode, BannerMode::Quiet);
    }
}
