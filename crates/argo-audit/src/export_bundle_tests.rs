//! Tests for [`crate::export_bundle`] — bundle assembly + format
//! dispatch + tar.gz layout, including the sibling `audit.csv` member
//! added by `mission-audit-export-csv-format`.

use super::*;
use crate::AuditKind;
use argo_store::SessionStore;
use std::sync::Arc;

async fn fresh_store() -> Arc<SessionStore> {
    let s = SessionStore::open(":memory:").await.unwrap();
    s.migrate().await.unwrap();
    Arc::new(s)
}

async fn write_n(store: &Arc<SessionStore>, n: usize) {
    #[allow(deprecated)]
    let sink = crate::AuditSink::new(Arc::clone(store));
    for i in 0..n {
        sink.write(
            None,
            AuditKind::TurnStart,
            serde_json::json!({"i": i}),
            None,
        )
        .await
        .unwrap();
    }
}

#[test]
fn parse_format_handles_common_aliases() {
    assert_eq!(BundleFormat::parse("json"), Some(BundleFormat::Json));
    assert_eq!(BundleFormat::parse("NDJSON"), Some(BundleFormat::Json));
    assert_eq!(BundleFormat::parse("csv"), Some(BundleFormat::Csv));
    assert_eq!(BundleFormat::parse("TAR"), Some(BundleFormat::Tar));
    assert_eq!(BundleFormat::parse("tar.gz"), Some(BundleFormat::Tar));
    assert_eq!(BundleFormat::parse("tgz"), Some(BundleFormat::Tar));
    assert!(BundleFormat::parse("xml").is_none());
}

#[test]
fn export_window_includes_endpoints() {
    let w = ExportWindow {
        from: DateTime::<Utc>::from_timestamp_micros(100),
        to: DateTime::<Utc>::from_timestamp_micros(200),
    };
    assert!(w.matches(100));
    assert!(w.matches(150));
    assert!(w.matches(200));
    assert!(!w.matches(99));
    assert!(!w.matches(201));
}

#[tokio::test]
async fn fetch_and_build_round_trip_json() {
    let store = fresh_store().await;
    write_n(&store, 3).await;
    let window = ExportWindow::default();
    let rows = fetch_rows_for_export(&store, crate::DEFAULT_AUDIT_TENANT_ID, &window)
        .await
        .unwrap();
    assert_eq!(rows.len(), 3);
    let bytes = build_bundle(&rows, BundleFormat::Json).unwrap();
    assert_eq!(bytes.iter().filter(|b| **b == b'\n').count(), 3);
}

#[tokio::test]
async fn fetch_and_build_round_trip_csv_has_header() {
    let store = fresh_store().await;
    write_n(&store, 2).await;
    let rows = fetch_rows_for_export(&store, crate::DEFAULT_AUDIT_TENANT_ID, &ExportWindow::default())
        .await
        .unwrap();
    let bytes = build_bundle(&rows, BundleFormat::Csv).unwrap();
    let s = String::from_utf8(bytes).unwrap();
    // RFC 4180 CRLF + first column label.
    assert!(s.starts_with("seq,"), "got: {s:?}");
    assert!(s.contains("\r\n"));
}

#[tokio::test]
async fn fetch_and_build_round_trip_tar() {
    let store = fresh_store().await;
    write_n(&store, 4).await;
    let rows = fetch_rows_for_export(&store, crate::DEFAULT_AUDIT_TENANT_ID, &ExportWindow::default())
        .await
        .unwrap();
    let bytes = build_bundle(&rows, BundleFormat::Tar).unwrap();
    // gzip magic prefix
    assert!(bytes.len() > 10);
    assert_eq!(&bytes[..2], &[0x1f, 0x8b]);
}

#[tokio::test]
async fn tar_bundle_contains_sibling_csv_with_header_and_rows() {
    use flate2::read::GzDecoder;
    use std::io::Read;

    let store = fresh_store().await;
    write_n(&store, 3).await;
    let rows = fetch_rows_for_export(&store, crate::DEFAULT_AUDIT_TENANT_ID, &ExportWindow::default())
        .await
        .unwrap();
    let bytes = build_bundle(&rows, BundleFormat::Tar).unwrap();

    let mut members: std::collections::HashMap<String, Vec<u8>> =
        std::collections::HashMap::new();
    let mut archive = tar::Archive::new(GzDecoder::new(&bytes[..]));
    for entry in archive.entries().unwrap() {
        let mut entry = entry.unwrap();
        let path = entry.path().unwrap().to_string_lossy().into_owned();
        let mut buf = Vec::new();
        entry.read_to_end(&mut buf).unwrap();
        members.insert(path, buf);
    }

    assert!(members.contains_key("audit.ndjson"));
    assert!(
        members.contains_key("audit.csv"),
        "audit.csv missing from tar"
    );
    assert!(members.contains_key("chain_metadata.json"));
    assert!(members.contains_key("verify.sh"));

    let csv = String::from_utf8(members["audit.csv"].clone()).unwrap();
    assert!(csv.starts_with("seq,"));
    assert!(csv.contains("\r\n"));
    // 3 data rows + 1 header row = 4 CRLF-terminated lines.
    let line_count = csv.matches("\r\n").count();
    assert_eq!(
        line_count, 4,
        "expected 4 CRLF-terminated lines, got: {csv:?}"
    );

    let metadata: serde_json::Value =
        serde_json::from_slice(&members["chain_metadata.json"]).unwrap();
    let manifest_csv_sha = metadata["audit_csv_sha256"]
        .as_str()
        .expect("audit_csv_sha256 in manifest");
    assert_eq!(manifest_csv_sha, sha256_hex(&members["audit.csv"]));
}

