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
use crate::transport::build_transport_for_tenant;
use crate::TransportGate;
use argo_audit::{AuditKind, AuditSink};
use argo_provider::{ProviderManifest, Transport};
use argo_routing::{Router, RoutingContext, TieredRouter};
use argo_store::{SessionHeader, SessionStore};
use argo_types::{
    ArgoResult, Message, ProviderModel, RoutingDecision, RoutingPolicy, SessionId, TenantId,
};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Wall-clock cap for the entire routing decision (classifier round
/// trip + decode). `TieredRouting::classify_threshold_ms` further
/// trims this; the smaller of the two wins.
const DEFAULT_ROUTING_DEADLINE_MS: u64 = 5_000;

/// Build a classifier `Transport` for the session's provider
/// manifest, honoring per-tenant BYOK key precedence (DEC-053
/// Enterprise). The classifier model id is taken from
/// `TieredRouting::classifier_model.model` at request time; we hand
/// back an `Arc<dyn Transport>` so the agent loop can keep its own
/// `Box<dyn Transport>` for the main turn without cloning.
///
/// This is the same per-tenant wire-up as the main turn loop
/// ([`build_transport_for_tenant`]): a BYOK tenant's own provider key
/// (stored under `[credentials.tenant.<tid>]`) WINS over the platform
/// key, so an Enterprise tenant's routing-stage classifier call bills
/// THEIR account, not SKYCOT's. `allow_platform_fallback = true` keeps
/// non-BYOK tenants byte-identical to the previous platform-key path.
pub async fn build_classifier_transport(
    manifest: ProviderManifest,
    workspace: &Path,
    tenant_id: &TenantId,
    gate: TransportGate<'_>,
) -> ArgoResult<Arc<dyn Transport>> {
    Ok(Arc::from(
        build_transport_for_tenant(manifest, workspace, tenant_id, true, gate).await?,
    ))
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
    cwd: &Path,
) -> ArgoResult<Option<RoutingDecision>> {
    let Some(policy) = session.routing_policy.as_ref() else {
        return Ok(None);
    };
    if !matches!(policy, RoutingPolicy::Tiered(_)) {
        return Ok(None);
    }
    // Same hard `[budget.ceiling]` gate as the main turn: the small
    // classifier round-trip is an authed `(tenant, session)` provider call,
    // so it is refused once the tenant is at its ceiling. READ-ONLY.
    let gate = TransportGate::Session {
        store,
        tenant_id: session.tenant_id,
        session_id: session.id,
        cwd,
    };
    let classifier =
        build_classifier_transport(manifest, cwd, &session.tenant_id, gate).await?;
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
        tenant_id: session.tenant_id,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Load the shipped anthropic manifest and rewrite its
    /// `api_key_env` to a unique per-test name so the env reads can't
    /// race the parallel harness, and so the platform env for that key
    /// is naturally absent.
    fn manifest_with_env(api_key_env: &str) -> ProviderManifest {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(std::path::Path::parent)
            .expect("workspace root")
            .join("manifests/providers/anthropic.toml");
        let body = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read fixture {}: {e}", path.display()))
            .replace(
                "api_key_env = \"ANTHROPIC_API_KEY\"",
                &format!("api_key_env = \"{api_key_env}\""),
            );
        ProviderManifest::from_toml_str(&body).expect("parse manifest")
    }

    /// Pins that the routing-stage classifier resolves the BYOK
    /// per-tenant key (DEC-053 Enterprise). The discriminator: a UNIQUE
    /// `api_key_env` whose process-env value is absent — so the only way
    /// to build a transport is via the tenant's OWN stored key. If
    /// `build_classifier_transport` had used the platform-only
    /// `build_transport`, this would fail with "<env> not set" because
    /// the platform key is absent. Success therefore proves the
    /// routing-stage transport flows through `build_transport_for_tenant`.
    #[tokio::test]
    async fn routing_stage_resolves_tenant_byok_key() {
        let ws = tempfile::tempdir().expect("ws tempdir");
        let tenant = TenantId::new();
        let env_key = "BYOK_ROUTING_STAGE_TENANT_WINS";
        argo_config::cred::set_tenant(ws.path(), &tenant, env_key, "TENANT-ROUTING-KEY")
            .expect("set tenant key");
        let manifest = manifest_with_env(env_key);

        // InternalTrusted gate keeps the test free of a live store — the
        // gate is a no-op wrapper and is orthogonal to key resolution.
        let result =
            build_classifier_transport(manifest, ws.path(), &tenant, TransportGate::InternalTrusted)
                .await;

        assert!(
            result.is_ok(),
            "routing-stage classifier must resolve the tenant's BYOK key \
             (platform env {env_key} is absent, so success proves per-tenant \
             resolution via build_transport_for_tenant); got: {:?}",
            result.err()
        );
    }
}
