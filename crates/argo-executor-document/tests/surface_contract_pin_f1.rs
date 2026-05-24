//! LIVE surface-contract pin tests for **F-1** (no `TenantId` on the
//! `argo-executor-document` public dispatch surface).
//!
//! This file is the v2 follow-on to
//! `eval-document-executor-cross-tenant-scoping` (#2127). The v1 audit
//! shipped 6 findings as `#[ignore]`'d FINDINGS-ONLY tests; v2 promotes
//! F-1 from a "documented finding" into a LIVE pin: the broken shape
//! cannot drift without these tests failing at compile or runtime.
//!
//! Each pin is a **compile-time fn-pointer / closure bind** with the
//! exact tenant-blind signature documented in the v1 audit. When the
//! follow-up fix-brief
//! (`mission-document-executor-tenant-stamp` /
//! `mission-document-executor-trait-takes-tenant-id`) lands a
//! `tenant_id: TenantId` parameter, these tests will fail to compile
//! — that compile error is the trigger to delete the pin and un-ignore
//! the companion `post_fix_*` test below.
//!
//! Companion docs:
//! - v1 report: `docs/skycot/evals/scheduled/2026-05-23/eval-document-executor-cross-tenant-scoping.md`
//! - v2 delta:  `docs/skycot/evals/scheduled/2026-05-23/eval-document-executor-cross-tenant-scoping-v2.md`

use argo_executor_document::{
    DocExecutor, DocOutcome, DocRequest, DocResult, MsGraphExecutor, MsGraphFile, MsGraphFormat,
    OutputPath,
};
use std::path::Path;

// =========================================================================
// LIVE PIN — DocExecutor::run signature
// =========================================================================

/// **PIN F-1.a** — `DocExecutor::run` is the sync dispatch entry. Its
/// only inputs are `(&DocRequest, &Path)`; no tenant axis. We bind the
/// method to a typed fn-pointer so any addition of a `TenantId` arg
/// fails to compile.
#[test]
fn pin_doc_executor_run_signature_no_tenant_arg() {
    let _: fn(&DocExecutor, &DocRequest, &Path) -> DocResult<DocOutcome> = DocExecutor::run;
}

/// **PIN F-1.b** — `OutputPath::resolve(bundle, relative)` is the path
/// guard used by every `DocRequest` variant. It enforces `..` /
/// absolute-path rejection but takes no tenant axis: the bundle dir
/// chosen by the caller is the only isolation boundary.
#[test]
fn pin_output_path_resolve_signature_no_tenant_arg() {
    let _: fn(&Path, &str) -> DocResult<OutputPath> = OutputPath::resolve;
}

/// **PIN F-1.c** — `MsGraphExecutor::create` is async and takes
/// `(MsGraphFormat, &str, &[u8])`. Binding to a closure with the exact
/// signature forces a compile error if a `tenant_id: TenantId` arg is
/// inserted ahead of any existing parameter.
#[test]
fn pin_msgraph_executor_create_signature_no_tenant_arg() {
    // The closure is never invoked — its body exists only so the
    // compiler type-checks the `create` call against the signature
    // we are pinning. If a `TenantId` arg lands, this closure will
    // refuse to compile.
    let _check = |exec: &MsGraphExecutor, fmt: MsGraphFormat, path: &str, bytes: &[u8]| {
        let _fut = exec.create(fmt, path, bytes);
    };
}

/// **PIN F-1.d** — `MsGraphExecutor::read` likewise: `(self, &str)`.
/// Two surfaces (create + read) both lack the tenant axis; both must
/// be pinned so a partial fix (one but not the other) is visible.
#[test]
fn pin_msgraph_executor_read_signature_no_tenant_arg() {
    let _check = |exec: &MsGraphExecutor, path: &str| {
        let _fut = exec.read(path);
    };
}

// =========================================================================
// LIVE PIN — DocRequest / DocOutcome JSON shape
// =========================================================================

/// **PIN F-1.e** — `DocRequest::Markdown` JSON shape is
/// `{ kind, output_path, body }` — explicitly no `tenant_id`.
/// Enumerating the keys defends against silent additions (a future
/// `tenant_id` field on the enum variant would surface here as an
/// unexpected key).
#[test]
fn pin_doc_request_markdown_json_keys_have_no_tenant() {
    let req = DocRequest::Markdown {
        output_path: "report.md".into(),
        body: "hi".into(),
    };
    let json = serde_json::to_value(&req).expect("serialize");
    let obj = json.as_object().expect("object");
    let mut keys: Vec<&str> = obj.keys().map(|s| s.as_str()).collect();
    keys.sort_unstable();
    assert_eq!(keys, vec!["body", "kind", "output_path"]);
}

