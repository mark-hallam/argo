//! `argo doctor throttles` — list per-action-type throttle buckets
//! and operator-configured policies
//! (`mission-per-action-type-throttling`).
//!
//! The runtime owns the `ThrottleRegistry`; this handler is the
//! operator-facing inspection surface. It reports two things:
//!
//! 1. The configured `[runtime.action_throttle]` policy map loaded
//!    from `argo.toml` (with `enabled = false` surfacing prominently
//!    so operators can verify the wire is off before tuning).
//! 2. Any live bucket state — currently empty when called from the
//!    CLI because the dispatch-side wiring is a follow-up; a future
//!    revision will read the registry off the running gateway via a
//!    `/throttles` endpoint or the embedded supervisor.
//!
//! The JSON output is the contract; the text form is operator
//! convenience and may render summarised counts that drop detail.
//!
//! Acceptance criteria from
//! `streams/mission-per-action-type-throttling.md` row 6: `argo
//! doctor throttles` lists current state per bucket.

use crate::home::resolve_home;
use argo_config::{ActionThrottleConfig, ArgoConfig};
use argo_runtime::action_throttle::{ThrottleConfig, ThrottlePolicy, ThrottleRegistry};
use argo_types::{ArgoError, ArgoResult};
use chrono::Utc;
use serde_json::{json, Value};
use std::time::Duration;

/// `argo doctor throttles` entry point. Reads the workspace config,
/// builds a [`ThrottleRegistry`] from the operator policies, and
/// emits either a human-readable text summary or a JSON object the
/// monitoring pipeline can ingest.
pub async fn run(json_out: bool) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let toml_path = cwd.join("argo.toml");
    let cfg = ArgoConfig::load(&toml_path).unwrap_or_default();
    let _home = resolve_home(&cwd).ok();
    let payload = collect(&cfg.runtime.action_throttle);
    if json_out {
        println!(
            "{}",
            serde_json::to_string(&payload)
                .map_err(|e| ArgoError::Provider(format!("serialize: {e}")))?
        );
        return Ok(());
    }
    print_text(&payload);
    Ok(())
}

/// Build the doctor-throttles payload from the loaded config. Pure
/// over config + `Utc::now()` so the JSON shape is stable across
/// callers (CLI today, gateway endpoint tomorrow).
#[must_use]
pub fn collect(cfg: &ActionThrottleConfig) -> Value {
    let runtime_cfg = build_runtime_config(cfg);
    let registry = ThrottleRegistry::new(runtime_cfg);
    let snapshot = registry.snapshot(Utc::now());
    let policies: Vec<Value> = {
        let mut entries: Vec<(&String, &argo_config::ActionThrottlePolicyConfig)> =
            cfg.policies.iter().collect();
        entries.sort_by(|a, b| a.0.cmp(b.0));
        entries
            .into_iter()
            .map(|(name, p)| {
                json!({
                    "action_type": name,
                    "tokens_per_sec": p.tokens_per_sec,
                    "burst": p.burst,
                    "valid": ThrottlePolicy::new(p.tokens_per_sec, p.burst).is_some(),
                })
            })
            .collect()
    };
    let buckets: Vec<Value> = snapshot
        .into_iter()
        .map(|b| {
            json!({
                "tenant_id": b.tenant_id,
                "action_type": b.action_type,
                "tokens_available": b.tokens_available,
                "burst": b.burst,
                "tokens_per_sec": b.tokens_per_sec,
                "last_refill": b.last_refill.to_rfc3339(),
            })
        })
        .collect();
    json!({
        "enabled": cfg.enabled,
        "audit_threshold_secs": cfg.audit_threshold_secs,
        "policies": policies,
        "buckets": buckets,
    })
}

fn build_runtime_config(cfg: &ActionThrottleConfig) -> ThrottleConfig {
    let mut policies = std::collections::HashMap::new();
    for (name, p) in &cfg.policies {
        if let Some(policy) = ThrottlePolicy::new(p.tokens_per_sec, p.burst) {
            policies.insert(name.clone(), policy);
        }
    }
    ThrottleConfig {
        enabled: cfg.enabled,
        policies,
        audit_threshold: Duration::from_secs(u64::from(cfg.audit_threshold_secs)),
    }
}

