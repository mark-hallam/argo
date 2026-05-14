//! `argo provider breakers` — per-provider circuit-breaker inspection.
//!
//! Lists every provider for which a manifest exists under
//! `manifests/providers/` plus its breaker knobs and current state.
//! Honours the optional `[provider.<id>.breaker]` block in `argo.toml`
//! (failure_threshold + half_open_probe_interval_ms; missing →
//! `BreakerConfig::default()`).
//!
//! Breakers are constructed fresh on every CLI invocation, so the
//! reported `state` is always `Closed` and `last_transition_at` is
//! always `null`. This command's value sits in surfacing the
//! *configured* knobs per provider and confirming that the wiring
//! (manifest → config block → `Breaker`) parses without errors —
//! sharing live runtime state across argo processes is out of scope
//! for this mission (in-memory only, per the brief's `## Scope`).
//!
//! Behaviour when a provider isn't configured: omitted silently. A
//! missing manifest means there's nothing to inspect; the operator
//! sees only the providers they've actually defined.

use argo_config::{BreakerSection, ProvidersConfig};
use argo_provider::{Breaker, BreakerConfig, CircuitState, ProviderManifest};
use argo_types::{ArgoError, ArgoResult};
use std::path::Path;
use std::time::Duration;

/// Run `argo provider breakers [--json]`. Text output is a fixed-width
/// table; JSON output is a versioned envelope.
pub fn run(json: bool) -> ArgoResult<()> {
    let cwd = std::env::current_dir()
        .map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let providers_dir = cwd.join("manifests").join("providers");
    let providers_config = read_providers_config(&cwd);
    let manifest_ids = read_manifest_ids(&providers_dir)?;

    let rows: Vec<BreakerRow> = manifest_ids
        .iter()
        .map(|id| compute_row(id, &providers_config))
        .collect();

    if json {
        print_json(&rows);
    } else {
        print_text(&rows, &providers_dir);
    }
    Ok(())
}

/// One row of the breakers report. Public for the test below.
#[derive(Debug, Clone)]
pub(crate) struct BreakerRow {
    pub provider_id: String,
    pub failure_threshold: u32,
    pub half_open_probe_interval_ms: u64,
    pub state: CircuitState,
    /// True when the operator explicitly set `[provider.<id>.breaker]`
    /// in `argo.toml`; false when fields fell back to defaults.
    pub configured: bool,
}

pub(crate) fn compute_row(provider_id: &str, providers_config: &ProvidersConfig) -> BreakerRow {
    let entry = providers_config.entry_or_default(provider_id);
    let breaker_section = entry.breaker_or_default();
    let configured = entry.breaker.is_some();
    let cfg = section_to_breaker_config(&breaker_section);
    let breaker = Breaker::new(cfg);
    BreakerRow {
        provider_id: provider_id.to_string(),
        failure_threshold: breaker_section.resolved_failure_threshold(),
        half_open_probe_interval_ms: breaker_section
            .resolved_half_open_probe_interval()
            .as_millis() as u64,
        state: breaker.state_of(provider_id),
        configured,
    }
}

fn section_to_breaker_config(s: &BreakerSection) -> BreakerConfig {
    let cooldown = s.resolved_half_open_probe_interval();
    let default_max = Duration::from_secs(300);
    BreakerConfig {
        failure_threshold: s.resolved_failure_threshold(),
        cooldown,
        cooldown_max: cooldown.max(default_max),
    }
}

fn read_providers_config(cwd: &Path) -> ProvidersConfig {
    let argo_toml = cwd.join("argo.toml");
    match std::fs::read_to_string(&argo_toml) {
        Ok(body) => ProvidersConfig::parse_from_argo_toml(&body).unwrap_or_default(),
        Err(_) => ProvidersConfig::default(),
    }
}

fn read_manifest_ids(dir: &Path) -> ArgoResult<Vec<String>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut ids = Vec::new();
    for entry in std::fs::read_dir(dir)
        .map_err(|e| ArgoError::Io(format!("read_dir {}: {e}", dir.display())))?
    {
        let e = entry.map_err(|e| ArgoError::Provider(format!("dir entry: {e}")))?;
        let p = e.path();
        if p.extension().is_none_or(|x| x != "toml") {
            continue;
        }
        let body = std::fs::read_to_string(&p)
            .map_err(|e| ArgoError::Io(format!("read {}: {e}", p.display())))?;
        if let Ok(m) = ProviderManifest::from_toml_str(&body) {
            ids.push(m.provider.id);
        }
    }
    ids.sort();
    Ok(ids)
}

