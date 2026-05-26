//! Truth-boundary enforcement for final assistant utterances.
//!
//! The provider's assistant message is already persisted by the time
//! this module runs. The gate controls only the text delivered to the
//! channel and writes a first-class audit event for the verdict.

use super::turn_helpers_run::RoundOutcome;
use crate::helpers::{emit_event, RunEventSink};
use argo_audit::{AuditKind, AuditSink};
use argo_capability::{
    evaluate_claim, load_policy, load_refusal_templates, ChannelKind, ClaimCandidate,
    ClaimDisposition, ClaimEvaluation, ClaimProvenance, ProvenanceSource, RefusalTemplateRegistry,
    TruthBoundaryPolicy,
};
use argo_store::approvals::{create_request, DefaultDecision, NewApprovalRequest};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId, TenantId};
use chrono::{Duration, Utc};
#[cfg(test)]
use std::future::Future;
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

const POLICY_PATH_ENV: &str = "ARGO_TRUTH_BOUNDARY_POLICY_PATH";
const REFUSAL_TEMPLATES_DIR_ENV: &str = "ARGO_TRUTH_BOUNDARY_REFUSAL_TEMPLATES_DIR";
const CHANNEL_KIND_ENV: &str = "ARGO_TRUTH_BOUNDARY_CHANNEL_KIND";
const APPROVAL_DEADLINE_HOURS: i64 = 24;

#[cfg(test)]
tokio::task_local! {
    static TEST_TRUTH_BOUNDARY_CTX: TruthBoundaryTurnCtx;
}

/// Per-turn truth-boundary state resolved before the provider loop.
#[derive(Debug, Clone)]
pub(super) struct TruthBoundaryTurnCtx {
    pub(super) policy: Option<Arc<TruthBoundaryPolicy>>,
    pub(super) channel: ChannelKind,
    pub(super) refusal_templates: Arc<RefusalTemplateRegistry>,
}

impl TruthBoundaryTurnCtx {
    /// No-op gate for sessions without an active policy.
    #[must_use]
    pub(super) fn disabled() -> Self {
        Self {
            policy: None,
            channel: ChannelKind::Chat,
            refusal_templates: Arc::new(RefusalTemplateRegistry::empty()),
        }
    }

    /// Build an active gate from caller-resolved session state.
    #[must_use]
    pub(super) fn new(
        policy: TruthBoundaryPolicy,
        channel: ChannelKind,
        refusal_templates: RefusalTemplateRegistry,
    ) -> Self {
        Self {
            policy: Some(Arc::new(policy)),
            channel,
            refusal_templates: Arc::new(refusal_templates),
        }
    }

    fn active_policy_for(&self, tenant_id: TenantId) -> Option<&TruthBoundaryPolicy> {
        let policy = self.policy.as_deref()?;
        (policy.tenant_id == tenant_id).then_some(policy)
    }
}

/// Load optional truth-boundary state from process configuration.
///
/// Session boot will eventually set this state directly. Until that
/// channel wiring lands, the runner defaults to no-op and can be
/// exercised by tests or operators through explicit env paths.
pub(super) fn load_truth_boundary_ctx(
    _cwd: &Path,
    _tenant_id: TenantId,
) -> ArgoResult<TruthBoundaryTurnCtx> {
    #[cfg(test)]
    if let Ok(ctx) = TEST_TRUTH_BOUNDARY_CTX.try_with(Clone::clone) {
        return Ok(ctx);
    }
    let Some(policy_path) = std::env::var_os(POLICY_PATH_ENV) else {
        return Ok(TruthBoundaryTurnCtx::disabled());
    };
    let policy = load_policy(Path::new(&policy_path))?;
    let channel = load_channel_kind()?;
    let refusal_templates = load_refusal_template_registry()?;
    Ok(TruthBoundaryTurnCtx::new(
        policy,
        channel,
        refusal_templates,
    ))
}

#[cfg(test)]
pub(super) async fn with_test_truth_boundary_ctx<F, Fut, T>(ctx: TruthBoundaryTurnCtx, f: F) -> T
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = T>,
{
    TEST_TRUTH_BOUNDARY_CTX.scope(ctx, f()).await
}

