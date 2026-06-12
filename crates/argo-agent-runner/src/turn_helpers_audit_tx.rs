//! Transactional turn mutation + audit choreography for the per-round
//! dispatch loop.
//!
//! The runner persists two kinds of per-round message rows — the
//! assistant message and the tool-result message — each paired with a
//! hash-chained `audit_record` row. Before `fix-runner-audit-
//! transactional` those were SEPARATE awaits (`store.append_message`
//! then `audit.write`), so a crash/error between them could leave a
//! message mutation that escaped the tamper-evident audit chain (or an
//! audit row with no mutation). The hash-chained audit trail is a core
//! product differentiator; an un-audited mutation is an integrity hole
//! (eval finding 3.1, `eval_findings_audit_chain.rs`).
//!
//! Each helper here opens ONE sqlx transaction, performs the message
//! INSERT (via [`argo_store::SessionStore::append_message_in_tx`]) and
//! the audit INSERT (via [`argo_audit::AuditSink::write_in_tx`]) inside
//! it, commits both together, then runs the best-effort post-commit
//! fan-out. If either insert fails the tx is dropped before commit and
//! both roll back atomically. This mirrors the #2078 workflow-runner
//! `run_transactional_audit` shape rather than inventing a parallel tx
//! helper.

use argo_audit::{AuditKind, AuditSink};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, MessageRole, SessionId};
use std::sync::Arc;
use uuid::Uuid;

/// Persist the assistant message + its `AssistantMessage` audit row in
/// one transaction. The audit payload carries `message_id`, so the
/// message INSERT must happen first (inside the same tx) to obtain the
/// id. Returns `(assistant_id, payload)` — the caller emits the live
/// `RunEventSink` event from the returned payload AFTER the tx commits.
#[allow(clippy::too_many_arguments)]
pub(super) async fn persist_assistant_round(
    store: &Arc<SessionStore>,
    audit: &AuditSink,
    session_id: SessionId,
    assistant_content: &serde_json::Value,
    round: usize,
    cost_usd: Option<f64>,
    tokens_in: Option<u32>,
    tokens_out: Option<u32>,
    tool_calls_len: usize,
) -> ArgoResult<(Uuid, serde_json::Value)> {
    let mut tx = store
        .pool()
        .begin()
        .await
        .map_err(|e| ArgoError::Provider(format!("assistant audit tx begin: {e}")))?;
    let assistant_id = store
        .append_message_in_tx(
            &mut tx,
            session_id,
            MessageRole::Assistant,
            &assistant_content.to_string(),
        )
        .await?;
    // UF A7 — stamp the turn's USD cost onto the audit row so the SPA
    // activity feed (`ActivityRow.tsx`) renders a cost chip. `None`
    // (mock provider / no pricing) emits a JSON `null`, never a
    // fabricated zero (NO THEATER).
    let payload = serde_json::json!({
        "message_id": assistant_id,
        "round": round,
        "tool_calls": tool_calls_len,
        "tokens_in": tokens_in,
        "tokens_out": tokens_out,
        "cost_usd": cost_usd,
    });
    let post = audit
        .write_in_tx(
            &mut tx,
            Some(session_id),
            AuditKind::AssistantMessage,
            payload.clone(),
            None,
        )
        .await?;
    tx.commit()
        .await
        .map_err(|e| ArgoError::Provider(format!("assistant audit tx commit: {e}")))?;
    audit.after_commit_fanout(post).await.ok();
    Ok((assistant_id, payload))
}

/// Persist the `ToolResult` audit row + the tool-result message in one
/// transaction. The audit fires first (matching the historical emit
/// order), then the message INSERT; both commit together or roll back
/// together. `result_payload` is consumed; the caller keeps its own
/// clone for the post-commit `RunEventSink` event.
pub(super) async fn persist_tool_result(
    store: &Arc<SessionStore>,
    audit: &AuditSink,
    session_id: SessionId,
    result_payload: serde_json::Value,
    tool_msg: &serde_json::Value,
) -> ArgoResult<()> {
    let mut tx = store
        .pool()
        .begin()
        .await
        .map_err(|e| ArgoError::Provider(format!("tool-result audit tx begin: {e}")))?;
    let post = audit
        .write_in_tx(
            &mut tx,
            Some(session_id),
            AuditKind::ToolResult,
            result_payload,
            None,
        )
        .await?;
    store
        .append_message_in_tx(&mut tx, session_id, MessageRole::Tool, &tool_msg.to_string())
        .await?;
    tx.commit()
        .await
        .map_err(|e| ArgoError::Provider(format!("tool-result audit tx commit: {e}")))?;
    audit.after_commit_fanout(post).await.ok();
    Ok(())
}
