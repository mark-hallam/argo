use super::*;
use crate::router::RoutingContext;
use argo_provider::{
    ProviderRequest, ProviderResponse, ProviderResponseDelta, StopReason, Transport,
};
use argo_types::{
    ArgoError, ArgoResult, CapabilitySet, ProviderModel, RoutingPolicy, SessionId, TieredRouting,
};
use async_trait::async_trait;
use futures::stream::BoxStream;
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// A scripted classifier transport: each `complete()` call pulls the
/// next entry off the queue. Lets us drive deterministic tier outcomes,
/// errors, and slow responses from tests.
struct MockClassifier {
    script: Mutex<std::collections::VecDeque<MockResponse>>,
    calls: std::sync::atomic::AtomicUsize,
}

enum MockResponse {
    Text(&'static str),
    Err(&'static str),
    Slow(Duration),
}

impl MockClassifier {
    fn new(script: Vec<MockResponse>) -> Arc<Self> {
        Arc::new(Self {
            script: Mutex::new(script.into_iter().collect()),
            calls: std::sync::atomic::AtomicUsize::new(0),
        })
    }

    fn call_count(&self) -> usize {
        self.calls.load(std::sync::atomic::Ordering::SeqCst)
    }
}

#[async_trait]
impl Transport for MockClassifier {
    fn id(&self) -> &str {
        "mock-classifier"
    }
    fn capabilities(&self) -> &CapabilitySet {
        static EMPTY: std::sync::OnceLock<CapabilitySet> = std::sync::OnceLock::new();
        EMPTY.get_or_init(CapabilitySet::empty)
    }
    async fn complete(&self, _req: &ProviderRequest) -> ArgoResult<ProviderResponse> {
        self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let next = self.script.lock().pop_front();
        match next {
            Some(MockResponse::Text(t)) => Ok(ProviderResponse {
                text: Some(t.to_string()),
                reasoning: None,
                tool_calls: vec![],
                stop_reason: StopReason::EndTurn,
                tokens_in: None,
                tokens_out: None,
                provider_meta: serde_json::Value::Null,
            }),
            Some(MockResponse::Err(msg)) => Err(ArgoError::Provider(msg.into())),
            Some(MockResponse::Slow(d)) => {
                tokio::time::sleep(d).await;
                Ok(ProviderResponse {
                    text: Some("simple".into()),
                    reasoning: None,
                    tool_calls: vec![],
                    stop_reason: StopReason::EndTurn,
                    tokens_in: None,
                    tokens_out: None,
                    provider_meta: serde_json::Value::Null,
                })
            }
            None => Err(ArgoError::Provider("script exhausted".into())),
        }
    }
    async fn complete_stream(
        &self,
        _req: &ProviderRequest,
    ) -> ArgoResult<BoxStream<'static, ArgoResult<ProviderResponseDelta>>> {
        Err(ArgoError::Provider("not used".into()))
    }
}

fn pm(provider: &str, model: &str) -> ProviderModel {
    ProviderModel {
        provider: provider.into(),
        model: model.into(),
    }
}

fn tiered_policy() -> TieredRouting {
    TieredRouting {
        classifier_model: pm("openai", "gpt-4o-mini"),
        simple: pm("openai", "gpt-4o-mini"),
        medium: pm("openai", "gpt-4o"),
        complex: pm("anthropic", "claude-opus-4-6"),
        classify_threshold_ms: 80,
        fallback_to_default_on_error: true,
    }
}

fn ctx_with<'a>(
    session_id: SessionId,
    user_message: &'a str,
    policy: TieredRouting,
    deadline: Instant,
) -> RoutingContext<'a> {
    RoutingContext {
        session_id,
        turn: 1,
        messages: &[],
        user_message,
        tools_in_scope: &[],
        default_model: pm("openai", "gpt-4o-mini"),
        policy: RoutingPolicy::Tiered(policy),
        deadline,
    }
}

#[tokio::test]
async fn classifier_picks_simple_tier() {
    let classifier = MockClassifier::new(vec![MockResponse::Text("simple")]);
    let router = TieredRouter::new(classifier.clone());
    let ctx = ctx_with(
        SessionId::new(),
        "what's 2+2?",
        tiered_policy(),
        Instant::now() + Duration::from_secs(60),
    );
    let d = router.route(&ctx).await.unwrap();
    assert_eq!(d.tier, "simple");
    assert_eq!(d.policy, "tiered");
    assert_eq!(d.chosen_provider_model, pm("openai", "gpt-4o-mini"));
    assert_eq!(classifier.call_count(), 1);
}

#[tokio::test]
async fn classifier_picks_complex_tier() {
    let classifier = MockClassifier::new(vec![MockResponse::Text("complex")]);
    let router = TieredRouter::new(classifier);
    let ctx = ctx_with(
        SessionId::new(),
        "design a distributed rate limiter",
        tiered_policy(),
        Instant::now() + Duration::from_secs(60),
    );
    let d = router.route(&ctx).await.unwrap();
    assert_eq!(d.tier, "complex");
    assert_eq!(d.chosen_provider_model, pm("anthropic", "claude-opus-4-6"));
}

