//! Cost-tier routing types (Amendment 1).
use crate::ids::{RoutingDecisionId, SessionId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A `(provider, model)` pair.
///
/// ```
/// use argo_types::ProviderModel;
/// let model = ProviderModel { provider: "openai".into(), model: "gpt-4.1".into() };
/// let json = serde_json::to_string(&model).unwrap();
/// assert_eq!(serde_json::from_str::<ProviderModel>(&json).unwrap(), model);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderModel {
    /// Provider id (must match a manifest).
    pub provider: String,
    /// Model id within the provider.
    pub model: String,
}

/// Top-level routing policy.
///
/// ```
/// use argo_types::{ProviderModel, RoutingPolicy, TieredRouting};
/// let cheap = ProviderModel { provider: "openai".into(), model: "gpt-4.1-mini".into() };
/// let smart = ProviderModel { provider: "anthropic".into(), model: "claude-sonnet".into() };
/// let policy = RoutingPolicy::Tiered(TieredRouting {
///     classifier_model: cheap.clone(), simple: cheap.clone(), medium: smart.clone(),
///     complex: smart, classify_threshold_ms: 250, fallback_to_default_on_error: true,
/// });
/// assert!(serde_json::to_string(&policy).unwrap().contains("tiered"));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "policy", rename_all = "snake_case")]
pub enum RoutingPolicy {
    /// Always use the configured default model.
    Fixed,
    /// Tiered — classify first, then route by tier.
    Tiered(TieredRouting),
    /// Delegate to a plugin.
    PolicyPlugin {
        /// Plugin name.
        plugin: String,
    },
}

/// Tiered routing config.
///
/// ```
/// use argo_types::{ProviderModel, TieredRouting};
/// let model = ProviderModel { provider: "openai".into(), model: "gpt-4.1-mini".into() };
/// let routing = TieredRouting {
///     classifier_model: model.clone(), simple: model.clone(), medium: model.clone(),
///     complex: model, classify_threshold_ms: 500, fallback_to_default_on_error: true,
/// };
/// assert_eq!(routing.classify_threshold_ms, 500);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TieredRouting {
    /// Cheap classifier model.
    pub classifier_model: ProviderModel,
    /// Tier for simple prompts.
    pub simple: ProviderModel,
    /// Tier for medium prompts.
    pub medium: ProviderModel,
    /// Tier for complex prompts.
    pub complex: ProviderModel,
    /// Max classifier latency (ms); falls through to default if exceeded.
    pub classify_threshold_ms: u64,
    /// Whether to fall back to default on classifier error.
    pub fallback_to_default_on_error: bool,
}

/// A persisted routing decision (one per turn when policy ≠ Fixed).
///
/// ```
/// use argo_types::{ProviderModel, RoutingDecision, RoutingDecisionId, SessionId};
/// let decision = RoutingDecision {
///     id: RoutingDecisionId::new(), session_id: SessionId::new(), turn: 1,
///     policy: "tiered".into(), tier: "simple".into(), classified_at: chrono::Utc::now(),
///     classifier_latency_ms: 12,
///     chosen_provider_model: ProviderModel { provider: "openai".into(), model: "gpt-4.1-mini".into() },
///     reason: "short prompt".into(),
/// };
/// assert_eq!(decision.turn, 1);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDecision {
    /// Stable ID.
    pub id: RoutingDecisionId,
    /// Owning session.
    pub session_id: SessionId,
    /// Turn number.
    pub turn: u32,
    /// Policy in effect (e.g. `"tiered"`).
    pub policy: String,
    /// Chosen tier (`"simple"` / `"medium"` / `"complex"` / `"unrouted"`).
    pub tier: String,
    /// When the decision was made.
    pub classified_at: DateTime<Utc>,
    /// Classifier latency (ms).
    pub classifier_latency_ms: u64,
    /// Chosen model.
    pub chosen_provider_model: ProviderModel,
    /// Free-form rationale.
    pub reason: String,
}