fn print_text(payload: &Value) {
    let enabled = payload["enabled"].as_bool().unwrap_or(false);
    let threshold = payload["audit_threshold_secs"].as_u64().unwrap_or(0);
    let status = if enabled { "ENABLED" } else { "DISABLED" };
    println!("argo doctor throttles — per-action-type throttle gate");
    println!("  status:            {status}");
    println!("  audit threshold:   {threshold}s (runtime.action_throttled when wait ≥)");
    let policies = payload["policies"].as_array().cloned().unwrap_or_default();
    if policies.is_empty() {
        println!("  policies:          (none configured)");
    } else {
        println!("  policies ({}):", policies.len());
        for p in &policies {
            let action = p["action_type"].as_str().unwrap_or("?");
            let tps = p["tokens_per_sec"].as_f64().unwrap_or(0.0);
            let burst = p["burst"].as_u64().unwrap_or(0);
            let valid = p["valid"].as_bool().unwrap_or(false);
            let tag = if valid { "" } else { "  [INVALID — tokens_per_sec or burst is zero / non-finite]" };
            println!("    - {action}: {tps} tok/s, burst {burst}{tag}");
        }
    }
    let buckets = payload["buckets"].as_array().cloned().unwrap_or_default();
    if buckets.is_empty() {
        println!("  buckets:           (no live state — registry inspection is CLI-snapshot only in v1)");
    } else {
        println!("  buckets ({}):", buckets.len());
        for b in &buckets {
            let tenant = b["tenant_id"].as_str().unwrap_or("?");
            let action = b["action_type"].as_str().unwrap_or("?");
            let available = b["tokens_available"].as_f64().unwrap_or(0.0);
            let burst = b["burst"].as_u64().unwrap_or(0);
            println!(
                "    - tenant={tenant}  action={action}  tokens={available:.2}/{burst}"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_config::ActionThrottlePolicyConfig;

    fn cfg_disabled() -> ActionThrottleConfig {
        ActionThrottleConfig::default()
    }

    fn cfg_with(action: &str, tokens_per_sec: f32, burst: u32) -> ActionThrottleConfig {
        let mut policies = std::collections::HashMap::new();
        policies.insert(
            action.to_string(),
            ActionThrottlePolicyConfig {
                tokens_per_sec,
                burst,
            },
        );
        ActionThrottleConfig {
            enabled: true,
            audit_threshold_secs: 10,
            policies,
        }
    }

    #[test]
    fn disabled_payload_reports_disabled() {
        let payload = collect(&cfg_disabled());
        assert_eq!(payload["enabled"], false);
        assert!(
            payload["policies"].as_array().unwrap().is_empty(),
            "default config has no policies"
        );
        assert!(payload["buckets"].as_array().unwrap().is_empty());
    }

    #[test]
    fn enabled_payload_surfaces_each_policy() {
        let payload = collect(&cfg_with("send_email", 5.0, 10));
        assert_eq!(payload["enabled"], true);
        assert_eq!(payload["audit_threshold_secs"], 10);
        let policies = payload["policies"].as_array().expect("policies array");
        assert_eq!(policies.len(), 1);
        assert_eq!(policies[0]["action_type"], "send_email");
        assert_eq!(policies[0]["burst"], 10);
        assert_eq!(policies[0]["valid"], true);
    }

    #[test]
    fn invalid_policy_surfaces_invalid_flag() {
        // tokens_per_sec=0 → rejected at registry construction; the
        // operator-facing JSON still surfaces the entry tagged so
        // the operator can spot it.
        let payload = collect(&cfg_with("bad_action", 0.0, 5));
        let policies = payload["policies"].as_array().expect("policies array");
        assert_eq!(policies[0]["valid"], false);
    }
}
