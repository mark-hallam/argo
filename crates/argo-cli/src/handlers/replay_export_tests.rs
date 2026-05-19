#![cfg(test)]
#![allow(deprecated)]

use super::*;
use argo_audit::{AuditKind, AuditSink};
use argo_store::SessionStore;
use argo_types::{TenantId, WorkspaceId};
use flate2::read::GzDecoder;
use std::io::Read;
use std::sync::Arc;

#[test]
fn parse_session_id_rejects_bad_string() {
    let err = parse_session_id("not-a-uuid").unwrap_err();
    assert!(format!("{err}").contains("invalid session id"));
}

#[test]
fn ensure_parent_dir_errors_when_missing() {
    let p = Path::new("/tmp/this-dir-should-not-exist-mission-replay-cli-bundle-export-xyz/out.tar");
    let err = ensure_parent_dir(p).unwrap_err();
    assert!(format!("{err}").contains("output parent does not exist"));
}

#[test]
fn ensure_parent_dir_passes_when_present() {
    let p = Path::new("/tmp/some-file.tar");
    assert!(ensure_parent_dir(p).is_ok());
}

#[test]
fn render_manifest_prefix_includes_metadata() {
    let header = argo_store::SessionHeader {
        id: argo_types::SessionId::new(),
        tenant_id: TenantId::new(),
        workspace_id: WorkspaceId::new(),
        model: "claude-sonnet-4-6".into(),
        provider: "anthropic".into(),
        title: Some("incident-12 root cause".into()),
        started_at: chrono::Utc::now(),
        ended_at: None,
        vertical_id: None,
        worker_id: None,
        routing_policy: None,
    };
    let prefix = render_manifest_prefix(&header, &[], false);
    assert!(prefix.contains("session_id ="));
    assert!(prefix.contains("tenant_id ="));
    assert!(prefix.contains("model = \"claude-sonnet-4-6\""));
    assert!(prefix.contains("provider = \"anthropic\""));
    assert!(prefix.contains("title = \"incident-12 root cause\""));
    assert!(prefix.contains("audit_row_count = 0"));
}

#[test]
fn render_manifest_prefix_sanitizes_title_when_requested() {
    let header = argo_store::SessionHeader {
        id: argo_types::SessionId::new(),
        tenant_id: TenantId::new(),
        workspace_id: WorkspaceId::new(),
        model: "m".into(),
        provider: "p".into(),
        title: Some("escalation for ops@example.com".into()),
        started_at: chrono::Utc::now(),
        ended_at: None,
        vertical_id: None,
        worker_id: None,
        routing_policy: None,
    };
    let prefix = render_manifest_prefix(&header, &[], true);
    assert!(!prefix.contains("ops@example.com"));
    assert!(prefix.contains("email-redacted-"));
}

#[tokio::test]
async fn end_to_end_bundle_round_trips_audit_chain() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let store = Arc::new(store);
    let tenant = TenantId::new();
    let ws = WorkspaceId::new();
    sqlx::query(
        "INSERT INTO workspace (id, tenant_id, name, root_path, created_at)
         VALUES (?, ?, 'replay-export-test', '.', strftime('%s','now'))",
    )
    .bind(ws.0.as_bytes().as_slice())
    .bind(tenant.0.as_bytes().as_slice())
    .execute(store.pool())
    .await
    .unwrap();
    let sid = store
        .create_session(tenant, ws, "mock", "mock", Some("incident-replay"), None)
        .await
        .unwrap();
    let sink = AuditSink::new(Arc::clone(&store));
    sink.write(
        Some(sid),
        AuditKind::TurnStart,
        serde_json::json!({"prompt": "what is the api key sk-abcdefghijklmnopqrstuvwx?"}),
        None,
    )
    .await
    .unwrap();
    sink.write(
        Some(sid),
        AuditKind::ToolCall,
        serde_json::json!({"tool": "fs_read", "args": {"path": "/etc/hosts"}}),
        None,
    )
    .await
    .unwrap();
    sink.write(
        Some(sid),
        AuditKind::ToolResult,
        serde_json::json!({"ok": true, "bytes": 124}),
        None,
    )
    .await
    .unwrap();

    let rows = rows_for_session(store.as_ref(), sid).await.unwrap();
    assert_eq!(rows.len(), 3);
    let verify = verify_session(store.as_ref(), sid).await.unwrap();
    assert!(verify.ok);

    let manifest_prefix = render_manifest_prefix(
        &store.get_session(sid).await.unwrap().unwrap(),
        &rows,
        true,
    );
    let files = build_files(&rows, &verify, &[]).unwrap();
    let bytes = build_bundle(
        files,
        manifest_prefix,
        BundleOptions {
            sanitize: true,
            compressed: true,
            readme: README,
        },
    )
    .unwrap();
    let mut decoder = GzDecoder::new(bytes.as_slice());
    let mut raw = Vec::new();
    decoder.read_to_end(&mut raw).unwrap();
    let mut archive = tar::Archive::new(raw.as_slice());
    let mut names: Vec<String> = Vec::new();
    let mut chain: Option<Vec<u8>> = None;
    for entry in archive.entries().unwrap() {
        let mut e = entry.unwrap();
        let path = e.path().unwrap().to_string_lossy().to_string();
        let mut body = Vec::new();
        e.read_to_end(&mut body).unwrap();
        if path == "audit/chain.jsonl" {
            chain = Some(body.clone());
        }
        names.push(path);
    }
    assert!(names.contains(&"manifest.toml".into()));
    assert!(names.contains(&"audit/chain.jsonl".into()));
    assert!(names.contains(&"audit/verify.txt".into()));
    assert!(names.contains(&"bundle/action-2.json".into()));
    assert!(names.contains(&"README.md".into()));

    let chain_bytes = chain.expect("chain.jsonl present");
    let lines: Vec<&[u8]> = chain_bytes
        .split(|b| *b == b'\n')
        .filter(|l| !l.is_empty())
        .collect();
    assert_eq!(lines.len(), 3);
    // Sanitize stripped the sk-* key from the payload.
    let chain_str = std::str::from_utf8(&chain_bytes).unwrap();
    assert!(!chain_str.contains("sk-abcdefghijklmnopqrstuvwx"));
    assert!(chain_str.contains("sk-redacted-"));
}