#[tokio::test]
async fn unknown_classifier_output_falls_through_to_default() {
    let classifier = MockClassifier::new(vec![MockResponse::Text("uhhh whatever")]);
    let router = TieredRouter::new(classifier);
    let ctx = ctx_with(
        SessionId::new(),
        "hi",
        tiered_policy(),
        Instant::now() + Duration::from_secs(60),
    );
    let d = router.route(&ctx).await.unwrap();
    assert_eq!(d.tier, "unrouted");
    assert_eq!(d.chosen_provider_model, pm("openai", "gpt-4o-mini"));
    assert!(d.reason.contains("unknown tier"));
}

#[tokio::test]
async fn classifier_error_with_fallback_does_not_halt() {
    // §24.4: classifier crash → falls through to default; never halts.
    let classifier = MockClassifier::new(vec![MockResponse::Err("simulated 500")]);
    let router = TieredRouter::new(classifier);
    let ctx = ctx_with(
        SessionId::new(),
        "anything",
        tiered_policy(),
        Instant::now() + Duration::from_secs(60),
    );
    let d = router.route(&ctx).await.unwrap();
    assert_eq!(d.tier, "unrouted");
    assert!(d.reason.contains("simulated 500"));
}

#[tokio::test]
async fn classifier_error_without_fallback_propagates() {
    let mut policy = tiered_policy();
    policy.fallback_to_default_on_error = false;
    let classifier = MockClassifier::new(vec![MockResponse::Err("nope")]);
    let router = TieredRouter::new(classifier);
    let ctx = ctx_with(
        SessionId::new(),
        "x",
        policy,
        Instant::now() + Duration::from_secs(60),
    );
    let err = router.route(&ctx).await.unwrap_err();
    assert!(matches!(err, ArgoError::RoutingFailed { .. }));
}

#[tokio::test]
async fn classifier_slower_than_threshold_falls_through() {
    // Mock sleeps 200ms; threshold is 50ms. Must fall through.
    let classifier = MockClassifier::new(vec![MockResponse::Slow(Duration::from_millis(200))]);
    let mut policy = tiered_policy();
    policy.classify_threshold_ms = 50;
    let router = TieredRouter::new(classifier);
    let ctx = ctx_with(
        SessionId::new(),
        "blah",
        policy,
        Instant::now() + Duration::from_secs(60),
    );
    let d = router.route(&ctx).await.unwrap();
    assert_eq!(d.tier, "unrouted");
    assert!(d.reason.contains("slow"));
}

#[tokio::test]
async fn cache_hit_skips_classifier_call() {
    // Two routes, but only one classifier slot — second must come from cache.
    let classifier = MockClassifier::new(vec![MockResponse::Text("medium")]);
    let router = TieredRouter::new(classifier.clone());
    let session = SessionId::new();
    let ctx = ctx_with(
        session,
        "review this PR",
        tiered_policy(),
        Instant::now() + Duration::from_secs(60),
    );
    let first = router.route(&ctx).await.unwrap();
    assert_eq!(first.tier, "medium");
    let second = router.route(&ctx).await.unwrap();
    assert_eq!(second.tier, "medium");
    assert_eq!(second.reason, "cache hit");
    assert_eq!(second.classifier_latency_ms, 0);
    assert_eq!(classifier.call_count(), 1);
}

#[tokio::test]
async fn rejects_non_tiered_policy() {
    let classifier = MockClassifier::new(vec![]);
    let router = TieredRouter::new(classifier);
    let ctx = RoutingContext {
        session_id: SessionId::new(),
        turn: 1,
        messages: &[],
        user_message: "x",
        tools_in_scope: &[],
        default_model: pm("openai", "gpt-4o-mini"),
        policy: RoutingPolicy::Fixed,
        deadline: Instant::now() + Duration::from_secs(60),
    };
    let err = router.route(&ctx).await.unwrap_err();
    assert!(matches!(err, ArgoError::RoutingFailed { .. }));
}

#[test]
fn parse_tier_strips_punctuation_and_case() {
    assert_eq!(parse_tier("simple").as_deref(), Some("simple"));
    assert_eq!(parse_tier("SIMPLE.").as_deref(), Some("simple"));
    assert_eq!(parse_tier("  Complex\n").as_deref(), Some("complex"));
    assert_eq!(parse_tier("MEDIUM"), Some("medium".to_string()));
    // Stricter: "medium-ish" is a hedge — refuse rather than guess.
    assert_eq!(parse_tier("medium-ish"), None);
    assert_eq!(parse_tier("undefined"), None);
    assert_eq!(parse_tier(""), None);
}

#[test]
fn truncate_for_classifier_respects_byte_budget_and_utf8() {
    let s = "a".repeat(CLASSIFIER_PROMPT_CHAR_BUDGET + 10);
    let out = truncate_for_classifier(&s);
    assert!(out.len() <= CLASSIFIER_PROMPT_CHAR_BUDGET);
    let multibyte = "é".repeat(CLASSIFIER_PROMPT_CHAR_BUDGET);
    let out = truncate_for_classifier(&multibyte);
    assert!(out.len() <= CLASSIFIER_PROMPT_CHAR_BUDGET);
    assert!(std::str::from_utf8(out.as_bytes()).is_ok());
}
