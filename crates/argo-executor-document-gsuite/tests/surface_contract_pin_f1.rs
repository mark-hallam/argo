//! LIVE surface-contract pin tests for **F-1** in `argo-executor-document-gsuite`.
//!
//! Companion to `surface_contract_pin_f1.rs` in `argo-executor-document/`
//! and `argo-executor-document-csv/`. See the v2 audit-report delta at
//! `docs/skycot/evals/scheduled/2026-05-23/eval-document-executor-cross-tenant-scoping-v2.md`.
//!
//! `GsuiteWriteRequest` derives `Serialize`, so JSON shape pins are
//! the cleanest probe. `GsuiteWriteReport` does not derive `Serialize`,
//! so we exhaustively destructure it instead.

use argo_executor_document_gsuite::{
    DocumentGsuite, GsuiteDocId, GsuiteDocKind, GsuiteRejection, GsuiteWriteReport,
    GsuiteWriteRequest,
};
use argo_types::ArgoResult;

// =========================================================================
// LIVE PIN — DocumentGsuite::write signature
// =========================================================================

/// **PIN F-1.a (gsuite)** — `DocumentGsuite::write` is async and takes
/// `(&self, GsuiteWriteRequest)`. The compile-time check is the
/// closure bind: any `tenant_id` arg insertion makes it stop
/// compiling.
#[test]
fn pin_document_gsuite_write_signature_no_tenant_arg() {
    let _check = |exec: &DocumentGsuite, req: GsuiteWriteRequest| {
        let _fut: std::pin::Pin<Box<dyn std::future::Future<Output = ArgoResult<GsuiteWriteReport>>>> =
            Box::pin(exec.write(req));
    };
}

// =========================================================================
// LIVE PIN — GsuiteWriteRequest field set + JSON shape
// =========================================================================

/// **PIN F-1.b (gsuite)** — `GsuiteWriteRequest` field set is exactly
/// `{ kind, title, drive_folder_id, doc_body_text, sheet_schema,
/// sheet_rows, slides }` — no `tenant_id`. Exhaustive destructure is
/// the compile-time guard.
#[test]
fn pin_gsuite_write_request_field_set_has_no_tenant() {
    let req = GsuiteWriteRequest {
        kind: GsuiteDocKind::Doc,
        title: "x".into(),
        drive_folder_id: None,
        doc_body_text: None,
        sheet_schema: None,
        sheet_rows: Vec::new(),
        slides: Vec::new(),
    };
    let GsuiteWriteRequest {
        kind,
        title,
        drive_folder_id,
        doc_body_text,
        sheet_schema,
        sheet_rows,
        slides,
    } = req;
    let _ = (
        kind,
        title,
        drive_folder_id,
        doc_body_text,
        sheet_schema,
        sheet_rows,
        slides,
    );
}

/// **PIN F-1.c (gsuite)** — `GsuiteWriteRequest` JSON shape for a
/// `Doc` kind. Keys observed today: `kind`, `title`, `drive_folder_id`,
/// `doc_body_text`, `sheet_schema`, `sheet_rows`, `slides`. No
/// `tenant_id`.
#[test]
fn pin_gsuite_write_request_json_keys_have_no_tenant() {
    let req = GsuiteWriteRequest {
        kind: GsuiteDocKind::Doc,
        title: "Tenant A quarterly".into(),
        drive_folder_id: Some("folder-123".into()),
        doc_body_text: Some("# Body".into()),
        sheet_schema: None,
        sheet_rows: Vec::new(),
        slides: Vec::new(),
    };
    let json = serde_json::to_value(&req).expect("serialize");
    let obj = json.as_object().expect("object");
    let mut keys: Vec<&str> = obj.keys().map(|s| s.as_str()).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        vec![
            "doc_body_text",
            "drive_folder_id",
            "kind",
            "sheet_rows",
            "sheet_schema",
            "slides",
            "title",
        ]
    );
}

// =========================================================================
// LIVE PIN — GsuiteWriteReport field set
// =========================================================================

/// **PIN F-1.d (gsuite)** — `GsuiteWriteReport` field set is exactly
/// `{ doc_id, web_view_link, mime_type, kind, created_at,
/// sha256_of_request, rejections }` — no tenant tag on the
/// audit-ledger outcome row.
#[test]
fn pin_gsuite_write_report_field_set_has_no_tenant() {
    let report = GsuiteWriteReport {
        doc_id: GsuiteDocId("drive-id".into()),
        web_view_link: "https://drive.google.com/x".into(),
        mime_type: "application/vnd.google-apps.document".into(),
        kind: GsuiteDocKind::Doc,
        created_at: "2026-05-24T00:00:00Z".into(),
        sha256_of_request: "deadbeef".into(),
        rejections: Vec::<GsuiteRejection>::new(),
    };
    let GsuiteWriteReport {
        doc_id,
        web_view_link,
        mime_type,
        kind,
        created_at,
        sha256_of_request,
        rejections,
    } = report;
    let _ = (
        doc_id,
        web_view_link,
        mime_type,
        kind,
        created_at,
        sha256_of_request,
        rejections,
    );
}

// =========================================================================
// POST-FIX — #[ignore]'d, un-ignore when the fix-brief lands
// =========================================================================

/// **POST-FIX F-1 (gsuite)** — companion to the pins above. Today
/// `GsuiteWriteRequest` carries no tenant_id; once
/// `mission-document-executor-tenant-stamp` (or
/// `mission-document-executors-per-tenant-credentials`) lands, the
/// JSON shape should advertise a tenant axis.
///
/// **Un-ignore when**: the fix-brief adds the field. At that point
/// `pin_gsuite_write_request_json_keys_have_no_tenant` will fail and
/// this test should be promoted to a LIVE positive regression check.
#[test]
#[ignore = "POST-FIX: un-ignore when mission-document-executor-tenant-stamp lands; see surface_contract_pin_f1.rs header"]
fn post_fix_gsuite_write_request_carries_tenant_axis() {
    let req = GsuiteWriteRequest {
        kind: GsuiteDocKind::Doc,
        title: "x".into(),
        drive_folder_id: None,
        doc_body_text: None,
        sheet_schema: None,
        sheet_rows: Vec::new(),
        slides: Vec::new(),
    };
    let json = serde_json::to_value(&req).expect("serialize");
    let obj = json.as_object().expect("object");
    assert!(
        obj.contains_key("tenant_id"),
        "POST-FIX: GsuiteWriteRequest should serialise a tenant_id once the fix-brief lands"
    );
}
