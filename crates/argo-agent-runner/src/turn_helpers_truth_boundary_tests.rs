#![allow(deprecated)]

use super::turn_helpers::run_turn_with_tools;
use super::turn_helpers_run::RoundOutcome;
use super::turn_helpers_truth_boundary::{
    apply_truth_boundary_gate, default_claim_provenance, parse_channel_kind, pick_refusal_variant,
    with_test_truth_boundary_ctx, TruthBoundaryTurnCtx,
};
use super::turn_skills_runtime::PendingAbOutcome;
use crate::helpers::SpendCtx;
use crate::turn_vertical::SessionGrantState;
use argo_audit::{rows_for_session, AuditSink};
use argo_capability::{
    ChannelKind, ChannelScope, ClaimClass, ClaimDisposition, ClaimProvenance, ProvenanceSource,
    RefusalTemplate, RefusalTemplateRegistry, TruthBoundaryPolicy,
};
use argo_provider::{ProviderRequest, ProviderResponse, StopReason, Transport};
use argo_store::approvals::list_pending;
use argo_store::SessionStore;
use argo_types::{ArgoResult, CapabilitySet, SessionId, TenantId};
use futures::StreamExt;
use std::sync::Arc;
use uuid::Uuid;

fn class(name: &str, disposition: ClaimDisposition, channels: ChannelScope) -> ClaimClass {
    ClaimClass {
        name: name.to_string(),
        disposition,
        channels,
        escalate_to: None,
    }
}

fn policy(
    tenant_id: TenantId,
    disposition: ClaimDisposition,
    channels: ChannelScope,
) -> TruthBoundaryPolicy {
    TruthBoundaryPolicy {
        id: "pitch-policy".to_string(),
        tenant_id,
        claim_classes: vec![class("revenue", disposition, channels)],
    }
}

fn registry() -> RefusalTemplateRegistry {
    RefusalTemplateRegistry::new(vec![RefusalTemplate {
        id: "defer-financials".to_string(),
        variants: vec![
            "I need to defer to Mark on financials.".to_string(),
            "I cannot share that financial claim here.".to_string(),
        ],
    }])
}

fn ctx(
    tenant_id: TenantId,
    disposition: ClaimDisposition,
    channel: ChannelKind,
    channels: ChannelScope,
) -> TruthBoundaryTurnCtx {
    TruthBoundaryTurnCtx::new(
        policy(tenant_id, disposition, channels),
        channel,
        registry(),
    )
}

fn outcome(text: &str, declared_classes: Vec<&str>) -> RoundOutcome {
    outcome_with_provenance(text, declared_classes, default_claim_provenance())
}

fn outcome_with_provenance(
    text: &str,
    declared_classes: Vec<&str>,
    claim_provenance: ClaimProvenance,
) -> RoundOutcome {
    RoundOutcome {
        assistant_id: Uuid::now_v7(),
        text: text.to_string(),
        declared_claim_classes: declared_classes.into_iter().map(str::to_string).collect(),
        claim_provenance,
        tokens_in: Some(5),
        tokens_out: Some(3),
        tool_calls: Vec::new(),
    }
}

async fn fresh_session() -> (Arc<SessionStore>, AuditSink, SessionId, TenantId) {
    let store = Arc::new(SessionStore::open(":memory:").await.expect("open store"));
    store.migrate().await.expect("migrate store");
    let (tenant, workspace) = store
        .ensure_workspace("truth-boundary", "/tmp/truth-boundary")
        .await
        .expect("workspace");
    let session_id = store
        .create_session(
            tenant,
            workspace,
            "mock-model",
            "mock-provider",
            Some("t"),
            None,
        )
        .await
        .expect("session");
    let audit = AuditSink::new(Arc::clone(&store));
    (store, audit, session_id, tenant)
}

async fn audit_kinds(
    store: &Arc<SessionStore>,
    audit: &AuditSink,
    session_id: SessionId,
) -> Vec<String> {
    audit.flush_pending().await.expect("flush audit");
    rows_for_session(store, session_id)
        .await
        .expect("audit rows")
        .into_iter()
        .map(|row| row.kind)
        .collect()
}

#[tokio::test]
async fn truth_boundary_allowed_claim_passes_with_audit() {
    let (store, audit, session_id, tenant) = fresh_session().await;
    let ctx = ctx(
        tenant,
        ClaimDisposition::Allow,
        ChannelKind::Chat,
        ChannelScope::AllChannels,
    );
    let result = apply_truth_boundary_gate(
        &ctx,
        &store,
        &audit,
        None,
        session_id,
        tenant,
        outcome("ARR is available internally.", vec!["revenue"]),
    )
    .await
    .expect("gate");

    assert_eq!(result.text, "ARR is available internally.");
    assert_eq!(
        audit_kinds(&store, &audit, session_id).await,
        vec!["claim.allowed"]
    );
}

