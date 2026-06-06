//! Hire audit-row emission, split out of `routes_workers_hire` to keep
//! that handler under the 500-LOC ceiling (HARD RULE 6). Writes the
//! structural `HttpRequest` row plus the named `WorkerHired` domain row,
//! and — when the SPA tags a Genesis fallback — a `GenesisFallbackUsed`
//! row. Behavior is byte-for-byte the pre-split emission; only the
//! module home changed.

use super::routes_workers_hire_gate::{redact_text, redact_value};
use argo_audit::{AuditKind, AuditSink};
use argo_identity::{HireDraft, WorkerId};
use argo_store::SessionStore;
use argo_types::TenantId;
use chrono::Utc;
use serde_json::{json, Value};

#[allow(clippy::too_many_arguments)]
pub(super) async fn emit_hire_audit(
    store: &std::sync::Arc<SessionStore>,
    tenant_id: TenantId,
    principal_id: &str,
    worker_id: WorkerId,
    problem: &str,
    answers: &Value,
    draft: &HireDraft,
    genesis_fallback: bool,
) {
    let payload = json!({
        "endpoint": "workers.hire",
        "worker_id": worker_id.to_string(),
        "vertical_id": draft.suggested_vertical,
        "persona_name": draft.suggested_name,
        "approval_policy_level": draft.approval_policy.level,
        "duty_count": draft.duties.len(),
        "problem": redact_text(problem),
        "answers": redact_value(answers),
        "draft": json!({
            "suggested_name": draft.suggested_name,
            "suggested_vertical": draft.suggested_vertical,
            "charter_len": draft.charter.len(),
            "duty_entry_actions": draft
                .duties
                .iter()
                .map(|d| d.entry_action.clone())
                .collect::<Vec<_>>(),
        }),
    });
    let sink = AuditSink::new_with_tenant(std::sync::Arc::clone(store), tenant_id.to_string());
    if let Err(e) = sink
        .write(
            None,
            AuditKind::HttpRequest,
            payload,
            Some(principal_id.to_string()),
        )
        .await
    {
        tracing::warn!(
            target: "gateway::workers_hire",
            error = %e,
            worker_id = %worker_id,
            "hire audit write failed"
        );
    }
    // Domain event (stream `web-audit-kinds-for-journey-events`):
    // alongside the structural `HttpRequest` row above, emit a
    // named `WorkerHired` row so audit consumers scanning for
    // "who joined the workspace today?" see the hire directly
    // instead of `HttpRequest + endpoint=workers.hire`.
    let domain_payload = json!({
        "actor": principal_id,
        "tenant_id": tenant_id.to_string(),
        "worker_id": worker_id.to_string(),
        "vertical_id": draft.suggested_vertical,
        "name": draft.suggested_name,
        "duty_count": draft.duties.len(),
        "hired_at": Utc::now().to_rfc3339(),
    });
    if let Err(e) = sink
        .write(
            None,
            AuditKind::WorkerHired,
            domain_payload,
            Some(principal_id.to_string()),
        )
        .await
    {
        tracing::warn!(
            target: "gateway::workers_hire",
            error = %e,
            worker_id = %worker_id,
            "worker_hired audit write failed"
        );
    }
    // Stream `mission-genesis-fallback-audit-kind`: when the SPA tags
    // the hire body with `"genesis_fallback": true`, the Step 3
    // Genesis call fell back to the local stub draft. Emit a named
    // row so operators can see Genesis-failure events directly in
    // `/audit` without filtering on the generic HttpRequest payload.
    if genesis_fallback {
        let fallback_payload = json!({
            "actor": principal_id,
            "tenant_id": tenant_id.to_string(),
            "worker_id": worker_id.to_string(),
            "vertical_id": draft.suggested_vertical,
        });
        if let Err(e) = sink
            .write(
                None,
                AuditKind::GenesisFallbackUsed,
                fallback_payload,
                Some(principal_id.to_string()),
            )
            .await
        {
            tracing::warn!(
                target: "gateway::workers_hire",
                error = %e,
                worker_id = %worker_id,
                "genesis_fallback_used audit write failed"
            );
        }
    }
}
