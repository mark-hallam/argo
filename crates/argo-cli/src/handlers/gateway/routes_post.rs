//! POST handlers + heavier query helpers extracted from `routes.rs`
//! to keep that file under the 500-LOC budget. The dispatch table
//! still lives in `routes.rs`; these are the response builders it
//! delegates to.
use argo_store::SessionStore;
use argo_types::{MessageRole, SessionId};
use std::sync::Arc;

use super::{body_from_request, store_unavailable};

pub(super) async fn create_session_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let body = body_from_request(req);
    let parsed: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => {
            return (
                "400 Bad Request",
                format!("{{\"error\":\"parse body: {e}\"}}"),
            );
        }
    };
    let model = parsed["model"].as_str().unwrap_or("");
    let provider = parsed["provider"].as_str().unwrap_or("");
    if model.is_empty() || provider.is_empty() {
        return (
            "400 Bad Request",
            "{\"error\":\"required fields: model, provider\"}".to_string(),
        );
    }
    let title = parsed["title"].as_str();
    let cwd = match std::env::current_dir() {
        Ok(p) => p,
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"cwd: {e}\"}}"),
            );
        }
    };
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, workspace) = match store.ensure_workspace(name, &root).await {
        Ok(p) => p,
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"ensure_workspace: {e}\"}}"),
            );
        }
    };
    match store
        .create_session(tenant, workspace, model, provider, title)
        .await
    {
        Ok(id) => (
            "201 Created",
            serde_json::to_string(&serde_json::json!({
                "id": id.0.to_string(),
                "model": model,
                "provider": provider,
                "title": title,
            }))
            .unwrap_or_else(|_| "{}".to_string()),
        ),
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"create_session: {e}\"}}"),
        ),
    }
}

pub(super) async fn append_message_response(
    sid: &str,
    req: &str,
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
    let body = body_from_request(req);
    let parsed: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => {
            return (
                "400 Bad Request",
                format!("{{\"error\":\"parse body: {e}\"}}"),
            );
        }
    };
    let role = match parsed["role"].as_str() {
        Some("user") => MessageRole::User,
        Some("assistant") => MessageRole::Assistant,
        Some("system") => MessageRole::System,
        Some("tool") => MessageRole::Tool,
        Some(other) => {
            return (
                "400 Bad Request",
                format!("{{\"error\":\"unknown role '{other}': use user|assistant|system|tool\"}}"),
            );
        }
        None => {
            return (
                "400 Bad Request",
                "{\"error\":\"required field: role\"}".to_string(),
            );
        }
    };
    let content_json = match parsed.get("content") {
        Some(c) => c.to_string(),
        None => {
            return (
                "400 Bad Request",
                "{\"error\":\"required field: content (typed JSON value)\"}".to_string(),
            );
        }
    };
    match store.append_message(id, role, &content_json).await {
        Ok(msg_id) => (
            "201 Created",
            serde_json::to_string(&serde_json::json!({
                "id": msg_id.to_string(),
                "session_id": sid,
            }))
            .unwrap_or_else(|_| "{}".to_string()),
        ),
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"append_message: {e}\"}}"),
        ),
    }
}

pub(super) async fn run_turn_response(
    sid: &str,
    req: &str,
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
    let body = body_from_request(req);
    let parsed: serde_json::Value = if body.is_empty() {
        serde_json::Value::Null
    } else {
        match serde_json::from_str(body) {
            Ok(v) => v,
            Err(e) => {
                return (
                    "400 Bad Request",
                    format!("{{\"error\":\"parse body: {e}\"}}"),
                );
            }
        }
    };
    let user = parsed.get("user").and_then(|v| v.as_str());
    let max_rounds = parsed
        .get("max_tool_rounds")
        .and_then(serde_json::Value::as_u64)
        .and_then(|n| usize::try_from(n).ok())
        .unwrap_or(crate::handlers::chat::MAX_TOOL_ROUNDS);
    // The gateway always streams text to its own stdout (visible
    // when running `argo gateway start` in foreground); the
    // response body is still the final JSON. SSE / chunked
    // streaming over HTTP is its own follow-up.
    let stream = parsed
        .get("stream")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let cwd = match std::env::current_dir() {
        Ok(p) => p,
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"cwd: {e}\"}}"),
            );
        }
    };
    match crate::handlers::chat::run_turn(store, &cwd, id, user, stream, max_rounds).await {
        Ok(result) => (
            "201 Created",
            serde_json::to_string(&serde_json::json!({
                "assistant_id": result.assistant_id.to_string(),
                "text": result.text,
                "tokens_in": result.tokens_in,
                "tokens_out": result.tokens_out,
            }))
            .unwrap_or_else(|_| "{}".to_string()),
        ),
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"run_turn: {e}\"}}"),
        ),
    }
}

