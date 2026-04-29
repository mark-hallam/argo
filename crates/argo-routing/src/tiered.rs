//! Cost-tier classifier router (§24.2).
//!
//! Runs the configured `classifier_model` against the fixed prompt, then
//! picks one of `{simple, medium, complex}` from the tier table. Falls
//! through to `default_model` if the classifier exceeds
//! `classify_threshold_ms`, errors, or returns garbage — `never halts the
//! loop` (§24.4).
use crate::cache::TierCache;
use crate::router::{Router, RoutingContext};
use argo_provider::{ProviderRequest, Sampling, Transport};
use argo_types::{
    ArgoResult, ContentBlock, Message, MessageId, MessageRole, ProviderModel, RoutingDecision,
    RoutingDecisionId, RoutingPolicy, SessionId, TieredRouting,
};
use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Hard cap on the user-message excerpt fed to the classifier
/// (§24.2 — "truncated to 800 tokens"). We approximate with a byte cap
/// since we don't yet have a tokenizer hooked up.
const CLASSIFIER_PROMPT_CHAR_BUDGET: usize = 3200;

/// The fixed classifier prompt prefix from §24.2.
const CLASSIFIER_PROMPT_PREFIX: &str =
    "Classify the following user request as `simple`, `medium`, or `complex`. \
    Reply with the single word only. Request: \"\"\"";

/// Manifest-driven cost-tier router.
///
/// ```
/// # async fn doc() -> argo_types::ArgoResult<()> {
/// use argo_provider::{OpenAiCompatTransport, ProviderManifest};
/// use argo_routing::TieredRouter;
/// use std::sync::Arc;
/// let toml = include_str!("../../../manifests/providers/openai-compatible.toml");
/// let manifest = ProviderManifest::from_toml_str(toml)?;
/// let transport = OpenAiCompatTransport::from_manifest(manifest, "sk-test")?;
/// let _router = TieredRouter::new(Arc::new(transport));
/// # Ok(()) }
/// ```
pub struct TieredRouter {
    classifier_transport: Arc<dyn Transport>,
    cache: TierCache,
}

impl std::fmt::Debug for TieredRouter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TieredRouter")
            .field("classifier_transport_id", &self.classifier_transport.id())
            .finish()
    }
}

impl TieredRouter {
    /// Build with the given classifier transport. The transport's model
    /// id is taken from `TieredRouting::classifier_model.model` at call
    /// time.
    #[must_use]
    pub fn new(classifier_transport: Arc<dyn Transport>) -> Self {
        Self {
            classifier_transport,
            cache: TierCache::default(),
        }
    }

    /// New router with a custom cache (for tests that want short TTLs).
    #[must_use]
    pub fn with_cache(classifier_transport: Arc<dyn Transport>, cache: TierCache) -> Self {
        Self {
            classifier_transport,
            cache,
        }
    }
}

#[async_trait]
impl Router for TieredRouter {
    async fn route(&self, ctx: &RoutingContext<'_>) -> ArgoResult<RoutingDecision> {
        let tiered = match &ctx.policy {
            RoutingPolicy::Tiered(t) => t.clone(),
            other => {
                return Err(argo_types::ArgoError::RoutingFailed {
                    reason: format!("TieredRouter requires RoutingPolicy::Tiered; got {other:?}"),
                })
            }
        };
        let msg_hash = TierCache::hash_message(ctx.user_message);
        if let Some(tier) = self.cache.get(ctx.session_id, msg_hash) {
            return Ok(decision_for_tier(ctx, &tiered, &tier, 0, "cache hit"));
        }
        let started = Instant::now();
        let outcome = classify(
            self.classifier_transport.as_ref(),
            &tiered,
            ctx.session_id,
            ctx.user_message,
            ctx.deadline,
        )
        .await;
        let elapsed_ms = started.elapsed().as_millis() as u64;
        let (tier, reason) = match outcome {
            ClassifyOutcome::Tier(t) => (t, "classified".to_string()),
            ClassifyOutcome::Slow => ("unrouted".into(), "classifier slow → default".into()),
            ClassifyOutcome::Error(e) => {
                if !tiered.fallback_to_default_on_error {
                    return Err(argo_types::ArgoError::RoutingFailed { reason: e });
                }
                ("unrouted".into(), format!("classifier error: {e}"))
            }
            ClassifyOutcome::Unknown(raw) => (
                "unrouted".into(),
                format!("classifier returned unknown tier: {raw}"),
            ),
        };
        self.cache.insert(ctx.session_id, msg_hash, tier.clone());
        Ok(decision_for_tier(ctx, &tiered, &tier, elapsed_ms, &reason))
    }
}

