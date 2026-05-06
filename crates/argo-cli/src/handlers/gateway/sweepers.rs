//! Tokio sweeper that drives the orchestrator handoff walker
//! (Stream `claude-orchestrator-handoff-walker`).
//!
//! Spawned by `gateway/mod.rs::start` after the session store is
//! open. Runs on a fixed interval (default 60s; override via
//! `ARGO_HANDOFF_SWEEP_SECS`) and calls
//! [`HandoffWalker::sweep_once`] on each tick.
//!
//! Address-scheme routing for `Human` steps lives here (in the
//! `SchemeRoutingNotifier`): `mailto:` → email, `slack:` →
//! slack, `sms:`/`tel:` → SMS, `expo:`/`apns:`/`fcm:` → push.
//! Unknown / ambiguous schemes fall back to email per the brief.
//! When the matching channel slot isn't wired, the notifier
//! returns an error and the walker treats it as chain exhaustion
//! — the operator still learns via the audit row.
use argo_orchestrator::{HandoffNotifier, HandoffWalker, OrchestratorStore};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;

/// Default sweep interval when `ARGO_HANDOFF_SWEEP_SECS` is unset.
pub const DEFAULT_SWEEP_SECS: u64 = 60;

/// Spawn the periodic sweeper. Returns `Some(handle)` if the
/// orchestrator schema is reachable; `None` when bootstrapping
/// failed (gateway logs and continues — `/health` etc. still
/// answer).
pub fn spawn_handoff_sweeper(store: Arc<SessionStore>) -> Option<JoinHandle<()>> {
    let interval_secs = std::env::var("ARGO_HANDOFF_SWEEP_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(DEFAULT_SWEEP_SECS);
    let pool = store.pool().clone();
    let orch_store = Arc::new(OrchestratorStore::new(Arc::clone(&store)));
    let notifier: Arc<dyn HandoffNotifier> = Arc::new(SchemeRoutingNotifier::log_only());
    let walker = HandoffWalker::new(Arc::clone(&orch_store), pool, notifier);
    let handle = tokio::spawn(async move {
        // Ensure schema is reachable before the first tick. If the
        // tables are missing we log + bail; the gateway keeps
        // running for callers that don't need orchestrator state.
        if let Err(e) = orch_store.ensure_schema().await {
            eprintln!("argo gateway: orchestrator schema unavailable ({e}); handoff sweeper disabled");
            return;
        }
        let mut ticker = tokio::time::interval(Duration::from_secs(interval_secs));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            let now = chrono::Utc::now();
            match walker.sweep_once(now).await {
                Ok(report) => {
                    if report.expired > 0 {
                        eprintln!(
                            "argo gateway: handoff sweeper — expired={} re_handed_off={} chain_exhausted={} notifications={}",
                            report.expired,
                            report.re_handed_off,
                            report.chain_exhausted,
                            report.notifications,
                        );
                    }
                }
                Err(e) => {
                    eprintln!("argo gateway: handoff sweeper error: {e}");
                }
            }
        }
    });
    Some(handle)
}

/// Address-scheme router. `notify_human` checks the prefix and
/// dispatches to the matching channel slot. Unknown / ambiguous
/// schemes fall back to email per the brief. The default
/// (`log_only`) construction has no channel slots wired — every
/// notify is logged + returns success so the operator still
/// gets a structured event in the gateway log even before the
/// real channel transports are wired into the gateway process.
pub struct SchemeRoutingNotifier {
    /// `true` → `notify_human` returns Ok with a synthetic
    /// delivery id and logs to stderr. Real wiring lands in a
    /// follow-up brief that adds NotifyChannel slots to the
    /// gateway start path.
    log_only: bool,
}

impl SchemeRoutingNotifier {
    /// Logging-only notifier — every `notify_human` succeeds with
    /// a synthetic delivery id and writes a structured stderr
    /// line. Used until the real channel wiring lands.
    #[must_use]
    pub fn log_only() -> Self {
        Self { log_only: true }
    }
}

#[async_trait]
impl HandoffNotifier for SchemeRoutingNotifier {
    async fn notify_human(&self, addr: &str, summary: &str) -> ArgoResult<String> {
        let channel = resolve_channel(addr);
        if self.log_only {
            eprintln!(
                "argo gateway: handoff notifier (log-only) — channel={channel} addr={addr} summary={summary}"
            );
            return Ok(format!("log:{channel}:{addr}"));
        }
        Err(ArgoError::Provider(format!(
            "no channel wired for scheme '{channel}' (addr {addr}); waiting on follow-up brief"
        )))
    }
}

/// Resolve an address scheme to a channel id. Per the brief:
/// `mailto:` → email, `slack:` → slack, `sms:`/`tel:` → sms,
/// `expo:`/`apns:`/`fcm:` → push. Anything else (including bare
/// `user@host` strings without a scheme) falls back to email.
pub fn resolve_channel(addr: &str) -> &'static str {
    let scheme = addr.split_once(':').map(|(s, _)| s.to_lowercase());
    match scheme.as_deref() {
        Some("mailto") | Some("email") => "email",
        Some("slack") => "slack",
        Some("sms") | Some("tel") => "sms",
        Some("expo") | Some("apns") | Some("fcm") | Some("push") => "push",
        _ => "email",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_email_scheme() {
        assert_eq!(resolve_channel("mailto:ops@acmeco.com"), "email");
        assert_eq!(resolve_channel("email:mark@acmeco.com"), "email");
    }

    #[test]
    fn resolve_slack_scheme() {
        assert_eq!(resolve_channel("slack:#oncall"), "slack");
    }

    #[test]
    fn resolve_sms_scheme() {
        assert_eq!(resolve_channel("sms:+15551234567"), "sms");
        assert_eq!(resolve_channel("tel:+15551234567"), "sms");
    }

    #[test]
    fn resolve_push_scheme() {
        assert_eq!(resolve_channel("expo:ExponentPushToken[abc]"), "push");
        assert_eq!(resolve_channel("apns:device-id"), "push");
        assert_eq!(resolve_channel("fcm:token"), "push");
    }

    #[test]
    fn resolve_unknown_falls_back_to_email() {
        assert_eq!(resolve_channel("unknown:thing"), "email");
        assert_eq!(resolve_channel("alice@example.com"), "email");
        assert_eq!(resolve_channel(""), "email");
    }

    #[tokio::test]
    async fn log_only_notifier_succeeds_with_synthetic_id() {
        let n = SchemeRoutingNotifier::log_only();
        let id = n
            .notify_human("mailto:test@example.com", "summary")
            .await
            .unwrap();
        assert!(id.starts_with("log:email:"));
    }
}
