//! Unit tests for the action-level API executor.

use super::*;
use crate::handlers::worker_actions::ActionRecord;
use argo_identity::WorkerId;
use argo_vertical::{ActionDeclaration, ExecutorDeclaration, ExecutorTagged};
use serde_json::json;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::thread;
use std::time::Duration;

fn sample_action() -> ActionRecord {
    ActionRecord {
        schema_key: ActionRecord::SCHEMA_KEY.to_string(),
        action_id: "act-1".into(),
        job_id: "job-1".into(),
        worker_id: WorkerId::nil(),
        item_id: Some("item-7".into()),
        action_type: "post_ticket".into(),
        risk_level: "low".into(),
        requires_approval: false,
        state: "executing".into(),
        payload: json!({"customer": {"name": "Ada"}, "message": "hello"}),
        approved_by: None,
        approved_at: None,
        executed_at: None,
        platform: Some("zendesk".into()),
        result: None,
        evidence: vec![],
        blocked_reason: None,
        created_at: "2026-05-07T00:00:00Z".into(),
    }
}

fn decl(binding: Option<String>) -> ActionDeclaration {
    let mut tagged = ExecutorTagged {
        kind: "api".into(),
        ..Default::default()
    };
    tagged.binding = binding;
    ActionDeclaration {
        executor: ExecutorDeclaration::Tagged(tagged),
        ..Default::default()
    }
}

fn one_shot_server(status: &'static str, body: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buf = [0; 4096];
        let n = stream.read(&mut buf).unwrap();
        let req = String::from_utf8_lossy(&buf[..n]);
        assert!(req.contains("POST /run/act-1"));
        assert!(req.to_ascii_lowercase().contains("x-customer: ada"));
        assert!(req.contains(r#""body":"Ada""#));
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(response.as_bytes()).unwrap();
    });
    format!("http://{addr}/run/{{{{ action_id }}}}")
}

fn binding_json(url: String) -> String {
    json!({
        "url": url,
        "method": "POST",
        "headers": {"X-Customer": "{{ payload.customer.name }}"},
        "body_template": r#"{"body":"{{ payload.customer.name }}"}"#,
    })
    .to_string()
}

#[tokio::test]
async fn success_returns_completed() {
    let url = one_shot_server("200 OK", r#"{"ok":true}"#);
    let pool = ApiPool::default();
    let outcome = dispatch_action(
        &sample_action(),
        Path::new("."),
        &decl(Some(binding_json(url))),
        &pool,
        Duration::from_secs(2),
    )
    .await
    .unwrap();
    match outcome {
        ActionOutcome::Completed { result_json, .. } => {
            assert_eq!(result_json["executor"], "api");
            assert_eq!(result_json["status"], 200);
            assert_eq!(result_json["body"]["ok"], true);
        }
        other => panic!("expected Completed, got {other:?}"),
    }
}

#[tokio::test]
async fn non_2xx_status_returns_provider_error() {
    let url = one_shot_server("503 Service Unavailable", r#"{"error":"down"}"#);
    let pool = ApiPool::default();
    let err = dispatch_action(
        &sample_action(),
        Path::new("."),
        &decl(Some(binding_json(url))),
        &pool,
        Duration::from_secs(2),
    )
    .await
    .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("HTTP 503"), "got {msg}");
    assert!(msg.contains("down"), "got {msg}");
}

#[tokio::test]
async fn timeout_returns_provider_error() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    thread::spawn(move || {
        let (_stream, _) = listener.accept().unwrap();
        thread::sleep(Duration::from_millis(200));
    });
    let pool = ApiPool::default();
    let err = dispatch_action(
        &sample_action(),
        Path::new("."),
        &decl(Some(format!("http://{addr}/slow"))),
        &pool,
        Duration::from_millis(20),
    )
    .await
    .unwrap_err();
    assert!(err.to_string().contains("exceeded wallclock budget"));
}

#[tokio::test]
async fn missing_url_binding_returns_provider_error() {
    let pool = ApiPool::default();
    let err = dispatch_action(
        &sample_action(),
        Path::new("."),
        &decl(None),
        &pool,
        Duration::from_secs(1),
    )
    .await
    .unwrap_err();
    assert!(err.to_string().contains("missing `executor.binding`"));
}

#[test]
fn args_substitution_renders_payload_and_action_fields() {
    let rendered = render_string(
        "send {{ payload.customer.name }} for {{ action_id }}",
        &sample_action(),
    )
    .unwrap();
    assert_eq!(rendered, "send Ada for act-1");
}

#[test]
fn secret_substitution_reads_credential_fallback() {
    std::env::set_var("API_EXECUTOR_TEST_SECRET", "secret-123");
    let rendered = render_string(
        "Bearer {{ secret.API_EXECUTOR_TEST_SECRET }}",
        &sample_action(),
    )
    .unwrap();
    assert_eq!(rendered, "Bearer secret-123");
    std::env::remove_var("API_EXECUTOR_TEST_SECRET");
}
