//! Event-source anomaly dispatch over approval notification channels.
//!
//! `handlers/alerts.rs::AlertKind` is for operator-authored rules
//! over metric snapshots. The types here are for immediate anomaly
//! emission at the event source: breaker opens, budget ceiling hits,
//! gateway boot fails, or the audit chain breaks.

use crate::{Notification, NotifyChannel, OperatorTarget, RiskLevel};
use argo_config::AnomalyAlertingConfig;
use argo_types::{AnomalyAlert, AnomalyDispatch, AnomalyKind, ArgoError, ArgoResult, Severity};
use async_trait::async_trait;
use chrono::{Duration, Utc};
use std::collections::BTreeMap;
use std::sync::Arc;

/// No-op dispatcher used when `[runtime.anomaly_alerting]` is disabled.
#[derive(Debug, Default)]
pub struct NoopDispatch;

#[async_trait]
impl AnomalyDispatch for NoopDispatch {
    async fn dispatch(&self, _alert: AnomalyAlert) -> ArgoResult<()> {
        Ok(())
    }
}

/// Fan-out dispatcher backed by concrete [`NotifyChannel`] instances.
pub struct MultiChannelDispatch {
    channels: BTreeMap<String, Arc<dyn NotifyChannel>>,
    routing: BTreeMap<AnomalyKind, Vec<String>>,
    threshold: Severity,
}

impl MultiChannelDispatch {
    /// Build a dispatcher from explicit channel registry and routing.
    #[must_use]
    pub fn new(
        channels: BTreeMap<String, Arc<dyn NotifyChannel>>,
        routing: BTreeMap<AnomalyKind, Vec<String>>,
        threshold: Severity,
    ) -> Self {
        Self {
            channels,
            routing,
            threshold,
        }
    }

    /// Build from `argo.toml` runtime config.
    #[must_use]
    pub fn from_config(
        cfg: &AnomalyAlertingConfig,
        channels: BTreeMap<String, Arc<dyn NotifyChannel>>,
    ) -> Arc<dyn AnomalyDispatch> {
        if !cfg.enabled {
            return Arc::new(NoopDispatch);
        }
        let routing = cfg
            .channel_routing
            .iter()
            .map(|(kind, channels)| (AnomalyKind::parse(kind), channels.clone()))
            .collect();
        Arc::new(Self::new(
            channels,
            routing,
            Severity::parse(&cfg.default_severity_threshold),
        ))
    }

    fn route_for(&self, kind: &AnomalyKind) -> Vec<String> {
        self.routing
            .get(kind)
            .cloned()
            .unwrap_or_else(|| self.channels.keys().cloned().collect())
    }
}

#[async_trait]
impl AnomalyDispatch for MultiChannelDispatch {
    async fn dispatch(&self, alert: AnomalyAlert) -> ArgoResult<()> {
        if alert.severity < self.threshold {
            return Ok(());
        }
        let mut failures = Vec::new();
        for id in self.route_for(&alert.kind) {
            let Some(channel) = self.channels.get(&id) else {
                continue;
            };
            if let Err(e) = channel.send(notification_for(&alert)).await {
                failures.push(format!("{id}: {e}"));
            }
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(ArgoError::Provider(format!(
                "anomaly dispatch failed: {}",
                failures.join("; ")
            )))
        }
    }
}

fn notification_for(alert: &AnomalyAlert) -> Notification {
    let context = if alert.context.is_empty() {
        String::new()
    } else {
        let pairs = alert
            .context
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join(", ");
        format!("\n\nContext: {pairs}")
    };
    let now = Utc::now();
    Notification {
        target: OperatorTarget {
            operator_id: "operator".into(),
            email: "operator@example.invalid".into(),
        },
        action_id: format!("anomaly:{}", alert.kind),
        approval_id: format!("anomaly:{}:{}", alert.kind, now.timestamp_millis()),
        risk_level: RiskLevel::High,
        prompt_body: format!("[{:?}] {}{}", alert.severity, alert.message, context),
        approve_url: String::new(),
        reject_url: String::new(),
        expiry: now + Duration::minutes(15),
    }
}

/// Test helper for building a channel registry.
#[cfg(test)]
pub(crate) fn registry(
    channels: Vec<Arc<dyn NotifyChannel>>,
) -> BTreeMap<String, Arc<dyn NotifyChannel>> {
    channels
        .into_iter()
        .map(|ch| (ch.id().to_string(), ch))
        .collect()
}

/// Test helper for constructing a sent receipt.
#[cfg(test)]
pub(crate) fn receipt(channel_id: crate::ChannelId) -> crate::DeliveryReceipt {
    crate::DeliveryReceipt {
        channel_id,
        notification_id: "test".into(),
        provider_message_id: None,
        status: crate::DeliveryStatus::Sent,
        deferred_reason: None,
        delivered_at: Utc::now(),
    }
}

#[cfg(test)]
#[path = "dispatch_tests.rs"]
mod tests;
