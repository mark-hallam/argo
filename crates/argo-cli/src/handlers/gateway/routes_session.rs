//! Session-detail GET endpoints: per-session header / spend ledger.
//! Extracted from `routes.rs` to keep that file under the
//! 500-LOC budget.
use argo_store::SessionStore;
use argo_types::SessionId;
use std::sync::Arc;

use super::store_unavailable;

/// `GET /sessions/<id>/spend/total` — single-number total
/// spend for one session, in USD micros. Convenient for
/// dashboards that only want the figure (no ledger rows).
pub(super) async fn session_spend_total_response(
    sid: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let id = match uuid::Uuid::parse_str(sid) {
        Ok(u) => SessionId(u),
        Err(e) => {
            return (
                "400 Bad Request",
                format!("{{\"error\":\"invalid session id: {e}\"}}"),
            );
        }
    };
    let total = super::session_spend_micros(store, id).await.unwrap_or(0);
    (
        "200 OK",
        format!(
            "{{\"session_id\":\"{sid}\",\"total_usd_micros\":{total}}}"
        ),
    )
}

pub(super) async fn session_spend_response(
    sid: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let id = match uuid::Uuid::parse_str(sid) {
        Ok(u) => SessionId(u),
        Err(e) => {
            return (
                "400 Bad Request",
                format!("{{\"error\":\"invalid session id: {e}\"}}"),
            );
        }
    };
    let total = super::session_spend_micros(store, id).await.unwrap_or(0);
    type Row = (i64, i64, String, String);
    let rows: Vec<Row> = match sqlx::query_as(
        "SELECT charged_at_micros, amount_usd_micros, provider, model FROM spend_ledger \
         WHERE session_id = ? ORDER BY charged_at_micros ASC",
    )
    .bind(id.0.as_bytes().as_slice())
    .fetch_all(store.pool())
    .await
    {
        Ok(r) => r,
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"session spend rows: {e}\"}}"),
            );
        }
    };
    let entries: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|(ts, micros, provider, model)| {
            serde_json::json!({
                "charged_at_micros": ts,
                "amount_usd_micros": u64::try_from(micros).unwrap_or(0),
                "provider": provider,
                "model": model,
            })
        })
        .collect();
    let body = serde_json::json!({
        "session_id": sid,
        "total_usd_micros": total,
        "rows": entries,
    });
    (
        "200 OK",
        serde_json::to_string(&body).unwrap_or_else(|_| "{}".to_string()),
    )
}

/// `GET /sessions/<id>[?format=markdown]` — header view.
/// Default JSON shape: `{id, model, provider, title,
/// started_at, ended_at, elapsed_seconds, message_count,
/// tool_call_count, spent_usd_micros, …}`. With
/// `?format=markdown`, returns a paste-able header doc with
/// the same fields rendered as markdown bullets.
pub(super) async fn session_show_response_with_format(
    sid: &str,
    store: Option<&Arc<SessionStore>>,
    format: Option<&str>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let id = match uuid::Uuid::parse_str(sid) {
        Ok(u) => SessionId(u),
        Err(e) => {
            return (
                "400 Bad Request",
                format!("{{\"error\":\"invalid session id: {e}\"}}"),
            );
        }
    };
    let session = match store.get_session(id).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            return (
                "404 Not Found",
                format!("{{\"error\":\"no session {sid}\"}}"),
            );
        }
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"get_session: {e}\"}}"),
            );
        }
    };
    let count = store.message_count(id).await.unwrap_or(0);
    let tool_calls = super::session_tool_call_count(store, id).await.unwrap_or(0);
    let spent = super::session_spend_micros(store, id).await.unwrap_or(0);
    let now = chrono::Utc::now();
    let end = session.ended_at.unwrap_or(now);
    let elapsed_seconds = (end - session.started_at).num_seconds().max(0);
    if matches!(format, Some("markdown")) {
        let title = session
            .title
            .as_deref()
            .filter(|t| !t.is_empty())
            .unwrap_or("(untitled)");
        let mut out = format!("# Session {title} (`{}`)\n\n", session.id.0);
        out.push_str(&format!("- **Provider**: `{}`\n", session.provider));
        out.push_str(&format!("- **Model**: `{}`\n", session.model));
        out.push_str(&format!(
            "- **Started**: {}\n",
            session.started_at.to_rfc3339()
        ));
        if let Some(end) = session.ended_at {
            out.push_str(&format!("- **Ended**: {}\n", end.to_rfc3339()));
        }
        out.push_str(&format!("- **Elapsed**: {elapsed_seconds}s\n"));
        out.push_str(&format!("- **Messages**: {count}\n"));
        out.push_str(&format!("- **Tool calls**: {tool_calls}\n"));
        out.push_str(&format!(
            "- **Spend**: ${:.4}\n",
            spent as f64 / 1_000_000.0
        ));
        return ("200 OK", out);
    }
    let row = serde_json::json!({
        "id": session.id.0.to_string(),
        "tenant_id": session.tenant_id.0.to_string(),
        "workspace_id": session.workspace_id.0.to_string(),
        "model": session.model,
        "provider": session.provider,
        "title": session.title,
        "started_at": session.started_at.to_rfc3339(),
        "ended_at": session.ended_at.map(|d| d.to_rfc3339()),
        "elapsed_seconds": elapsed_seconds,
        "message_count": count,
        "tool_call_count": tool_calls,
        "spent_usd_micros": spent,
    });
    (
        "200 OK",
        serde_json::to_string(&row).unwrap_or_else(|_| "{}".to_string()),
    )
}
