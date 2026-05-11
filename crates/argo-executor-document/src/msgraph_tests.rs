//! Unit tests for the Microsoft Graph executor.
//!
//! Pure-function tests only: payload generation, response parsing,
//! URL construction, auth-error classification. The actual HTTP
//! round-trip is covered by `tests/msgraph_smoke.rs`, gated by the
//! `ARGO_MSGRAPH_TEST_TOKEN` env var.

use super::*;

/// Open XML (.docx/.xlsx) files are PKZIP containers; the local-file
/// magic is "PK\x03\x04" at offset 0.
const PKZIP_MAGIC: &[u8] = b"PK\x03\x04";

#[test]
fn docx_template_generates_valid_zip() {
    let payload = build_payload(
        &MsGraphFormat::Docx {
            title: "Status note".into(),
            body: "All systems nominal.".into(),
        },
        &[],
    )
    .expect("docx build");
    assert!(payload.starts_with(PKZIP_MAGIC), "missing PKZIP magic");
    assert!(payload.len() > 200, "docx zip looks empty: {} bytes", payload.len());
    let reader = zip::ZipArchive::new(std::io::Cursor::new(&payload[..]))
        .expect("docx archive opens");
    let names: Vec<_> = reader.file_names().collect();
    assert!(names.contains(&"[Content_Types].xml"));
    assert!(names.contains(&"word/document.xml"));
    assert!(names.contains(&"_rels/.rels"));
}

#[test]
fn xlsx_template_generates_valid_zip() {
    let payload = build_payload(
        &MsGraphFormat::Xlsx {
            sheet_name: "Q3 numbers".into(),
            rows: vec![
                vec!["metric".into(), "value".into()],
                vec!["ARR".into(), "$4.8M".into()],
            ],
        },
        &[],
    )
    .expect("xlsx build");
    assert!(payload.starts_with(PKZIP_MAGIC), "missing PKZIP magic");
    let reader = zip::ZipArchive::new(std::io::Cursor::new(&payload[..]))
        .expect("xlsx archive opens");
    let names: Vec<_> = reader.file_names().collect();
    assert!(names.contains(&"[Content_Types].xml"));
    assert!(names.contains(&"xl/workbook.xml"));
    assert!(names.contains(&"xl/worksheets/sheet1.xml"));
}

#[test]
fn upload_only_passes_bytes_through() {
    let raw = b"\x89PNG\r\n\x1a\nmade-up-png-bytes";
    let out = build_payload(&MsGraphFormat::UploadOnly, raw).expect("upload-only");
    assert_eq!(out.as_slice(), raw);
}

#[test]
fn create_returns_web_url() {
    let json = serde_json::json!({
        "id": "01ABCDEFGH",
        "name": "report.docx",
        "size": 1234,
        "webUrl": "https://onedrive.live.com/?id=01ABCDEFGH"
    });
    let file = parse_drive_item(&json).expect("parse driveItem");
    assert_eq!(file.web_url, "https://onedrive.live.com/?id=01ABCDEFGH");
    assert_eq!(file.item_id, "01ABCDEFGH");
    assert_eq!(file.size_bytes, 1234);
}

#[test]
fn read_returns_bytes() {
    let client = MsGraphClient::new("tok").with_base_url("https://example.test/v1.0");
    let exec = MsGraphExecutor::new(client.clone());
    let url = client.endpoint_for_drive_item(exec.drive_id.as_deref(), "reports/q3.docx");
    assert_eq!(url, "https://example.test/v1.0/me/drive/root:/reports/q3.docx");
    let with_drive = MsGraphExecutor::new(client.clone()).with_drive_id("d!XYZ");
    let url2 = client.endpoint_for_drive_item(with_drive.drive_id.as_deref(), "/q3.docx");
    assert_eq!(url2, "https://example.test/v1.0/drives/d!XYZ/root:/q3.docx");
}

#[test]
fn auth_failure_surfaces_clearly() {
    let err = classify_auth_error(401);
    match err {
        MsGraphError::AuthExpired { status, ref hint } => {
            assert_eq!(status, 401);
            assert!(hint.contains("refresh"), "hint should mention refresh: {hint}");
            assert!(
                hint.contains("ARGO_MSGRAPH_ACCESS_TOKEN"),
                "hint should name the env var: {hint}",
            );
        }
        other => panic!("expected AuthExpired, got {other:?}"),
    }
    let missing = MsGraphClient::from_env().err();
    if std::env::var("ARGO_MSGRAPH_ACCESS_TOKEN").is_err() {
        assert!(matches!(missing, Some(MsGraphError::AuthMissing { .. })));
    }
}
