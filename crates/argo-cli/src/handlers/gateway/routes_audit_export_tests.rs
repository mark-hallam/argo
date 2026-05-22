//! Tests for `routes_audit_export` — POST export/preview + GET signed download.

#![allow(deprecated)]

use super::{download_response, export_response, preview_response, sign_export_url};
use argo_audit::export_bundle::BundleFormat;
use argo_audit::{AuditKind, AuditSink};
use argo_store::SessionStore;
use base64::{engine::general_purpose, Engine as _};
use serde_json::{json, Value};
use std::fs;
use std::process::Command;
use std::sync::Arc;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

async fn mem_store() -> Arc<SessionStore> {
    let s = SessionStore::open(":memory:").await.expect("open mem db");
    s.migrate().await.expect("migrate");
    Arc::new(s)
}

async fn seed(store: &Arc<SessionStore>, n: usize) {
    let sink = AuditSink::new(Arc::clone(store));
    for i in 0..n {
        sink.write(
            None,
            AuditKind::ToolCall,
            json!({"i": i, "tool": "echo"}),
            None,
        )
        .await
        .expect("write audit");
    }
}

fn post(path: &str, body: Value) -> String {
    let body = body.to_string();
    format!(
        "POST {path} HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

fn get(path: &str) -> String {
    format!("GET {path} HTTP/1.1\r\nHost: x\r\n\r\n")
}

fn parse(resp: &(&str, String)) -> Value {
    serde_json::from_str(&resp.1)
        .unwrap_or_else(|_| panic!("invalid JSON: status={} body={}", resp.0, resp.1))
}

fn unique_export_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir()
        .join("argo-test-audit-export")
        .join(format!("{tag}-{}", uuid::Uuid::now_v7().simple()));
    std::fs::create_dir_all(&dir).unwrap();
    std::env::set_var("ARGO_AUDIT_EXPORT_DIR", &dir);
    dir
}

#[tokio::test]
async fn export_returns_signed_url_and_writes_bundle() {
    let _dir = unique_export_dir("export-signed");
    let store = mem_store().await;
    seed(&store, 3).await;
    let req = post(
        "/api/v1/audit/export",
        json!({"format": "json"}),
    );
    let resp = export_response(&req, Some(&store)).await;
    assert!(resp.0.starts_with("201"), "status: {} body: {}", resp.0, resp.1);
    let body = parse(&resp);
    assert_eq!(body["row_count"], 3);
    assert!(body["byte_size"].as_u64().unwrap() > 0);
    let url = body["download_url"].as_str().unwrap();
    assert!(url.starts_with("/api/v1/audit/export/"));
    assert!(url.contains("&sig="));
    assert!(url.contains("expires="));
    assert!(body["sha256"].as_str().unwrap().len() == 64);
}

#[tokio::test]
async fn download_signed_url_round_trip_returns_bundle_bytes() {
    let _dir = unique_export_dir("download-roundtrip");
    let store = mem_store().await;
    seed(&store, 2).await;
    // POST to generate.
    let post_resp = export_response(
        &post("/api/v1/audit/export", json!({"format": "json"})),
        Some(&store),
    )
    .await;
    let post_body = parse(&post_resp);
    let url = post_body["download_url"].as_str().unwrap().to_string();
    let bundle_id = post_body["bundle_id"].as_str().unwrap().to_string();
    let expected_sha = post_body["sha256"].as_str().unwrap().to_string();
    // GET the signed URL.
    let get_resp = super::try_dispatch_get(&get(&url), Some(&store))
        .await
        .expect("download dispatched");
    assert!(get_resp.0.starts_with("200"), "status: {}", get_resp.0);
    let get_body = parse(&get_resp);
    assert_eq!(get_body["bundle_id"], bundle_id);
    assert_eq!(get_body["sha256"], expected_sha);
    let b64 = get_body["b64"].as_str().unwrap();
    let bytes = general_purpose::STANDARD.decode(b64).unwrap();
    // NDJSON = 2 rows = 2 newline-terminated JSON lines.
    assert_eq!(bytes.iter().filter(|b| **b == b'\n').count(), 2);
}

#[tokio::test]
async fn download_expired_signature_is_rejected() {
    let _dir = unique_export_dir("download-expired");
    let store = mem_store().await;
    let bundle_id = "expired-test-bundle";
    let past = chrono::Utc::now().timestamp() - 100;
    let url = sign_export_url(bundle_id, BundleFormat::Json, past);
    let resp = download_response(bundle_id, &url, Some(&store)).await;
    assert!(resp.0.starts_with("401"), "status: {}", resp.0);
    assert_eq!(parse(&resp)["error"]["code"], "unauthorized");
}

#[tokio::test]
async fn download_tampered_signature_is_rejected() {
    let _dir = unique_export_dir("download-tampered");
    let store = mem_store().await;
    let bundle_id = "tampered-test-bundle";
    let future = chrono::Utc::now().timestamp() + 600;
    let good_url = sign_export_url(bundle_id, BundleFormat::Json, future);
    // Flip the last hex char.
    let mut bad: Vec<char> = good_url.chars().collect();
    let last = bad.len() - 1;
    bad[last] = if bad[last] == '0' { '1' } else { '0' };
    let bad_url: String = bad.into_iter().collect();
    let resp = download_response(bundle_id, &bad_url, Some(&store)).await;
    assert!(resp.0.starts_with("401"), "status: {}", resp.0);
}

#[tokio::test]
async fn download_unknown_bundle_returns_404() {
    let _dir = unique_export_dir("download-unknown");
    let store = mem_store().await;
    let bundle_id = "nonexistent-bundle";
    let future = chrono::Utc::now().timestamp() + 600;
    let url = sign_export_url(bundle_id, BundleFormat::Json, future);
    let resp = download_response(bundle_id, &url, Some(&store)).await;
    assert!(resp.0.starts_with("404"), "status: {}", resp.0);
}

#[tokio::test]
async fn preview_returns_count_estimate_and_sample() {
    let store = mem_store().await;
    seed(&store, 7).await;
    let resp = preview_response(
        &post("/api/v1/audit/export/preview", json!({"format": "json"})),
        Some(&store),
    )
    .await;
    assert!(resp.0.starts_with("200"), "status: {}", resp.0);
    let body = parse(&resp);
    assert_eq!(body["row_count"], 7);
    assert!(body["byte_estimate"].as_u64().unwrap() > 0);
    assert_eq!(body["sample"].as_array().unwrap().len(), 5);
}

#[tokio::test]
async fn preview_with_future_window_returns_zero_rows() {
    let store = mem_store().await;
    seed(&store, 3).await;
    let future = (chrono::Utc::now() + chrono::Duration::days(30)).to_rfc3339();
    let resp = preview_response(
        &post(
            "/api/v1/audit/export/preview",
            json!({"format": "json", "from": future}),
        ),
        Some(&store),
    )
    .await;
    let body = parse(&resp);
    assert_eq!(body["row_count"], 0);
    assert!(body["sample"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn export_missing_format_returns_400() {
    let _dir = unique_export_dir("export-missing-format");
    let store = mem_store().await;
    let resp = export_response(
        &post("/api/v1/audit/export", json!({})),
        Some(&store),
    )
    .await;
    assert!(resp.0.starts_with("400"), "status: {}", resp.0);
}

#[tokio::test]
async fn export_bad_format_returns_400() {
    let _dir = unique_export_dir("export-bad-format");
    let store = mem_store().await;
    let resp = export_response(
        &post("/api/v1/audit/export", json!({"format": "xml"})),
        Some(&store),
    )
    .await;
    assert!(resp.0.starts_with("400"), "status: {}", resp.0);
}

#[tokio::test]
async fn export_window_validation_rejects_from_after_to() {
    let _dir = unique_export_dir("export-bad-window");
    let store = mem_store().await;
    let resp = export_response(
        &post(
            "/api/v1/audit/export",
            json!({
                "format": "json",
                "from": "2026-06-01T00:00:00Z",
                "to":   "2026-05-01T00:00:00Z",
            }),
        ),
        Some(&store),
    )
    .await;
    assert!(resp.0.starts_with("400"), "status: {}", resp.0);
}

#[tokio::test]
async fn export_csv_format_writes_csv_bundle() {
    let _dir = unique_export_dir("export-csv");
    let store = mem_store().await;
    seed(&store, 2).await;
    let resp = export_response(
        &post("/api/v1/audit/export", json!({"format": "csv"})),
        Some(&store),
    )
    .await;
    assert!(resp.0.starts_with("201"), "status: {}", resp.0);
    let body = parse(&resp);
    assert_eq!(body["format"], "csv");
    let url = body["download_url"].as_str().unwrap();
    let dl = super::try_dispatch_get(&get(url), Some(&store)).await.unwrap();
    let dl_body = parse(&dl);
    let bytes = general_purpose::STANDARD
        .decode(dl_body["b64"].as_str().unwrap())
        .unwrap();
    let s = String::from_utf8(bytes).unwrap();
    assert!(s.starts_with("seq,"));
}

#[tokio::test]
async fn export_tar_format_writes_gzip_magic() {
    let _dir = unique_export_dir("export-tar");
    let store = mem_store().await;
    seed(&store, 2).await;
    let resp = export_response(
        &post("/api/v1/audit/export", json!({"format": "tar"})),
        Some(&store),
    )
    .await;
    assert!(resp.0.starts_with("201"), "status: {} body: {}", resp.0, resp.1);
    let body = parse(&resp);
    let url = body["download_url"].as_str().unwrap();
    let dl = super::try_dispatch_get(&get(url), Some(&store)).await.unwrap();
    let dl_body = parse(&dl);
    let bytes = general_purpose::STANDARD
        .decode(dl_body["b64"].as_str().unwrap())
        .unwrap();
    assert_eq!(&bytes[..2], &[0x1f, 0x8b], "gzip magic");
}

#[tokio::test]
async fn exported_bundle_contains_verify_script_executes_clean() {
    let dir = unique_export_dir("export-tar-verify");
    let unpack_dir = dir.join("unpacked");
    fs::create_dir_all(&unpack_dir).unwrap();
    let store = mem_store().await;
    seed(&store, 3).await;
    let resp = export_response(
        &post("/api/v1/audit/export", json!({"format": "tar"})),
        Some(&store),
    )
    .await;
    assert!(
        resp.0.starts_with("201"),
        "status: {} body: {}",
        resp.0,
        resp.1
    );
    let body = parse(&resp);
    let url = body["download_url"].as_str().unwrap();
    let dl = super::try_dispatch_get(&get(url), Some(&store))
        .await
        .unwrap();
    let dl_body = parse(&dl);
    let bytes = general_purpose::STANDARD
        .decode(dl_body["b64"].as_str().unwrap())
        .unwrap();
    let bundle_path = dir.join("bundle.tar.gz");
    fs::write(&bundle_path, bytes).unwrap();

    let tar_status = Command::new("tar")
        .arg("-xzf")
        .arg(&bundle_path)
        .arg("-C")
        .arg(&unpack_dir)
        .status()
        .unwrap();
    assert!(tar_status.success(), "tar extraction failed: {tar_status}");

    let verify_path = unpack_dir.join("verify.sh");
    assert!(verify_path.exists(), "verify.sh missing from export tar");
    let csv_path = unpack_dir.join("audit.csv");
    assert!(csv_path.exists(), "audit.csv missing from export tar");
    let csv_body = fs::read_to_string(&csv_path).unwrap();
    assert!(csv_body.starts_with("seq,"), "csv header: {csv_body:?}");
    assert!(csv_body.contains("\r\n"), "csv must be RFC 4180 CRLF: {csv_body:?}");
    // 3 seeded rows + 1 header row.
    assert_eq!(csv_body.matches("\r\n").count(), 4);
    #[cfg(unix)]
    assert_eq!(
        fs::metadata(&verify_path).unwrap().permissions().mode() & 0o777,
        0o755
    );
    let chmod_status = Command::new("chmod")
        .arg("+x")
        .arg(&verify_path)
        .status()
        .unwrap();
    assert!(chmod_status.success(), "chmod failed: {chmod_status}");
    let output = Command::new(&verify_path)
        .current_dir(&unpack_dir)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "verify.sh failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("chain OK"),
        "stdout={}",
        String::from_utf8_lossy(&output.stdout)
    );
}