pub(crate) fn rows_to_json(rows: &[BreakerRow]) -> serde_json::Value {
    let arr: Vec<serde_json::Value> = rows.iter().map(row_to_json).collect();
    serde_json::json!({
        "version": 1,
        "providers": arr,
    })
}

fn print_json(rows: &[BreakerRow]) {
    let envelope = rows_to_json(rows);
    let body = serde_json::to_string(&envelope).unwrap_or_else(|_| "{}".to_string());
    println!("{body}");
}

fn row_to_json(r: &BreakerRow) -> serde_json::Value {
    serde_json::json!({
        "provider_id": r.provider_id,
        "failure_threshold": r.failure_threshold,
        "half_open_probe_interval_ms": r.half_open_probe_interval_ms,
        "state": state_to_str(r.state),
        "last_transition_at": serde_json::Value::Null,
        "configured": r.configured,
    })
}

fn state_to_str(s: CircuitState) -> &'static str {
    match s {
        CircuitState::Closed => "Closed",
        CircuitState::Open => "Open",
        CircuitState::HalfOpen => "HalfOpen",
    }
}

fn print_text(rows: &[BreakerRow], providers_dir: &Path) {
    if rows.is_empty() {
        println!(
            "(no provider manifests found under {})",
            providers_dir.display()
        );
        return;
    }
    println!(
        "{:<24} {:>12} {:>14} {:>10} {:<10}",
        "PROVIDER", "FAIL_THRESH", "PROBE_MS", "STATE", "CONFIG"
    );
    for r in rows {
        println!(
            "{:<24} {:>12} {:>14} {:>10} {:<10}",
            r.provider_id,
            r.failure_threshold,
            r.half_open_probe_interval_ms,
            state_to_str(r.state),
            if r.configured { "custom" } else { "default" },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_uses_defaults_when_no_provider_block_configured() {
        let cfg = ProvidersConfig::default();
        let row = compute_row("anthropic", &cfg);
        assert_eq!(row.provider_id, "anthropic");
        assert_eq!(
            row.failure_threshold,
            argo_config::provider::DEFAULT_FAILURE_THRESHOLD
        );
        assert_eq!(
            row.half_open_probe_interval_ms,
            argo_config::provider::DEFAULT_HALF_OPEN_PROBE_INTERVAL_MS
        );
        assert_eq!(row.state, CircuitState::Closed);
        assert!(!row.configured);
    }

    #[test]
    fn row_uses_custom_overrides_from_argo_toml() {
        let body = r#"
[provider.anthropic.breaker]
failure_threshold = 3
half_open_probe_interval_ms = 5000
"#;
        let cfg = ProvidersConfig::parse_from_argo_toml(body).expect("parse ok");
        let row = compute_row("anthropic", &cfg);
        assert_eq!(row.failure_threshold, 3);
        assert_eq!(row.half_open_probe_interval_ms, 5000);
        assert!(row.configured);
        // Fresh breaker still reports Closed.
        assert_eq!(row.state, CircuitState::Closed);
    }

    #[test]
    fn json_envelope_is_versioned_and_arrays_per_provider() {
        let rows = vec![
            BreakerRow {
                provider_id: "anthropic".into(),
                failure_threshold: 5,
                half_open_probe_interval_ms: 30_000,
                state: CircuitState::Closed,
                configured: false,
            },
            BreakerRow {
                provider_id: "openai".into(),
                failure_threshold: 3,
                half_open_probe_interval_ms: 1_500,
                state: CircuitState::Closed,
                configured: true,
            },
        ];
        let env = rows_to_json(&rows);
        assert_eq!(env["version"], 1);
        let providers = env["providers"].as_array().expect("providers array");
        assert_eq!(providers.len(), 2);
        assert_eq!(providers[0]["provider_id"], "anthropic");
        assert_eq!(providers[0]["state"], "Closed");
        assert_eq!(providers[0]["last_transition_at"], serde_json::Value::Null);
        assert_eq!(providers[0]["configured"], false);
        assert_eq!(providers[1]["provider_id"], "openai");
        assert_eq!(providers[1]["failure_threshold"], 3);
        assert_eq!(providers[1]["configured"], true);
    }
}
