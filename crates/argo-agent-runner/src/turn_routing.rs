//! Tier-routing wiring for the per-turn agent loop (Stream
//! `mission-claim-3-runtime-wiring-v1x`).
//!
//! Consulted by `turn::run_turn_with_events_inner` whenever a
//! session's `routing_policy` selects `RoutingPolicy::Tiered`. The
//! tier router classifies the user's prompt against the configured
//! `classifier_model` and returns a `RoutingDecision` whose
//! `chosen_provider_model.model` overrides the session's default
//! model for the dispatched turn. Sessions with `None` (the v1.0
//! default) skip this path entirely.
//!
//! `argo-routing` stays the canonical owner of the routing
//! algorithm — this module is a pure consumer that adapts the
//! agent-loop's per-turn inputs into a [`RoutingContext`] and emits
//! an `AuditKind::RoutingDecision` row for every routed turn.
use crate::transport::build_transport;
use argo_audit::{AuditKind, AuditSink};
use argo_provider::{ProviderManifest, Transport};
use argo_routing::{Router, RoutingContext, TieredRouter};
use argo_store::{SessionHeader, SessionStore};
use argo_types::{ArgoResult, Message, ProviderModel, RoutingDecision, RoutingPolicy, SessionId};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Wall-clock cap for the entire routing decision (classifier round
/// trip + decode). `TieredRouting::classify_threshold_ms` further
/// trims this; the smaller of the two wins.
const DEFAULT_ROUTING_DEADLINE_MS: u64 = 5_000;

/// Build a classifier `Transport` for the session's provider
/// manifest. The classifier model id is taken from
/// `TieredRouting::classifier_model.model` at request time, so this
/// is the same wire-up as `build_transport`; we just hand back an
/// `Arc<dyn Transport>` so the agent loop can keep its own
/// `Box<dyn Transport>` for the main turn without cloning.
pub async fn build_classifier_transport(
    manifest: ProviderManifest,
) -> ArgoResult<Arc<dyn Transport>> {
    Ok(Arc::from(build_transport(manifest).await?))
}

/// Run the tier router when the session carries
/// `RoutingPolicy::Tiered` and return the decision; return `None`
/// for sessions with no policy or a non-tier policy (the agent loop
/// then keeps `session.model`).
pub async fn maybe_route_turn(
    store: &Arc<SessionStore>,
    session: &SessionHeader,
    manifest: ProviderManifest,
    user_message: Option<&str>,
    audit: &AuditSink,
) -> ArgoResult<Option<RoutingDecision>> {
    let Some(policy) = session.routing_policy.as_ref() else {
        return Ok(None);
    };
    if !matches!(policy, RoutingPolicy::Tiered(_)) {
        return Ok(None);
    }
    let classifier = build_classifier_transport(manifest).await?;
    maybe_route_turn_with_transport(store, session, classifier, user_message, audit).await
}

/// Test-friendly entry point that bypasses the manifest-driven
/// transport build: callers supply their own classifier `Transport`.
/// The agent loop's `maybe_route_turn` is a thin wrapper that builds
/// the transport from the manifest and forwards.
pub async fn maybe_route_turn_with_transport(
    store: &Arc<SessionStore>,
    session: &SessionHeader,
    classifier: Arc<dyn Transport>,
    user_message: Option<&str>,
    audit: &AuditSink,
) -> ArgoResult<Option<RoutingDecision>> {
    let Some(policy) = session.routing_policy.as_ref() else {
        return Ok(None);
    };
    if !matches!(policy, RoutingPolicy::Tiered(_)) {
        return Ok(None);
    }
    let turn = count_user_turns(store, session.id).await;
    let user_msg = user_message.unwrap_or("");
    let messages: Vec<Message> = Vec::new();
    let default_model = ProviderModel {
        provider: session.provider.clone(),
        model: session.model.clone(),
    };
    let router = TieredRouter::new(classifier);
    let deadline = Instant::now() + Duration::from_millis(DEFAULT_ROUTING_DEADLINE_MS);
    let ctx = RoutingContext {
        session_id: session.id,
        turn,
        messages: messages.as_slice(),
        user_message: user_msg,
        tools_in_scope: &[],
        default_model,
        policy: policy.clone(),
        deadline,
    };
    let decision = router.route(&ctx).await?;
    let payload = serde_json::json!({
        "session_id": session.id.0.to_string(),
        "turn": decision.turn,
        "tier": decision.tier,
        "model": decision.chosen_provider_model.model,
        "provider": decision.chosen_provider_model.provider,
        "elapsed_ms": decision.classifier_latency_ms,
    });
    // Never fail the turn on audit-write IO; the decision is still
    // returned so the agent loop can route. Matches the
    // best-effort semantics of TurnStart/TurnEnd writes elsewhere
    // in this file.
    let _ = audit
        .write(Some(session.id), AuditKind::RoutingDecision, payload, None)
        .await;
    Ok(Some(decision))
}

/// Compute the 1-based turn number for the current user message.
/// Counts persisted `role = 'user'` messages on the session — the
/// caller appends the new user row before invoking the router so the
/// returned count already includes this turn. Returns 0 when the
/// message read fails; the decision is still emitted, just with a
/// less precise turn label.
async fn count_user_turns(store: &Arc<SessionStore>, session_id: SessionId) -> u32 {
    let Ok(rows) = store.list_messages(session_id).await else {
        return 0;
    };
    u32::try_from(rows.iter().filter(|m| m.role == "user").count()).unwrap_or(0)
}