/// Apply the policy gate to one final no-tool-call provider outcome.
#[allow(clippy::too_many_arguments)]
pub(super) async fn apply_truth_boundary_gate(
    ctx: &TruthBoundaryTurnCtx,
    store: &Arc<SessionStore>,
    audit: &AuditSink,
    events: Option<&RunEventSink>,
    session_id: SessionId,
    tenant_id: TenantId,
    outcome: RoundOutcome,
) -> ArgoResult<RoundOutcome> {
    let Some(policy) = ctx.active_policy_for(tenant_id) else {
        return Ok(outcome);
    };
    let candidate = runtime_candidate(&outcome);
    let mut eval = evaluate_claim(
        policy,
        ctx.channel,
        &ClaimCandidate {
            text: candidate.text.clone(),
            declared_classes: candidate.declared_classes.clone(),
        },
    );
    eval.provenance = candidate.provenance;
    apply_evaluation(
        ctx, store, audit, events, session_id, tenant_id, outcome, eval,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn apply_evaluation(
    ctx: &TruthBoundaryTurnCtx,
    store: &Arc<SessionStore>,
    audit: &AuditSink,
    events: Option<&RunEventSink>,
    session_id: SessionId,
    tenant_id: TenantId,
    outcome: RoundOutcome,
    eval: ClaimEvaluation,
) -> ArgoResult<RoundOutcome> {
    let policy = ctx
        .active_policy_for(tenant_id)
        .ok_or_else(|| ArgoError::Provider("truth-boundary policy disappeared".into()))?;
    match &eval.disposition {
        ClaimDisposition::Allow => {
            emit_claim_allowed(
                audit, events, session_id, tenant_id, policy, ctx, &outcome, &eval,
            )
            .await?;
            Ok(outcome)
        }
        ClaimDisposition::Forbid {
            refusal_template_id,
        } => {
            emit_claim_blocked(
                audit, events, session_id, tenant_id, policy, ctx, &outcome, &eval,
            )
            .await?;
            let refusal =
                pick_refusal_variant(&ctx.refusal_templates, refusal_template_id, session_id)?;
            Ok(outcome.with_text(refusal))
        }
        ClaimDisposition::Escalate => {
            let approval_id = enqueue_approval(store, tenant_id, &outcome.text).await?;
            emit_claim_escalated(
                audit,
                events,
                session_id,
                tenant_id,
                policy,
                ctx,
                &eval,
                &approval_id,
            )
            .await?;
            Ok(outcome.with_text(pending_approval_message(&approval_id)))
        }
    }
}

fn runtime_candidate(outcome: &RoundOutcome) -> RuntimeClaimCandidate {
    RuntimeClaimCandidate {
        text: outcome.text.clone(),
        declared_classes: outcome.declared_claim_classes.clone(),
        provenance: outcome.claim_provenance.clone(),
    }
}

#[derive(Debug, Clone)]
struct RuntimeClaimCandidate {
    text: String,
    declared_classes: Vec<String>,
    provenance: ClaimProvenance,
}

#[allow(clippy::too_many_arguments)]
async fn emit_claim_allowed(
    audit: &AuditSink,
    events: Option<&RunEventSink>,
    session_id: SessionId,
    tenant_id: TenantId,
    policy: &TruthBoundaryPolicy,
    ctx: &TruthBoundaryTurnCtx,
    outcome: &RoundOutcome,
    eval: &ClaimEvaluation,
) -> ArgoResult<()> {
    emit_claim_event(
        audit,
        events,
        session_id,
        AuditKind::ClaimAllowed,
        "ClaimAllowed",
        base_payload(tenant_id, policy, ctx, outcome, eval),
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn emit_claim_blocked(
    audit: &AuditSink,
    events: Option<&RunEventSink>,
    session_id: SessionId,
    tenant_id: TenantId,
    policy: &TruthBoundaryPolicy,
    ctx: &TruthBoundaryTurnCtx,
    outcome: &RoundOutcome,
    eval: &ClaimEvaluation,
) -> ArgoResult<()> {
    emit_claim_event(
        audit,
        events,
        session_id,
        AuditKind::ClaimBlocked,
        "ClaimBlocked",
        base_payload(tenant_id, policy, ctx, outcome, eval),
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn emit_claim_escalated(
    audit: &AuditSink,
    events: Option<&RunEventSink>,
    session_id: SessionId,
    tenant_id: TenantId,
    policy: &TruthBoundaryPolicy,
    ctx: &TruthBoundaryTurnCtx,
    eval: &ClaimEvaluation,
    approval_id: &str,
) -> ArgoResult<()> {
    let mut payload = base_verdict_payload(tenant_id, policy, ctx, eval);
    payload["approval_id"] = serde_json::Value::String(approval_id.to_string());
    emit_claim_event(
        audit,
        events,
        session_id,
        AuditKind::ClaimEscalated,
        "ClaimEscalated",
        payload,
    )
    .await
}

async fn emit_claim_event(
    audit: &AuditSink,
    events: Option<&RunEventSink>,
    session_id: SessionId,
    kind: AuditKind,
    event_name: &str,
    payload: serde_json::Value,
) -> ArgoResult<()> {
    audit
        .write(Some(session_id), kind, payload.clone(), None)
        .await?;
    emit_event(events, event_name, payload);
    Ok(())
}

fn base_payload(
    tenant_id: TenantId,
    policy: &TruthBoundaryPolicy,
    ctx: &TruthBoundaryTurnCtx,
    outcome: &RoundOutcome,
    eval: &ClaimEvaluation,
) -> serde_json::Value {
    let mut payload = base_verdict_payload(tenant_id, policy, ctx, eval);
    payload["candidate_excerpt"] = serde_json::Value::String(candidate_excerpt(&outcome.text));
    payload
}

fn base_verdict_payload(
    tenant_id: TenantId,
    policy: &TruthBoundaryPolicy,
    ctx: &TruthBoundaryTurnCtx,
    eval: &ClaimEvaluation,
) -> serde_json::Value {
    serde_json::json!({
        "tenant_id": tenant_id.to_string(),
        "policy_id": policy.id.as_str(),
        "channel": channel_label(ctx.channel),
        "matched_class": eval.matched_class.as_deref(),
        "confidence": eval.confidence,
        "provenance": provenance_payload(&eval.provenance),
    })
}

fn provenance_payload(provenance: &ClaimProvenance) -> serde_json::Value {
    serde_json::json!({
        "source": provenance_source_label(provenance.source),
        "source_id": provenance.source_id.as_str(),
        "source_excerpt": candidate_excerpt(&provenance.source_excerpt),
    })
}

pub(super) fn pick_refusal_variant(
    registry: &RefusalTemplateRegistry,
    template_id: &str,
    session_id: SessionId,
) -> ArgoResult<String> {
    let template = registry
        .get(template_id)
        .ok_or_else(|| ArgoError::Schema(format!("missing refusal template '{template_id}'")))?;
    if template.variants.is_empty() {
        return Err(ArgoError::Schema(format!(
            "refusal template '{template_id}' has no variants"
        )));
    }
    let index = stable_variant_index(session_id, template_id, template.variants.len());
    Ok(template.variants[index].clone())
}

fn stable_variant_index(session_id: SessionId, template_id: &str, len: usize) -> usize {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for b in session_id.0.as_bytes().iter().chain(template_id.as_bytes()) {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    (hash as usize) % len
}

async fn enqueue_approval(
    store: &Arc<SessionStore>,
    tenant_id: TenantId,
    candidate_text: &str,
) -> ArgoResult<String> {
    let action_id = Uuid::now_v7().as_bytes().to_vec();
    let deadline_at = Utc::now()
        .checked_add_signed(Duration::hours(APPROVAL_DEADLINE_HOURS))
        .unwrap_or_else(Utc::now)
        .timestamp_micros();
    create_request(
        store.pool(),
        &NewApprovalRequest {
            action_id,
            tenant_id,
            requested_by: "truth-boundary",
            deadline_at,
            default_decision: DefaultDecision::Deny,
            draft_content: Some(candidate_text),
        },
    )
    .await
}

fn pending_approval_message(approval_id: &str) -> String {
    format!("I need approval before I can answer that. Approval request: {approval_id}")
}

pub(super) fn default_claim_provenance() -> ClaimProvenance {
    ClaimProvenance {
        source: ProvenanceSource::Generative,
        source_id: String::new(),
        source_excerpt: String::new(),
    }
}

fn load_channel_kind() -> ArgoResult<ChannelKind> {
    let Some(raw) = std::env::var_os(CHANNEL_KIND_ENV) else {
        return Ok(ChannelKind::Chat);
    };
    parse_channel_kind(&raw.to_string_lossy())
}

pub(super) fn parse_channel_kind(raw: &str) -> ArgoResult<ChannelKind> {
    match raw.to_ascii_lowercase().as_str() {
        "rehearsal" => Ok(ChannelKind::Rehearsal),
        "webembed" | "web_embed" | "web-embed" => Ok(ChannelKind::WebEmbed),
        "livemeeting" | "live_meeting" | "live-meeting" => Ok(ChannelKind::LiveMeeting),
        "chat" => Ok(ChannelKind::Chat),
        other => Err(ArgoError::Schema(format!(
            "unknown truth-boundary channel kind '{other}'"
        ))),
    }
}

fn load_refusal_template_registry() -> ArgoResult<RefusalTemplateRegistry> {
    let Some(dir) = std::env::var_os(REFUSAL_TEMPLATES_DIR_ENV) else {
        return Ok(RefusalTemplateRegistry::empty());
    };
    Ok(RefusalTemplateRegistry::new(load_refusal_templates(
        Path::new(&dir),
    )?))
}

fn candidate_excerpt(text: &str) -> String {
    const LIMIT: usize = 240;
    let mut out = text.chars().take(LIMIT).collect::<String>();
    if text.chars().count() > LIMIT {
        out.push_str("...");
    }
    out
}

fn channel_label(channel: ChannelKind) -> &'static str {
    match channel {
        ChannelKind::Rehearsal => "rehearsal",
        ChannelKind::WebEmbed => "web_embed",
        ChannelKind::LiveMeeting => "live_meeting",
        ChannelKind::Chat => "chat",
    }
}

fn provenance_source_label(source: ProvenanceSource) -> &'static str {
    match source {
        ProvenanceSource::CanonicalQa => "canonical_qa",
        ProvenanceSource::ApprovedMemory => "approved_memory",
        ProvenanceSource::Generative => "generative",
        ProvenanceSource::EscalationStub => "escalation_stub",
    }
}
