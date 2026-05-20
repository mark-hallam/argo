use crate::handlers::worker_actions::ActionRecord;
use argo_audit::{AuditKind, AuditSink};
use argo_channels::Notification;
use chrono::Utc;
use serde_json::json;

/// Best-effort `AuditKind::ApprovalAsked` write. Failures are
/// logged via `tracing::warn!` and never propagate; audit emit is
/// observability, not a delivery-path correctness gate.
pub(crate) async fn emit_approval_asked(
    audit: &AuditSink,
    action: &ActionRecord,
    notification: &Notification,
    channel: &str,
    target: &str,
) {
    let payload = json!({
        "action_id": action.action_id,
        "worker_id": action.worker_id.0.to_string(),
        "channel": channel,
        "target": target,
        "risk_level": action.risk_level,
        "requested_at": Utc::now().to_rfc3339(),
        "expiry_ts": notification.expiry.to_rfc3339(),
    });
    if let Err(e) = audit
        .write(None, AuditKind::ApprovalAsked, payload, None)
        .await
    {
        tracing::warn!(
            action_id = %action.action_id,
            channel = %channel,
            error = %e,
            "approval.asked audit write failed; continuing",
        );
    }
}
