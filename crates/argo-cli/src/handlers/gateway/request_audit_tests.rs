use super::*;
use std::sync::Arc;
use std::time::Duration;

fn cfg() -> GatewayRequestAuditConfig {
    GatewayRequestAuditConfig::default()
}

#[test]
fn capture_returns_none_when_disabled() {
    let mut cfg = cfg();
    cfg.enabled = false;
    let req = "GET /health HTTP/1.1\r\nHost: x\r\n\r\n";
    assert!(capture(
        req,
        "127.0.0.1".parse().unwrap(),
        "127.0.0.1".parse().unwrap(),
        &cfg
    )
    .is_none());
}

#[test]
fn capture_strips_query_and_keeps_anon_principal() {
    let req = "GET /providers?token=secret HTTP/1.1\r\nHost: x\r\n\r\n";
    let got = capture(
        req,
        "127.0.0.1".parse().unwrap(),
        "127.0.0.1".parse().unwrap(),
        &cfg(),
    )
    .unwrap();
    assert_eq!(got.method, "GET");
    assert_eq!(got.path, "/providers");
    assert!(got.principal.is_none());
}

#[test]
fn capture_hashes_configured_non_empty_body() {
    let req = "POST /sessions HTTP/1.1\r\nHost: x\r\nContent-Length: 2\r\n\r\n{}";
    let got = capture(
        req,
        "127.0.0.1".parse().unwrap(),
        "127.0.0.1".parse().unwrap(),
        &cfg(),
    )
    .unwrap();
    assert!(got.request_body_sha256.is_some());
}

#[test]
fn capture_skips_get_body_hash() {
    let req = "GET /sessions HTTP/1.1\r\nHost: x\r\n\r\n";
    let got = capture(
        req,
        "127.0.0.1".parse().unwrap(),
        "127.0.0.1".parse().unwrap(),
        &cfg(),
    )
    .unwrap();
    assert!(got.request_body_sha256.is_none());
}

#[test]
fn capture_honors_x_forwarded_for_only_when_enabled() {
    let req = "GET / HTTP/1.1\r\nX-Forwarded-For: 203.0.113.9, 10.0.0.1\r\n\r\n";
    let peer = "127.0.0.1".parse().unwrap();
    let observed = "10.0.0.1".parse().unwrap();
    let direct = capture(req, peer, observed, &cfg()).unwrap();
    assert_eq!(direct.source_ip, "127.0.0.1");
    let mut proxy_cfg = cfg();
    proxy_cfg.behind_proxy = true;
    let proxied = capture(req, peer, observed, &proxy_cfg).unwrap();
    assert_eq!(proxied.source_ip, "203.0.113.9");
}

#[test]
fn user_agent_is_truncated() {
    let mut cfg = cfg();
    cfg.user_agent_max_len = 5;
    let req = "GET / HTTP/1.1\r\nUser-Agent: abcdefgh\r\n\r\n";
    let got = capture(
        req,
        "127.0.0.1".parse().unwrap(),
        "127.0.0.1".parse().unwrap(),
        &cfg,
    )
    .unwrap();
    assert_eq!(got.user_agent.as_deref(), Some("abcde"));
}

#[test]
fn response_body_len_parses_http_bytes() {
    let resp = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}";
    assert_eq!(response_body_len_from_bytes(resp), 2);
}

#[tokio::test]
async fn spawn_record_returns_before_audit_insert_commits() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("sessions.db");
    let store = Arc::new(SessionStore::open(&db).await.unwrap());
    store.migrate().await.unwrap();

    let mut tx = store.pool().begin().await.unwrap();
    sqlx::query(
        "INSERT INTO gateway_request_audit \
         (request_id, received_at, principal, source_ip, method, path, status_code, \
          latency_ms, request_body_sha256, response_body_len, user_agent) \
         VALUES (?, 1, NULL, '127.0.0.1', 'GET', '/blocker', 200, 1, NULL, 0, NULL)",
    )
    .bind(Uuid::now_v7().as_bytes().as_slice())
    .execute(&mut *tx)
    .await
    .unwrap();

    let request_id = Uuid::now_v7();
    let draft = RequestAuditDraft {
        request_id,
        received_at: 2,
        started: Instant::now(),
        principal: None,
        source_ip: "127.0.0.1".into(),
        method: "GET".into(),
        path: "/async".into(),
        request_body_sha256: None,
        user_agent: None,
    };
    let started = Instant::now();
    spawn_record(Some(draft), Some(&store), "200 OK", 2);
    assert!(started.elapsed() < Duration::from_millis(50));

    tokio::time::sleep(Duration::from_millis(20)).await;
    assert!(
        gateway_request_audit::get(store.pool(), &request_id.to_string())
            .await
            .unwrap()
            .is_none()
    );

    tx.rollback().await.unwrap();
    for _ in 0..20 {
        if gateway_request_audit::get(store.pool(), &request_id.to_string())
            .await
            .unwrap()
            .is_some()
        {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("audit row was not written after write lock released");
}
