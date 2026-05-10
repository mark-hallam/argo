//! Integration coverage for `session_search`: end-to-end seed → query
//! through the [`ToolRegistry`] dispatch path. Verifies capability
//! gating (`Capability::ToolDispatch`) AND the actual FTS5 ranking on
//! a seeded in-memory store, so the tool's wire shape matches the
//! agent's expected `{ hits: [...] }` payload.

use argo_capability::CapabilityEngine;
use argo_store::SessionStore;
use argo_tool::{SessionSearch, ToolContext, ToolRegistry};
use argo_types::{
    Capability, ContentBlock, MessageRole, Principal, SessionId, TenantId, WorkspaceId,
};
use std::sync::Arc;

fn principal() -> Principal {
    Principal::Local {
        os_user: "session-search-it".into(),
    }
}

fn engine_for_principal(grants: &[Capability]) -> CapabilityEngine {
    let engine = CapabilityEngine::with_defaults();
    engine.bind(principal(), "guest".into()).unwrap();
    for grant in grants {
        engine.grant_dynamic(principal(), grant.clone());
    }
    engine
}

async fn seeded_three_session_store() -> (Arc<SessionStore>, Vec<SessionId>) {
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

    let mut sessions = Vec::new();
    for (title, body) in [
        ("alpha", "shipping the postgres migration tonight"),
        ("beta", "user asked about the postgres rollback plan"),
        ("gamma", "totally unrelated discussion of llm prompt caching"),
    ] {
        let s = store
            .create_session(tenant, ws, "claude-haiku", "anthropic", Some(title), None)
            .await
            .unwrap();
        store
            .append_message(
                s,
                MessageRole::User,
                &format!(r#"[{{"type":"text","text":{:?}}}]"#, body),
            )
            .await
            .unwrap();
        sessions.push(s);
    }
    (Arc::new(store), sessions)
}

#[tokio::test]
async fn dispatch_returns_postgres_hits_in_rank_order() {
    let (store, sessions) = seeded_three_session_store().await;
    let mut reg = ToolRegistry::new();
    reg.register(Arc::new(SessionSearch::new(store)));

    let engine = engine_for_principal(&[Capability::ToolDispatch {
        tool_name_glob: "session_search".into(),
    }]);
    let ctx = ToolContext::new(principal(), std::path::PathBuf::from("/tmp"));

    let result = reg
        .dispatch(
            "session_search",
            serde_json::json!({"query": "postgres"}),
            &ctx,
            &engine,
        )
        .await
        .unwrap();
    assert!(!result.is_error, "tool errored: {:?}", result.content);

    let body = match &result.content[0] {
        ContentBlock::Text { text } => text.clone(),
        _ => panic!("expected text block"),
    };
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    let hits = v["hits"].as_array().unwrap();
    assert_eq!(hits.len(), 2, "expected exactly two postgres hits");

    // Both hits should belong to the seeded `alpha` / `beta` sessions
    // (gamma never mentions postgres). Order between alpha/beta is
    // FTS5-rank dependent; what matters is that gamma is excluded.
    let session_ids: Vec<String> = hits
        .iter()
        .map(|h| h["session_id"].as_str().unwrap().to_string())
        .collect();
    assert!(session_ids.iter().any(|s| s == &sessions[0].0.to_string()));
    assert!(session_ids.iter().any(|s| s == &sessions[1].0.to_string()));
    assert!(!session_ids.iter().any(|s| s == &sessions[2].0.to_string()));
    for h in hits {
        assert!(h["snippet"]
            .as_str()
            .unwrap()
            .to_lowercase()
            .contains("postgres"));
        assert!(h["ts_micros"].as_i64().unwrap() > 0);
    }
}

#[tokio::test]
async fn dispatch_denied_without_tool_dispatch_capability() {
    let (store, _) = seeded_three_session_store().await;
    let mut reg = ToolRegistry::new();
    reg.register(Arc::new(SessionSearch::new(store)));

    let engine = engine_for_principal(&[]);
    let ctx = ToolContext::new(principal(), std::path::PathBuf::from("/tmp"));

    let err = reg
        .dispatch(
            "session_search",
            serde_json::json!({"query": "postgres"}),
            &ctx,
            &engine,
        )
        .await
        .unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.to_lowercase().contains("capabil") || msg.to_lowercase().contains("denied"),
        "expected capability denial, got: {msg}"
    );
}