#[tokio::test]
async fn truth_boundary_forbidden_claim_uses_refusal_and_audit() {
    let (store, audit, session_id, tenant) = fresh_session().await;
    let ctx = ctx(
        tenant,
        ClaimDisposition::Forbid {
            refusal_template_id: "defer-financials".to_string(),
        },
        ChannelKind::WebEmbed,
        ChannelScope::AllChannels,
    );
    let result = apply_truth_boundary_gate(
        &ctx,
        &store,
        &audit,
        None,
        session_id,
        tenant,
        outcome("ARR is $4.2M.", vec!["revenue"]),
    )
    .await
    .expect("gate");

    assert_ne!(result.text, "ARR is $4.2M.");
    assert!(result.text.contains("financial"));
    assert_eq!(
        audit_kinds(&store, &audit, session_id).await,
        vec!["claim.blocked"]
    );
}

#[tokio::test]
async fn truth_boundary_escalated_claim_enqueues_approval_and_audit() {
    let (store, audit, session_id, tenant) = fresh_session().await;
    let ctx = ctx(
        tenant,
        ClaimDisposition::Escalate,
        ChannelKind::LiveMeeting,
        ChannelScope::AllChannels,
    );
    let result = apply_truth_boundary_gate(
        &ctx,
        &store,
        &audit,
        None,
        session_id,
        tenant,
        outcome("ARR is $4.2M.", vec!["revenue"]),
    )
    .await
    .expect("gate");

    let pending = list_pending(store.pool(), tenant, 10)
        .await
        .expect("pending approvals");
    assert_eq!(pending.len(), 1);
    assert!(result.text.contains("Approval request"));
    assert_eq!(
        audit_kinds(&store, &audit, session_id).await,
        vec!["claim.escalated"]
    );
}

#[tokio::test]
async fn truth_boundary_cross_tenant_policy_does_not_gate() {
    let (store, audit, session_id, tenant) = fresh_session().await;
    let other_tenant = TenantId::new();
    let ctx = ctx(
        other_tenant,
        ClaimDisposition::Forbid {
            refusal_template_id: "defer-financials".to_string(),
        },
        ChannelKind::WebEmbed,
        ChannelScope::AllChannels,
    );
    let result = apply_truth_boundary_gate(
        &ctx,
        &store,
        &audit,
        None,
        session_id,
        tenant,
        outcome("ARR is $4.2M.", vec!["revenue"]),
    )
    .await
    .expect("gate");

    assert_eq!(result.text, "ARR is $4.2M.");
    assert!(audit_kinds(&store, &audit, session_id).await.is_empty());
}

#[tokio::test]
async fn truth_boundary_channel_scope_allows_rehearsal() {
    let (store, audit, session_id, tenant) = fresh_session().await;
    let ctx = ctx(
        tenant,
        ClaimDisposition::Forbid {
            refusal_template_id: "defer-financials".to_string(),
        },
        ChannelKind::Rehearsal,
        ChannelScope::Only(vec![ChannelKind::WebEmbed]),
    );
    let result = apply_truth_boundary_gate(
        &ctx,
        &store,
        &audit,
        None,
        session_id,
        tenant,
        outcome("ARR is $4.2M.", vec!["revenue"]),
    )
    .await
    .expect("gate");

    assert_eq!(result.text, "ARR is $4.2M.");
    assert_eq!(
        audit_kinds(&store, &audit, session_id).await,
        vec!["claim.allowed"]
    );
}

#[tokio::test]
async fn truth_boundary_channel_scope_forbids_web_embed() {
    let (store, audit, session_id, tenant) = fresh_session().await;
    let ctx = ctx(
        tenant,
        ClaimDisposition::Forbid {
            refusal_template_id: "defer-financials".to_string(),
        },
        ChannelKind::WebEmbed,
        ChannelScope::Only(vec![ChannelKind::WebEmbed]),
    );
    let result = apply_truth_boundary_gate(
        &ctx,
        &store,
        &audit,
        None,
        session_id,
        tenant,
        outcome("ARR is $4.2M.", vec!["revenue"]),
    )
    .await
    .expect("gate");

    assert!(result.text.contains("financial"));
    assert_eq!(
        audit_kinds(&store, &audit, session_id).await,
        vec!["claim.blocked"]
    );
}

