use super::*;
use argo_core::RestartPolicy;
use std::path::Path;

const FULL_CONFIG: &str = r#"
[core]
home = "~/.argo"
workspace = "."
profile = "prod"

[budget]
default_iterations = 80
default_spend_per_session_usd_micros = 2_000_000
default_spend_per_day_usd_micros    = 20_000_000
default_spend_per_month_usd_micros  = 200_000_000
notify_at_pct = [70, 90]
on_max_iterations = "auto_continue"

[loop]
on_restart = "auto_resume"

[routing]
policy = "tiered"
[routing.tiered]
classifier = "openai-compatible:gpt-4o-mini"
simple = "openai-compatible:gpt-4o-mini"
medium = "anthropic:claude-haiku-4-5"
complex = "anthropic:claude-opus-4-6"
classify_threshold_ms = 60
"#;

#[test]
fn parses_full_config() {
    let cfg: ArgoConfig = toml::from_str(FULL_CONFIG).unwrap();
    assert_eq!(cfg.core.profile, "prod");
    assert_eq!(cfg.budget.default_iterations, 80);
    assert_eq!(cfg.budget.notify_at_pct, vec![70, 90]);
    assert_eq!(cfg.loop_.on_restart, "auto_resume");
    assert_eq!(cfg.routing.policy, "tiered");
    let tiered = cfg.routing.tiered.unwrap();
    assert_eq!(tiered.classify_threshold_ms, 60);
}

#[test]
fn defaults_when_file_missing() {
    let p = Path::new("/tmp/argo-config-does-not-exist-xyz.toml");
    let cfg = ArgoConfig::load(p).unwrap();
    // The shipped defaults populate $1/$10/$100 + [80, 95].
    assert_eq!(cfg.budget.default_spend_per_session_usd_micros, 1_000_000);
    assert_eq!(cfg.budget.notify_at_pct, vec![80, 95]);
    assert_eq!(cfg.loop_.on_restart, "prompt_user");
    assert_eq!(cfg.core.profile, "default");
}

#[test]
fn empty_toml_falls_back_to_defaults() {
    let cfg: ArgoConfig = toml::from_str("").unwrap();
    assert_eq!(cfg.tenant.mode, "single");
    assert!(cfg.audit.enabled);
    assert_eq!(cfg.parallel.max_parallel_tools, 8);
    assert!(cfg.delegate.fallback_chain_inheritance);
}

#[test]
fn budget_block_binds_to_cost_ceilings() {
    let cfg: ArgoConfig = toml::from_str(FULL_CONFIG).unwrap();
    let ceilings = cfg.budget.to_cost_ceilings();
    assert_eq!(ceilings.per_session_usd_micros, Some(2_000_000));
    assert_eq!(ceilings.per_day_usd_micros, Some(20_000_000));
    assert_eq!(ceilings.per_month_usd_micros, Some(200_000_000));
    assert_eq!(ceilings.notify_at_pct, vec![70, 90]);
}

#[test]
fn budget_zero_disables_per_scope_cap() {
    // Spec convention: 0 means "unset" for the optional spend caps.
    let toml_src = r#"
[budget]
default_spend_per_session_usd_micros = 0
default_spend_per_day_usd_micros = 5_000_000
default_spend_per_month_usd_micros = 0
"#;
    let cfg: ArgoConfig = toml::from_str(toml_src).unwrap();
    let ceilings = cfg.budget.to_cost_ceilings();
    assert_eq!(ceilings.per_session_usd_micros, None);
    assert_eq!(ceilings.per_day_usd_micros, Some(5_000_000));
    assert_eq!(ceilings.per_month_usd_micros, None);
}

#[test]
fn loop_block_binds_to_restart_policy() {
    for (raw, expected) in [
        ("prompt_user", RestartPolicy::PromptUser),
        ("auto_resume", RestartPolicy::AutoResume),
        ("discard", RestartPolicy::Discard),
        ("garbage", RestartPolicy::PromptUser),
    ] {
        let cfg: ArgoConfig = toml::from_str(&format!("[loop]\non_restart = \"{raw}\"")).unwrap();
        assert_eq!(cfg.loop_.restart_policy(), expected);
    }
}

#[test]
fn tilde_expands_to_home() {
    let dir = tempfile::tempdir().unwrap();
    let prev = std::env::var_os("HOME");
    std::env::set_var("HOME", dir.path());
    let cfg = ArgoConfig::default();
    let resolved = cfg.core.resolved_home();
    assert!(resolved.starts_with(dir.path()));
    if let Some(p) = prev {
        std::env::set_var("HOME", p);
    } else {
        std::env::remove_var("HOME");
    }
}

#[test]
fn load_round_trips_through_disk() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("argo.toml");
    std::fs::write(&p, FULL_CONFIG).unwrap();
    let cfg = ArgoConfig::load(&p).unwrap();
    assert_eq!(cfg.budget.default_iterations, 80);
}

#[test]
fn load_surfaces_parse_errors() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("argo.toml");
    std::fs::write(&p, "[budget]\ndefault_iterations = \"not-a-number\"").unwrap();
    let err = ArgoConfig::load(&p).unwrap_err();
    matches!(err, argo_types::ArgoError::Provider(_));
}