pub(super) async fn replay_response(
    sid: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    if store.is_none() {
        return store_unavailable();
    }
    let body = body_from_request(req);
    let parsed: serde_json::Value = if body.is_empty() {
        serde_json::Value::Null
    } else {
        match serde_json::from_str(body) {
            Ok(v) => v,
            Err(e) => {
                return (
                    "400 Bad Request",
                    format!("{{\"error\":\"parse body: {e}\"}}"),
                );
            }
        }
    };
    let model = parsed.get("model").and_then(|v| v.as_str());
    let provider = parsed.get("provider").and_then(|v| v.as_str());
    match crate::handlers::session_replay::replay_core(sid, model, provider).await {
        Ok(r) => (
            "201 Created",
            serde_json::to_string(&serde_json::json!({
                "source_id": r.source_id.0.to_string(),
                "new_id": r.new_id.0.to_string(),
                "provider": r.provider,
                "model": r.model,
                "user_message_count": r.user_message_count,
                "final_text": r.final_text,
            }))
            .unwrap_or_else(|_| "{}".to_string()),
        ),
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"replay: {e}\"}}"),
        ),
    }
}

pub(super) async fn steer_response(
    sid: &str,
    req: &str,
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
    let body = body_from_request(req);
    let parsed: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => {
            return (
                "400 Bad Request",
                format!("{{\"error\":\"parse body: {e}\"}}"),
            );
        }
    };
    let Some(text) = parsed.get("text").and_then(|v| v.as_str()) else {
        return (
            "400 Bad Request",
            "{\"error\":\"missing 'text' field\"}".to_string(),
        );
    };
    if let Ok(None) = store.get_session(id).await {
        return ("404 Not Found", format!("{{\"error\":\"no session {sid}\"}}"));
    }
    let content = serde_json::json!([{
        "type": "text",
        "text": format!("[steer] {text}"),
    }]);
    match store
        .append_message(id, MessageRole::System, &content.to_string())
        .await
    {
        Ok(msg_id) => (
            "201 Created",
            format!("{{\"message_id\":\"{msg_id}\"}}"),
        ),
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"steer: {e}\"}}"),
        ),
    }
}

pub(super) async fn costs_breakdown_response(
    mode: &str,
    store: Option<&Arc<SessionStore>>,
    format: Option<&str>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let sql = match mode {
        "by_model" => {
            "SELECT provider || ' / ' || model AS bucket, \
             COALESCE(SUM(amount_usd_micros), 0) AS total \
             FROM spend_ledger GROUP BY provider, model ORDER BY total DESC LIMIT 50"
        }
        "by_session" => {
            "SELECT lower(hex(session_id)) AS bucket, \
             COALESCE(SUM(amount_usd_micros), 0) AS total \
             FROM spend_ledger GROUP BY session_id ORDER BY total DESC LIMIT 50"
        }
        "by_provider" => {
            "SELECT provider AS bucket, \
             COALESCE(SUM(amount_usd_micros), 0) AS total \
             FROM spend_ledger GROUP BY provider ORDER BY total DESC LIMIT 50"
        }
        _ => unreachable!(),
    };
    match sqlx::query_as::<_, (String, i64)>(sql)
        .fetch_all(store.pool())
        .await
    {
        Ok(rows) => {
            if matches!(format, Some("markdown")) {
                let title = match mode {
                    "by_model" => "Spend by model",
                    "by_session" => "Spend by session",
                    "by_provider" => "Spend by provider",
                    _ => "Spend",
                };
                let n = rows.len();
                let mut out = format!("# {title} ({n} bucket{})\n\n", if n == 1 { "" } else { "s" });
                if !rows.is_empty() {
                    out.push_str("| bucket | total (USD) |\n");
                    out.push_str("| --- | ---: |\n");
                    for (b, t) in &rows {
                        out.push_str(&format!(
                            "| `{b}` | ${:.4} |\n",
                            *t as f64 / 1_000_000.0
                        ));
                    }
                }
                return ("200 OK", out);
            }
            let arr: Vec<serde_json::Value> = rows
                .into_iter()
                .map(|(b, t)| {
                    serde_json::json!({
                        "bucket": b,
                        "spent_usd_micros": u64::try_from(t).unwrap_or(0),
                    })
                })
                .collect();
            (
                "200 OK",
                serde_json::to_string(&serde_json::json!({mode: arr}))
                    .unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"costs breakdown: {e}\"}}"),
        ),
    }
}

/// `GET /sessions/<id>/judge[?format=markdown]` — list
/// judge_record rows scoped to one session. Mirrors
/// `argo judge verdict <id>`. Returns 200 + a JSON array
/// (possibly empty) or markdown table; 400 on bad uuid.
pub(super) async fn session_judge_response(
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
    match store.list_judge_records(id).await {
        Ok(rows) => {
            if matches!(format, Some("markdown")) {
                let n = rows.len();
                let mut out = format!(
                    "# Judge verdicts for session `{sid}` ({n} record{})\n\n",
                    if n == 1 { "" } else { "s" }
                );
                if rows.is_empty() {
                    out.push_str("_No judge records yet._\n");
                } else {
                    out.push_str("| subject | verdict | judge | reason |\n");
                    out.push_str("| --- | :---: | --- | --- |\n");
                    for r in &rows {
                        let reason = r
                            .reason
                            .as_deref()
                            .unwrap_or("—")
                            .replace('|', "\\|");
                        out.push_str(&format!(
                            "| `{}/{}` | `{}` | `{}/{}` | {reason} |\n",
                            r.subject_kind,
                            r.subject_ref,
                            r.verdict,
                            r.judge_provider,
                            r.judge_model,
                        ));
                    }
                }
                return ("200 OK", out);
            }
            (
                "200 OK",
                serde_json::to_string(&rows).unwrap_or_else(|_| "[]".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"list_judge_records: {e}\"}}"),
        ),
    }
}


