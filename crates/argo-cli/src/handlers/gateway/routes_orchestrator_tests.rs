//! Tests for orchestrator gateway routes — split from `routes_orchestrator.rs`
//! for the LOC budget. `#[path]`-mounted into the parent's `mod tests`.

use super::*;
use argo_identity::WorkerId;
use argo_orchestrator::{EscalationStep, Handoff};

static ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn req(method: &str, target: &str, token: Option<&str>, body: &str) -> String {
    let auth = token
        .map(|t| format!("Authorization: Bearer {t}\r\n"))
        .unwrap_or_default();
    format!("{method} {target} HTTP/1.1\r\n{auth}\r\n{body}")
}

async fn store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    Arc::new(store)
}

#[tokio::test]
async fn orchestrator_routes_auth_and_lifecycle() {
    let _guard = ENV_LOCK.lock().await;
    std::env::set_var("ARGO_WEBINBOX_BEARER_TOKEN", "tok");
    std::env::set_var(
        "ARGO_WEBINBOX_TENANT_ID",
        "019aff00-0000-7000-8000-000000000101",
    );
    let store = store().await;
    let tenant = "019aff00-0000-7000-8000-000000000101";
    let other = "019aff00-0000-7000-8000-000000000102";
    assert_eq!(
        dispatch_route(
            &req("GET", &format!("/sessions?tenant={tenant}"), None, ""),
            Some(&store)
        )
        .await
        .unwrap()
        .0,
        "401 Unauthorized"
    );
    assert_eq!(
        dispatch_route(
            &req(
                "GET",
                &format!("/sessions?tenant={tenant}"),
                Some("bad"),
                ""
            ),
            Some(&store)
        )
        .await
        .unwrap()
        .0,
        "401 Unauthorized"
    );
    assert_eq!(
        dispatch_route(
            &req("GET", &format!("/sessions?tenant={other}"), Some("tok"), ""),
            Some(&store)
        )
        .await
        .unwrap()
        .0,
        "403 Forbidden"
    );
    assert_eq!(
        dispatch_route(
            &req(
                "GET",
                &format!("/sessions?tenant={tenant}&oops=1"),
                Some("tok"),
                ""
            ),
            Some(&store)
        )
        .await
        .unwrap()
        .0,
        "400 Bad Request"
    );
    let (status, body) = dispatch_route(
        &req(
            "POST",
            "/sessions",
            Some("tok"),
            &json!({"tenant_id":tenant,"title":"demo"}).to_string(),
        ),
        Some(&store),
    )
    .await
    .unwrap();
    assert_eq!(status, "201 Created");
    let sid = serde_json::from_str::<Value>(&body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    let repo = OrchestratorStore::new(Arc::clone(&store));
    let from = WorkerId::new();
    let to = WorkerId::new();
    let h = Handoff::new(
        argo_orchestrator::SessionId::from_string(sid.clone()),
        from,
        EscalationStep::Worker(to),
        "finish",
        json!({"k":1}),
    );
    repo.insert_handoff(TenantId(uuid::Uuid::parse_str(tenant).unwrap()), &h)
        .await
        .unwrap();
    for target in [
        format!("/sessions?tenant={tenant}&status=active"),
        format!("/sessions/{sid}?tenant={tenant}"),
        format!("/handoffs?tenant={tenant}&session={sid}"),
        format!("/handoffs/{}?tenant={tenant}", h.id),
    ] {
        assert_eq!(
            dispatch_route(&req("GET", &target, Some("tok"), ""), Some(&store))
                .await
                .unwrap()
                .0,
            "200 OK"
        );
    }
    assert_eq!(
        dispatch_route(
            &req(
                "PATCH",
                &format!("/handoffs/{}/status?tenant={tenant}", h.id),
                Some("tok"),
                r#"{"status":"acknowledged"}"#
            ),
            Some(&store)
        )
        .await
        .unwrap()
        .0,
        "200 OK"
    );
    assert_eq!(
        dispatch_route(
            &req(
                "PATCH",
                &format!("/sessions/{sid}/status?tenant={tenant}"),
                Some("tok"),
                r#"{"status":"resolved"}"#
            ),
            Some(&store)
        )
        .await
        .unwrap()
        .0,
        "200 OK"
    );
    assert_eq!(
        dispatch_route(
            &req(
                "PATCH",
                &format!("/sessions/{sid}/status?tenant={tenant}"),
                Some("tok"),
                r#"{"status":"cancelled"}"#
            ),
            Some(&store)
        )
        .await
        .unwrap()
        .0,
        "409 Conflict"
    );
    std::env::remove_var("ARGO_WEBINBOX_BEARER_TOKEN");
    std::env::remove_var("ARGO_WEBINBOX_TENANT_ID");
}