/// **PIN F-1.f** — `DocRequest::Pdf` JSON shape is
/// `{ kind, output_path, title, body }`. Same enumeration check.
#[test]
fn pin_doc_request_pdf_json_keys_have_no_tenant() {
    let req = DocRequest::Pdf {
        output_path: "out.pdf".into(),
        title: "Tenant A Q3".into(),
        body: "# Hi".into(),
    };
    let json = serde_json::to_value(&req).expect("serialize");
    let obj = json.as_object().expect("object");
    let mut keys: Vec<&str> = obj.keys().map(|s| s.as_str()).collect();
    keys.sort_unstable();
    assert_eq!(keys, vec!["body", "kind", "output_path", "title"]);
}

/// **PIN F-1.g** — `DocRequest::MsGraph` JSON shape is
/// `{ kind, output_path, format, payload_b64 }`. Same enumeration.
#[test]
fn pin_doc_request_msgraph_json_keys_have_no_tenant() {
    let req = DocRequest::MsGraph {
        output_path: "out.docx".into(),
        format: MsGraphFormat::UploadOnly,
        payload_b64: String::new(),
    };
    let json = serde_json::to_value(&req).expect("serialize");
    let obj = json.as_object().expect("object");
    let mut keys: Vec<&str> = obj.keys().map(|s| s.as_str()).collect();
    keys.sort_unstable();
    assert_eq!(keys, vec!["format", "kind", "output_path", "payload_b64"]);
}

/// **PIN F-1.h** — `DocOutcome` (the audit-ledger envelope) JSON shape
/// is `{ kind, output_path, bytes_written }`. No tenant tag means the
/// audit-record consumer cannot tell from `DocOutcome` alone which
/// tenant the artifact belongs to.
#[test]
fn pin_doc_outcome_json_keys_have_no_tenant() {
    let outcome = DocOutcome {
        kind: "markdown".into(),
        output_path: std::path::PathBuf::from("/tmp/bundle/report.md"),
        bytes_written: 4,
    };
    let json = serde_json::to_value(&outcome).expect("serialize");
    let obj = json.as_object().expect("object");
    let mut keys: Vec<&str> = obj.keys().map(|s| s.as_str()).collect();
    keys.sort_unstable();
    assert_eq!(keys, vec!["bytes_written", "kind", "output_path"]);
}

/// **PIN F-1.i** — `MsGraphFile` (the Drive-upload outcome) JSON shape
/// is `{ web_url, item_id, size_bytes }`. Same observation as
/// `DocOutcome`: no tenant axis on the outcome row.
#[test]
fn pin_msgraph_file_outcome_json_keys_have_no_tenant() {
    let file = MsGraphFile {
        web_url: "https://onedrive.live.com/x".into(),
        item_id: "0123".into(),
        size_bytes: 42,
    };
    let json = serde_json::to_value(&file).expect("serialize");
    let obj = json.as_object().expect("object");
    let mut keys: Vec<&str> = obj.keys().map(|s| s.as_str()).collect();
    keys.sort_unstable();
    assert_eq!(keys, vec!["item_id", "size_bytes", "web_url"]);
}

// =========================================================================
// LIVE PIN — DocExecutor::new is a no-arg constructor (no tenant in
// the executor's own state either)
// =========================================================================

/// **PIN F-1.j** — `DocExecutor::new()` takes zero args. The executor
/// is stateless; there is no per-tenant configuration to bind at
/// construction time.
#[test]
fn pin_doc_executor_new_signature_no_tenant_arg() {
    let _: fn() -> DocExecutor = DocExecutor::new;
}

// =========================================================================
// POST-FIX — #[ignore]'d, un-ignore when the fix-brief lands
// =========================================================================

/// **POST-FIX F-1** — companion to the pins above. Today the
/// `DocRequest::Markdown` JSON shape carries no tenant_id; once
/// `mission-document-executor-tenant-stamp` lands, every variant
/// should surface a `tenant_id` key (or the executor's surface should
/// otherwise advertise a tenant axis).
///
/// **Un-ignore when**: the fix-brief makes one of the following true:
/// (a) `DocRequest` variants carry a `tenant_id: TenantId` field, or
/// (b) a new `DocExecutor::run_for_tenant(tenant, &DocRequest, &Path)`
/// surface is added and the bare `run` is deprecated. At that point
/// the relevant pin test (`pin_doc_request_markdown_json_keys_have_no_tenant`
/// / `pin_doc_executor_run_signature_no_tenant_arg`) will fail and
/// THIS test will pass.
#[test]
#[ignore = "POST-FIX: un-ignore when mission-document-executor-tenant-stamp lands; see surface_contract_pin_f1.rs header"]
fn post_fix_doc_request_carries_tenant_axis() {
    let req = DocRequest::Markdown {
        output_path: "report.md".into(),
        body: "hi".into(),
    };
    let json = serde_json::to_value(&req).expect("serialize");
    let obj = json.as_object().expect("object");
    assert!(
        obj.contains_key("tenant_id"),
        "POST-FIX: DocRequest should serialise a tenant_id once the fix-brief lands"
    );
}
