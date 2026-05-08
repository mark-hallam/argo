//! Configurable alerts (`.skycot/alerts.toml`) with a 60s evaluator.
//!
//! Operators currently tail logs or run `argo doctor` to spot trouble.
//! This module adds a declarative alerts file that defines threshold
//! rules (spend, error rate, approval timeout) and routes firings to
//! channels (file log, gateway event). A background task ticks every
//! 60s, builds a metrics snapshot, evaluates each rule, and writes
//! firings to the configured channels. Hysteresis prevents repeat
//! firing: a rule re-arms only after its metric falls back below
//! threshold.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration as StdDuration;

use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use serde::{Deserialize, Serialize};

const DEFAULT_TICK_SECS: u64 = 60;

/// Root config: `[[rule]]` array in `.skycot/alerts.toml`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AlertsConfig {
    #[serde(default, rename = "rule")]
    pub rules: Vec<AlertRule>,
}

/// One rule: a name, a kind-specific threshold/window block, and
/// a list of channels to route firings to.
#[derive(Debug, Clone, Deserialize)]
pub struct AlertRule {
    pub name: String,
    #[serde(flatten)]
    pub kind: AlertKind,
    #[serde(default)]
    pub channels: Vec<String>,
}

/// The three rule kinds. Discriminated by the TOML `kind` field.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AlertKind {
    /// Total spend over the window crossed `threshold_usd`.
    SpendTotal {
        threshold_usd: f64,
        window: String,
    },
    /// Error rate on `route` over the window crossed `threshold_pct`.
    ErrorRate {
        route: String,
        threshold_pct: f64,
        window: String,
    },
    /// Oldest pending approval older than `older_than` (e.g. "30m").
    ApprovalTimeout { older_than: String },
}

/// One firing of one rule at one tick. Serialized to file/event channels.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct AlertFiring {
    pub rule: String,
    pub kind: String,
    pub observed: f64,
    pub threshold: f64,
    pub fired_at_micros: i64,
    pub message: String,
}

/// Snapshot of the metrics needed by [`evaluate_once`]. The loop
/// builds this from `SessionStore`; tests construct it directly so
/// the evaluator stays pure.
#[derive(Debug, Clone, Default)]
pub struct AlertSnapshot {
    pub now_micros: i64,
    /// Spend (USD) for each spend rule, keyed by rule name.
    pub spend_usd_by_rule: HashMap<String, f64>,
    /// Error rate (percent) for each error-rate rule, keyed by name.
    pub error_pct_by_rule: HashMap<String, f64>,
    /// Age (seconds) of the oldest pending approval, or 0 if none.
    pub oldest_pending_approval_age_secs: u64,
}

/// Evaluate every rule against the snapshot. `fired` carries
/// hysteresis state across calls — names of rules that fired and have
/// not yet re-armed. Returns only the rules that fire on this tick
/// (i.e. crossed the threshold and were not already firing).
pub fn evaluate_once(
    cfg: &AlertsConfig,
    snapshot: &AlertSnapshot,
    fired: &mut HashSet<String>,
) -> Vec<AlertFiring> {
    let mut firings = Vec::new();
    let mut still_above: HashSet<String> = HashSet::new();
    for rule in &cfg.rules {
        let (observed, threshold, kind_label, message) = describe_rule(rule, snapshot);
        let above = threshold > 0.0 && observed >= threshold;
        if above {
            still_above.insert(rule.name.clone());
            if !fired.contains(&rule.name) {
                firings.push(AlertFiring {
                    rule: rule.name.clone(),
                    kind: kind_label.into(),
                    observed,
                    threshold,
                    fired_at_micros: snapshot.now_micros,
                    message,
                });
            }
        }
    }
    fired.retain(|n| still_above.contains(n));
    for f in &firings {
        fired.insert(f.rule.clone());
    }
    firings
}

fn describe_rule(rule: &AlertRule, snapshot: &AlertSnapshot) -> (f64, f64, &'static str, String) {
    match &rule.kind {
        AlertKind::SpendTotal {
            threshold_usd,
            window,
        } => {
            let observed = *snapshot.spend_usd_by_rule.get(&rule.name).unwrap_or(&0.0);
            let msg = format!(
                "spend over {window} reached ${observed:.2} (threshold ${threshold_usd:.2})"
            );
            (observed, *threshold_usd, "spend_total", msg)
        }
        AlertKind::ErrorRate {
            route,
            threshold_pct,
            window,
        } => {
            let observed = *snapshot.error_pct_by_rule.get(&rule.name).unwrap_or(&0.0);
            let msg = format!(
                "error rate on {route} over {window} reached {observed:.2}% (threshold {threshold_pct:.2}%)"
            );
            (observed, *threshold_pct, "error_rate", msg)
        }
        AlertKind::ApprovalTimeout { older_than } => {
            let observed = snapshot.oldest_pending_approval_age_secs as f64;
            let threshold = parse_window_secs(older_than).unwrap_or(0) as f64;
            let msg = format!(
                "oldest pending approval is {} seconds old (older_than {older_than})",
                observed as u64
            );
            (observed, threshold, "approval_timeout", msg)
        }
    }
}