#[tokio::test]
async fn empty_bundle_csv_has_header_only_and_manifest_pins_sha() {
    use flate2::read::GzDecoder;
    use std::io::Read;

    let store = fresh_store().await;
    let rows = fetch_rows_for_export(&store, crate::DEFAULT_AUDIT_TENANT_ID, &ExportWindow::default())
        .await
        .unwrap();
    assert!(rows.is_empty());
    let bytes = build_bundle(&rows, BundleFormat::Tar).unwrap();

    let mut archive = tar::Archive::new(GzDecoder::new(&bytes[..]));
    let mut audit_csv: Option<Vec<u8>> = None;
    let mut manifest: Option<Vec<u8>> = None;
    for entry in archive.entries().unwrap() {
        let mut entry = entry.unwrap();
        let path = entry.path().unwrap().to_string_lossy().into_owned();
        let mut buf = Vec::new();
        entry.read_to_end(&mut buf).unwrap();
        match path.as_str() {
            "audit.csv" => audit_csv = Some(buf),
            "chain_metadata.json" => manifest = Some(buf),
            _ => {}
        }
    }
    let audit_csv = audit_csv.expect("audit.csv");
    let manifest = manifest.expect("manifest");
    let s = String::from_utf8(audit_csv.clone()).unwrap();
    assert!(s.starts_with("seq,"));
    // Header-only file: exactly one CRLF terminator, no data rows.
    assert_eq!(s.matches("\r\n").count(), 1);
    let meta: serde_json::Value = serde_json::from_slice(&manifest).unwrap();
    assert_eq!(meta["audit_csv_sha256"], sha256_hex(&audit_csv));
    assert_eq!(meta["row_count"], 0);
}

#[tokio::test]
async fn preview_counts_and_samples_match_full_export() {
    let store = fresh_store().await;
    write_n(&store, 10).await;
    let p = fetch_preview(&store, crate::DEFAULT_AUDIT_TENANT_ID, &ExportWindow::default())
        .await
        .unwrap();
    assert_eq!(p.row_count, 10);
    assert_eq!(p.sample.len(), PREVIEW_SAMPLE_CAP);
    assert!(p.byte_estimate > 0);
}

#[tokio::test]
async fn fetch_rows_for_export_excludes_other_tenants() {
    let store = fresh_store().await;
    let sink_a = crate::AuditSink::new_with_tenant(Arc::clone(&store), "tenant-a".to_string());
    let sink_b = crate::AuditSink::new_with_tenant(Arc::clone(&store), "tenant-b".to_string());
    for i in 0..3 {
        sink_a
            .write(None, AuditKind::TurnStart, serde_json::json!({"i": i}), None)
            .await
            .unwrap();
    }
    for i in 0..2 {
        sink_b
            .write(None, AuditKind::TurnStart, serde_json::json!({"i": i}), None)
            .await
            .unwrap();
    }
    let rows_a = fetch_rows_for_export(&store, "tenant-a", &ExportWindow::default())
        .await
        .unwrap();
    assert_eq!(rows_a.len(), 3, "tenant-a should only see its own rows");
    let rows_b = fetch_rows_for_export(&store, "tenant-b", &ExportWindow::default())
        .await
        .unwrap();
    assert_eq!(rows_b.len(), 2, "tenant-b should only see its own rows");
    let preview_a = fetch_preview(&store, "tenant-a", &ExportWindow::default())
        .await
        .unwrap();
    assert_eq!(preview_a.row_count, 3);
    let preview_b = fetch_preview(&store, "tenant-b", &ExportWindow::default())
        .await
        .unwrap();
    assert_eq!(preview_b.row_count, 2);
    // Unknown tenant — empty.
    let rows_unknown =
        fetch_rows_for_export(&store, "tenant-c", &ExportWindow::default())
            .await
            .unwrap();
    assert!(rows_unknown.is_empty());
}

#[tokio::test]
async fn preview_window_filter_excludes_outside_rows() {
    let store = fresh_store().await;
    write_n(&store, 5).await;
    let far_future = DateTime::<Utc>::from_timestamp_micros(
        chrono::Utc::now().timestamp_micros() + 60_000_000_000,
    );
    let window = ExportWindow {
        from: far_future,
        to: None,
    };
    let p = fetch_preview(&store, crate::DEFAULT_AUDIT_TENANT_ID, &window)
        .await
        .unwrap();
    assert_eq!(p.row_count, 0);
    assert_eq!(p.byte_estimate, 0);
    assert!(p.sample.is_empty());
}
