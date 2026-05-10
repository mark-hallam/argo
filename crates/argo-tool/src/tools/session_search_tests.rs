//! Unit coverage for `session_search` (split out under the
//! 200-LOC budget called out in the brief). The integration test
//! that goes through the registry's dispatch + capability seam
//! lives in `tests/session_search_integration.rs`.
use super::*;
use argo_types::{MessageRole, Principal, TenantId, WorkspaceId};
use std::path::PathBuf;

async fn seeded_store() -> (Arc<SessionStore>, argo_types::SessionId) {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let tenant = TenantId::new();
    let ws = WorkspaceId::new();
    sqlx::query(
        "INSERT INTO workspace (id, tenant_id, name, root_path, created_at)
         VALUES (?, ?, 'test', '.', strftime('%s','now'))",
    )
    .bind(ws.0.as_bytes().as_slice())
    .bind(tenant.0.as_bytes().as_slice())
    .execute(store.pool())
    .await
    .unwrap();
    let session = store
        .create_session(tenant, ws, "claude-haiku", "anthropic", Some("seed"), None)
        .await
        .unwrap();
    (Arc::new(store), session)
}

fn ctx() -> ToolContext {
    ToolContext::new(
        Principal::Local {
            os_user: "u".into(),
        },
        PathBuf::from("/tmp"),
    )
}

fn body_text(r: &argo_types::ToolCallResult) -> String {
    match &r.content[0] {
        argo_types::ContentBlock::Text { text } => text.clone(),
        _ => panic!("expected text"),
    }
}

#[tokio::test]
async fn default_returns_not_wired_error() {
    let tool = SessionSearch::default();
    let r = tool
        .dispatch(serde_json::json!({"query": "anything"}), &ctx())
        .await
        .unwrap();
    assert!(r.is_error);
    let text = body_text(&r);
    assert!(text.contains("not wired"), "got: {text}");
}

#[tokio::test]
async fn empty_result_returns_empty_hits_array() {
    let (store, _) = seeded_store().await;
    let tool = SessionSearch::new(store);
    let r = tool
        .dispatch(serde_json::json!({"query": "nonexistentterm"}), &ctx())
        .await
        .unwrap();
    assert!(!r.is_error);
    let v: serde_json::Value = serde_json::from_str(&body_text(&r)).unwrap();
    assert_eq!(v["hits"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn returns_hits_when_messages_match() {
    let (store, session) = seeded_store().await;
    store
        .append_message(
            session,
            MessageRole::User,
            r#"[{"type":"text","text":"can you migrate to postgres?"}]"#,
        )
        .await
        .unwrap();
    store
        .append_message(
            session,
            MessageRole::Assistant,
            r#"[{"type":"text","text":"sure, here is the migration plan"}]"#,
        )
        .await
        .unwrap();
    let tool = SessionSearch::new(store);
    let r = tool
        .dispatch(serde_json::json!({"query": "migration"}), &ctx())
        .await
        .unwrap();
    assert!(!r.is_error, "tool errored: {:?}", r.content);
    let v: serde_json::Value = serde_json::from_str(&body_text(&r)).unwrap();
    let hits = v["hits"].as_array().unwrap();
    assert!(!hits.is_empty(), "expected at least one hit");
    assert!(hits[0]["snippet"]
        .as_str()
        .unwrap()
        .to_lowercase()
        .contains("migration"));
    assert_eq!(hits[0]["session_id"], session.0.to_string());
}

#[tokio::test]
async fn since_minutes_filters_old_hits_out() {
    let (store, session) = seeded_store().await;
    // Append one matching message, then back-date its created_at
    // far enough that any positive `since_minutes` excludes it.
    store
        .append_message(
            session,
            MessageRole::User,
            r#"[{"type":"text","text":"old migration plan"}]"#,
        )
        .await
        .unwrap();
    let stale_micros = Utc::now().timestamp_micros() - 60 * 60 * 1_000_000;
    sqlx::query("UPDATE message SET created_at = ? WHERE session_id = ?")
        .bind(stale_micros)
        .bind(session.0.as_bytes().as_slice())
        .execute(store.pool())
        .await
        .unwrap();

    let tool = SessionSearch::new(store);

    // No filter: hit comes through.
    let r = tool
        .dispatch(serde_json::json!({"query": "migration"}), &ctx())
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_str(&body_text(&r)).unwrap();
    assert_eq!(v["hits"].as_array().unwrap().len(), 1);

    // 5-minute window: excludes the back-dated row.
    let r = tool
        .dispatch(
            serde_json::json!({"query": "migration", "since_minutes": 5}),
            &ctx(),
        )
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_str(&body_text(&r)).unwrap();
    assert_eq!(v["hits"].as_array().unwrap().len(), 0);
}

#[test]
fn schema_exposes_required_query_field() {
    let t = SessionSearch::default();
    let params = t.parameters();
    let required = params["required"].as_array().unwrap();
    assert!(required.iter().any(|v| v == "query"));
}