#[tokio::test]
async fn truth_boundary_provenance_reaches_audit_payload() {
    let (store, audit, session_id, tenant) = fresh_session().await;
    let ctx = ctx(
        tenant,
        ClaimDisposition::Allow,
        ChannelKind::Chat,
        ChannelScope::AllChannels,
    );
    let provenance = ClaimProvenance {
        source: ProvenanceSource::CanonicalQa,
        source_id: "qa-revenue-v1".to_string(),
        source_excerpt: "approved answer".to_string(),
    };
    apply_truth_boundary_gate(
        &ctx,
        &store,
        &audit,
        None,
        session_id,
        tenant,
        outcome_with_provenance("Approved answer.", vec!["revenue"], provenance),
    )
    .await
    .expect("gate");

    audit.flush_pending().await.expect("flush audit");
    let rows = rows_for_session(&store, session_id)
        .await
        .expect("audit rows");
    assert_eq!(rows[0].payload["provenance"]["source"], "canonical_qa");
    assert_eq!(rows[0].payload["provenance"]["source_id"], "qa-revenue-v1");
}

#[tokio::test]
async fn truth_boundary_no_policy_short_circuits_without_audit() {
    let (store, audit, session_id, tenant) = fresh_session().await;
    let result = apply_truth_boundary_gate(
        &TruthBoundaryTurnCtx::disabled(),
        &store,
        &audit,
        None,
        session_id,
        tenant,
        outcome("ARR is $4.2M.", vec!["revenue"]),
    )
    .await
    .expect("gate");

    assert_eq!(result.text, "ARR is $4.2M.");
    assert!(audit_kinds(&store, &audit, session_id).await.is_empty());
}

#[tokio::test]
async fn truth_boundary_refusal_variant_is_deterministic() {
    let session_id = SessionId(Uuid::now_v7());
    let first =
        pick_refusal_variant(&registry(), "defer-financials", session_id).expect("first pick");
    let second =
        pick_refusal_variant(&registry(), "defer-financials", session_id).expect("second pick");

    assert_eq!(first, second);
}

#[tokio::test]
async fn truth_boundary_run_turn_with_tools_gates_final_once() {
    let (store, audit, session_id, tenant) = fresh_session().await;
    let ctx = ctx(
        tenant,
        ClaimDisposition::Allow,
        ChannelKind::Chat,
        ChannelScope::AllChannels,
    );
    let spend = SpendCtx {
        tenant_id: tenant,
        provider: "mock-provider",
        model: "mock-model",
        pricing: None,
    };
    let deny = std::collections::HashSet::new();
    let approval = std::collections::HashSet::new();
    let mut pending_ab: Vec<PendingAbOutcome> = Vec::new();
    let transport = StaticTextTransport::new("final answer");

    let result = with_test_truth_boundary_ctx(ctx, || async {
        run_turn_with_tools(
            &store,
            &transport,
            std::path::Path::new("/tmp/truth-boundary"),
            session_id,
            "mock-model",
            &audit,
            &spend,
            1,
            false,
            None,
            None,
            None,
            &deny,
            &approval,
            SessionGrantState::default(),
            None,
            "truth-boundary-test",
            &mut pending_ab,
        )
        .await
    })
    .await
    .expect("turn");

    assert_eq!(result.text, "final answer");
    assert_eq!(
        audit_kinds(&store, &audit, session_id).await,
        vec!["assistant_message", "claim.allowed"]
    );
}

#[test]
fn truth_boundary_channel_parser_accepts_manifest_spellings() {
    assert_eq!(
        parse_channel_kind("web-embed").expect("web embed"),
        ChannelKind::WebEmbed
    );
    assert_eq!(
        parse_channel_kind("live_meeting").expect("live meeting"),
        ChannelKind::LiveMeeting
    );
}

#[derive(Debug)]
struct StaticTextTransport {
    caps: CapabilitySet,
    text: String,
}

impl StaticTextTransport {
    fn new(text: &str) -> Self {
        Self {
            caps: CapabilitySet(Vec::new()),
            text: text.to_string(),
        }
    }
}

#[async_trait::async_trait]
impl Transport for StaticTextTransport {
    fn id(&self) -> &str {
        "static-text"
    }

    fn capabilities(&self) -> &CapabilitySet {
        &self.caps
    }

    async fn complete(&self, _req: &ProviderRequest) -> ArgoResult<ProviderResponse> {
        Ok(ProviderResponse {
            text: Some(self.text.clone()),
            reasoning: None,
            tool_calls: Vec::new(),
            stop_reason: StopReason::EndTurn,
            tokens_in: None,
            tokens_out: None,
            provider_meta: serde_json::Value::Null,
        })
    }

    async fn complete_stream(
        &self,
        _req: &ProviderRequest,
    ) -> ArgoResult<
        futures::stream::BoxStream<'static, ArgoResult<argo_provider::ProviderResponseDelta>>,
    > {
        Ok(futures::stream::empty().boxed())
    }
}