/// Parse a duration string like "24h", "10m", "30s", "1d" into seconds.
/// Returns `None` for malformed input.
pub fn parse_window_secs(s: &str) -> Option<u64> {
    let s = s.trim();
    if s.len() < 2 || !s.is_ascii() {
        return None;
    }
    let (num_part, unit) = s.split_at(s.len() - 1);
    let n: u64 = num_part.parse().ok()?;
    match unit {
        "s" => Some(n),
        "m" => Some(n * 60),
        "h" => Some(n * 3600),
        "d" => Some(n * 86400),
        _ => None,
    }
}

/// Read `.skycot/alerts.toml` (or any path). Absent file is not an
/// error — returns an empty config so callers can spawn the loop
/// unconditionally.
pub fn load_alerts_config(path: &Path) -> ArgoResult<AlertsConfig> {
    if !path.exists() {
        return Ok(AlertsConfig::default());
    }
    let body = std::fs::read_to_string(path)
        .map_err(|e| ArgoError::Provider(format!("read {}: {e}", path.display())))?;
    let cfg: AlertsConfig = toml::from_str(&body)
        .map_err(|e| ArgoError::Provider(format!("parse {}: {e}", path.display())))?;
    Ok(cfg)
}

/// Background task: tick every 60s, build snapshot, evaluate, route
/// firings. No-op when `cfg.rules` is empty so callers can spawn it
/// without checking.
pub async fn run_alerts_loop(store: Arc<SessionStore>, cfg: AlertsConfig) {
    if cfg.rules.is_empty() {
        return;
    }
    let mut fired: HashSet<String> = HashSet::new();
    let mut interval = tokio::time::interval(StdDuration::from_secs(DEFAULT_TICK_SECS));
    loop {
        interval.tick().await;
        let snapshot = build_snapshot(&store, &cfg).await;
        for firing in evaluate_once(&cfg, &snapshot, &mut fired) {
            route_firing(&cfg, &firing);
        }
    }
}

async fn build_snapshot(store: &SessionStore, cfg: &AlertsConfig) -> AlertSnapshot {
    let now_micros = chrono::Utc::now().timestamp_micros();
    let mut snapshot = AlertSnapshot {
        now_micros,
        ..Default::default()
    };
    let global = store.load_global_spend().await.ok();
    for rule in &cfg.rules {
        if let AlertKind::SpendTotal { window, .. } = &rule.kind {
            if let Some(g) = &global {
                let usd = pick_spend_window(g, window);
                snapshot.spend_usd_by_rule.insert(rule.name.clone(), usd);
            }
        }
    }
    snapshot
}

fn pick_spend_window(g: &argo_store::GlobalSpend, window: &str) -> f64 {
    let secs = parse_window_secs(window).unwrap_or(86_400);
    // The store exposes today/this_month/all_time only — pick the
    // closest bucket. Sub-day windows still use the daily total since
    // a finer bucket isn't available; documented in the PR notes.
    let micros = if secs >= 30 * 86_400 {
        g.this_month
    } else {
        g.today
    };
    (micros as f64) / 1_000_000.0
}

fn route_firing(cfg: &AlertsConfig, firing: &AlertFiring) {
    let Some(rule) = cfg.rules.iter().find(|r| r.name == firing.rule) else {
        return;
    };
    for channel in &rule.channels {
        if let Some(path) = channel.strip_prefix("file:") {
            if let Err(e) = write_to_file(path, firing) {
                eprintln!("argo gateway: alert file write {path}: {e}");
            }
        } else if channel == "event" {
            if let Ok(line) = serde_json::to_string(firing) {
                eprintln!("argo gateway: alert.event {line}");
            }
        }
    }
}

fn write_to_file(path: &str, firing: &AlertFiring) -> std::io::Result<()> {
    use std::io::Write;
    if let Some(parent) = std::path::Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    let line = serde_json::to_string(firing).unwrap_or_else(|_| "{}".to_string());
    writeln!(f, "{line}")
}