enum ClassifyOutcome {
    Tier(String),
    Slow,
    Error(String),
    Unknown(String),
}

fn decision_for_tier(
    ctx: &RoutingContext<'_>,
    tiered: &TieredRouting,
    tier: &str,
    classifier_latency_ms: u64,
    reason: &str,
) -> RoutingDecision {
    let chosen = match tier {
        "simple" => tiered.simple.clone(),
        "medium" => tiered.medium.clone(),
        "complex" => tiered.complex.clone(),
        _ => ctx.default_model.clone(),
    };
    RoutingDecision {
        id: RoutingDecisionId::new(),
        session_id: ctx.session_id,
        turn: ctx.turn,
        policy: "tiered".into(),
        tier: tier.to_string(),
        classified_at: Utc::now(),
        classifier_latency_ms,
        chosen_provider_model: chosen,
        reason: reason.to_string(),
    }
}

async fn classify(
    transport: &dyn Transport,
    tiered: &TieredRouting,
    session_id: SessionId,
    user_message: &str,
    deadline: Instant,
) -> ClassifyOutcome {
    let req = build_classifier_request(&tiered.classifier_model, session_id, user_message);
    let timeout = effective_timeout(tiered.classify_threshold_ms, deadline);
    match tokio::time::timeout(timeout, transport.complete(&req)).await {
        Ok(Ok(resp)) => match parse_tier(resp.text.as_deref().unwrap_or("")) {
            Some(tier) => ClassifyOutcome::Tier(tier),
            None => ClassifyOutcome::Unknown(resp.text.unwrap_or_default()),
        },
        Ok(Err(e)) => ClassifyOutcome::Error(e.to_string()),
        Err(_) => ClassifyOutcome::Slow,
    }
}

fn effective_timeout(threshold_ms: u64, deadline: Instant) -> Duration {
    let remaining = deadline.saturating_duration_since(Instant::now());
    let threshold = Duration::from_millis(threshold_ms);
    remaining.min(threshold)
}

fn build_classifier_request(
    classifier_model: &ProviderModel,
    session_id: SessionId,
    user_message: &str,
) -> ProviderRequest {
    let truncated = truncate_for_classifier(user_message);
    let prompt = format!("{CLASSIFIER_PROMPT_PREFIX}{truncated}\"\"\"");
    let user = Message {
        id: MessageId::new(),
        session_id,
        role: MessageRole::User,
        content: vec![ContentBlock::Text { text: prompt }],
        tool_calls: vec![],
        tool_call_id: None,
        reasoning: None,
        created_at: Utc::now(),
        provider_meta: serde_json::Value::Null,
    };
    ProviderRequest {
        model: classifier_model.model.clone(),
        messages: vec![user],
        tools: vec![],
        system: None,
        sampling: Sampling {
            temperature: Some(0.0),
            max_tokens: Some(4),
            ..Default::default()
        },
    }
}

fn truncate_for_classifier(s: &str) -> String {
    if s.len() <= CLASSIFIER_PROMPT_CHAR_BUDGET {
        return s.to_string();
    }
    let mut end = CLASSIFIER_PROMPT_CHAR_BUDGET;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    s[..end].to_string()
}

fn parse_tier(raw: &str) -> Option<String> {
    let trimmed = raw.trim().to_lowercase();
    let first_word = trimmed
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim_matches(|c: char| !c.is_alphabetic());
    match first_word {
        "simple" | "medium" | "complex" => Some(first_word.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
