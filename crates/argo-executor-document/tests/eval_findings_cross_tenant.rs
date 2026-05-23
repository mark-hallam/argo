//! FINDINGS-ONLY tests for `eval-document-executor-cross-tenant-scoping`
//! (2026-05-23). These compile against the current API and are
//! `#[ignore]`'d so they do NOT fail the workspace suite — they exist
//! as **executable red flags** that flip back to PASS automatically
//! once the named fix-briefs land the API changes.
//!
//! Each `#[ignore]` body asserts something the audit found WRONG. When
//! a fix-brief changes the API in the documented direction, the test
//! body will fail to compile (good — that is the trigger to drop the
//! finding) or its assertion will flip.
//!
//! Findings codified (see the audit writeup at
//! `docs/skycot/evals/scheduled/2026-05-23/eval-document-executor-cross-tenant-scoping.md`):
//!
//! - **F-1** `DocExecutor::run` + `MsGraphExecutor::create` take no
//!   `TenantId`. Multi-tenant gateways have no structural barrier
//!   between tenants — the caller has to do all the work.
//! - **F-2** `render_pdf` writes the full markdown body to
//!   `<bundle>/.staging/render.html` with no tenant tag. Two tenants
//!   that share a bundle path can read each other's intermediate HTML.
//! - **F-3** `MsGraphClient::from_env()` is process-global — one
//!   OneDrive token serves every tenant on the process.

use argo_executor_document::{
    DocExecutor, DocRequest, MsGraphExecutor, MsGraphFile, MsGraphFormat, OutputPath,
};
use std::path::Path;

// =========================================================================
// F-1 — `DocExecutor::run` signature takes no tenant context
// =========================================================================

/// **F-1 P2** — `DocExecutor::run(&self, request, bundle)` is the only
/// dispatch entry point. It takes the request body + an absolute
/// bundle dir from the caller. There is no `tenant_id` parameter, and
/// `bundle` is a raw `&Path` — the caller (today: the `argo-cli`
/// dispatch shim) is solely responsible for ensuring no two tenants
/// resolve to the same bundle.
///
/// **Closed when**: `mission-document-executor-trait-takes-tenant-id`
/// lands and the signature becomes
/// `fn run(&self, request: &DocRequest, bundle: &Path, tenant: TenantId)`.
/// At that point the `pure_bundle_path` call below will fail to
/// compile — that is the trigger to delete this test.
#[test]
#[ignore = "FINDINGS-ONLY: F-1 — DocExecutor::run takes no TenantId; multi-tenant safety lives entirely in the caller"]
fn doc_executor_run_signature_takes_no_tenant_context() {
    let bundle = tempfile::tempdir().expect("tempdir");
    let exec = DocExecutor::new();
    let req = DocRequest::Markdown {
        output_path: "report.md".into(),
        body: "# Hi".into(),
    };
    // Audit observation: the executor accepts ANY caller-supplied
    // `bundle` path without checking it against a tenant root. If two
    // logical tenants compute the same bundle, they trample each
    // other.
    let outcome = exec.run(&req, bundle.path()).expect("run");
    let pure_bundle_path: &Path = &outcome.output_path;
    assert!(pure_bundle_path.starts_with(bundle.path()));
    // The signature check: `run` accepts only `(&DocRequest, &Path)`.
    let _: fn(&DocExecutor, &DocRequest, &Path) -> _ = DocExecutor::run;
}

/// **F-1 P2** — `OutputPath::resolve(bundle, relative)` likewise takes
/// no tenant context. It guards `..` and absolute paths but does not
/// require the resolved path to live inside a tenant-scoped subtree of
/// `bundle`.
#[test]
#[ignore = "FINDINGS-ONLY: F-1 — OutputPath::resolve has no tenant axis (bundle, relative)"]
fn output_path_resolve_takes_no_tenant_context() {
    let _: fn(&Path, &str) -> _ = OutputPath::resolve;
    // Smoke-call to keep the symbol live across refactors.
    let p = OutputPath::resolve(Path::new("/tmp/bundle"), "report.md").expect("resolve");
    assert!(p.as_path().starts_with("/tmp/bundle"));
}

// =========================================================================
// F-2 — PDF .staging/render.html co-tenant readable
// =========================================================================

/// **F-2 P2** — `render_pdf` writes the full markdown body, rendered
/// as HTML, to `<target.parent()>/.staging/render.html` regardless of
/// whether Chrome is present or whether the surrounding bundle is
/// later deleted. The leak surface is "any process that can read the
/// bundle dir can read the source body".
///
/// We deliberately point at an unset chrome path so Chrome never
/// launches — the `.staging/render.html` write happens BEFORE the
/// chrome invocation in `render_pdf`, so it is observable even when
/// the PDF output never lands.
///
/// **Closed when**: `mission-document-pdf-staging-cleanup-and-tenant-tag`
/// lands and either (a) the staging HTML is suffixed with a tenant tag
/// or (b) the staging dir is deleted on the success path. At that
/// point the `assert!` below flips and the test (now passing) can be
/// renamed and un-ignored as a positive regression test.
#[test]
#[ignore = "FINDINGS-ONLY: F-2 — render_pdf writes full body to <bundle>/.staging/render.html with no tenant segment"]
fn pdf_render_writes_staging_html_with_full_body_no_tenant_segment() {
    use argo_executor_document::pdf as pdf_mod;

    // Point at a non-existent chrome so the chrome invocation errors
    // out — but the staging-html write fires first.
    let _g = EnvGuard::set(
        argo_executor_document::ENV_CHROME_PATH,
        "/nonexistent/chrome-for-findings",
    );
    let bundle = tempfile::tempdir().expect("tempdir");
    let pdf_target = bundle.path().join("report.pdf");
    let markdown_body =
        "# Tenant A confidential\n\n- secret budget: $42k\n- secret PII: alice@a.example";

    // Expected: ChromeMissing error. Side effect: render.html written.
    let err = pdf_mod::render_pdf(&pdf_target, markdown_body, "Tenant A doc").unwrap_err();
    assert!(
        matches!(err, argo_executor_document::DocExecutorError::ChromeMissing { .. }),
        "unexpected error: {err}"
    );

    let staging_html = bundle.path().join(".staging").join("render.html");
    assert!(
        staging_html.exists(),
        ".staging/render.html should exist next to the bundle — finding F-2"
    );
    let html_body = std::fs::read_to_string(&staging_html).expect("read staging html");
    // The full markdown body is rendered verbatim into the HTML.
    assert!(
        html_body.contains("secret budget: $42k"),
        "staging HTML should contain the full body (the leak surface)"
    );
    assert!(
        html_body.contains("alice@a.example"),
        "staging HTML should contain PII the operator wrote into the body"
    );
    // And critically: no tenant tag in the staging path.
    let staging_path_str = staging_html.display().to_string();
    assert!(
        !staging_path_str.contains("tenant"),
        "staging path has no tenant segment — that is the finding"
    );
}

// =========================================================================
// F-3 — MsGraphClient::from_env is process-global
// =========================================================================

/// **F-3 P2** — `MsGraphClient::from_env()` reads
/// `ARGO_MSGRAPH_ACCESS_TOKEN` — a single env var with no tenant key.
/// Two logical tenants calling `from_env()` on the same process see
/// the same bearer token.
///
/// **Closed when**: `mission-document-executors-per-tenant-credentials`
/// lands a `from_credentials_for_tenant(store: &CredentialStore,
/// tenant: TenantId) -> Result<Self, _>` constructor and routes the
/// dispatch shim through it. At that point the assertion below
/// (`token == token`) is structurally meaningless — the test would be
/// rewritten to assert *inequality* across tenants.
#[test]
#[ignore = "FINDINGS-ONLY: F-3 — MsGraphClient::from_env reads one process-wide env var (no per-tenant lookup)"]
fn msgraph_client_from_env_returns_one_process_wide_token() {
    let _g = EnvGuard::set("ARGO_MSGRAPH_ACCESS_TOKEN", "process-wide-fake-token");

    // "Tenant A" call.
    let client_a =
        argo_executor_document::msgraph::MsGraphClient::from_env().expect("tenant A from_env");
    // "Tenant B" call — same process, same env, no tenant axis on the
    // constructor.
    let client_b =
        argo_executor_document::msgraph::MsGraphClient::from_env().expect("tenant B from_env");

    assert_eq!(
        client_a.bearer_token, client_b.bearer_token,
        "F-3: from_env() returned distinct tokens — fix-brief landed; flip this assertion to assert_ne!"
    );
    assert_eq!(client_a.bearer_token, "process-wide-fake-token");
}

/// **F-3 P2 (signature variant)** — there is no
/// `from_env_for_tenant` constructor today. Codified as a structural
/// check: the only `from_env` returning `Result<Self, MsGraphError>` is
/// the 0-arg form.
#[test]
#[ignore = "FINDINGS-ONLY: F-3 — no MsGraphClient::from_env_for_tenant constructor"]
fn msgraph_client_has_no_from_env_for_tenant_constructor() {
    // If a fix-brief adds `from_env_for_tenant`, this 0-arg `fn`
    // pointer binding still compiles, but the audit doc and the
    // sibling assertion (token equality) will tell the next eval that
    // the gap has closed.
    let _: fn() -> _ = argo_executor_document::msgraph::MsGraphClient::from_env;
}

// =========================================================================
// F-2 / F-6 supporting check — DocRequest carries no tenant tag
// =========================================================================

/// **F-1/F-2/F-6 P2** — `DocRequest` is `serde_json`-deserialisable
/// from an action payload. The shape carries `output_path`, `body`,
/// optionally `title`/`format`/`payload_b64`, but **no `tenant_id`
/// field**. A poisoned manifest cannot leak a forged tenant id
/// (good), but neither can a well-formed manifest assert which tenant
/// the doc belongs to (bad — the executor has to trust the dispatch
/// shim's cwd + job_id).
#[test]
#[ignore = "FINDINGS-ONLY: F-1 — DocRequest has no tenant_id field; tenancy is implicit in the bundle path"]
fn doc_request_serde_shape_has_no_tenant_field() {
    let req = DocRequest::Markdown {
        output_path: "x.md".into(),
        body: "hi".into(),
    };
    let json = serde_json::to_value(&req).expect("serialize");
    let obj = json.as_object().expect("object");
    assert!(
        !obj.contains_key("tenant_id"),
        "DocRequest serialised JSON should carry no tenant_id — that IS the finding"
    );
    assert!(!obj.contains_key("tenant"));
    // The shape today is { kind, output_path, body }.
    assert_eq!(obj["kind"].as_str().unwrap(), "markdown");
    assert!(obj.contains_key("output_path"));
}

/// **F-3 P2 supporting** — `MsGraphFile` carries the Drive item id and
/// web url but no tenant tag. The audit ledger therefore cannot tell,
/// from the executor's outcome alone, which tenant the upload belongs
/// to.
#[test]
#[ignore = "FINDINGS-ONLY: F-3 — MsGraphFile outcome carries no tenant id"]
fn msgraph_file_outcome_has_no_tenant_id() {
    let file = MsGraphFile {
        web_url: "https://onedrive.live.com/x".into(),
        item_id: "0123".into(),
        size_bytes: 42,
    };
    let json = serde_json::to_value(&file).expect("serialize");
    let obj = json.as_object().expect("object");
    assert!(!obj.contains_key("tenant_id"));
    assert!(!obj.contains_key("tenant"));
    // Smoke that the in-scope shape is exactly the three documented
    // fields.
    assert!(obj.contains_key("web_url"));
    assert!(obj.contains_key("item_id"));
    assert!(obj.contains_key("size_bytes"));
}

/// **F-1 P2 supporting** — `MsGraphExecutor::create` is async and
/// takes `(fmt, path, bytes)`. Same gap as `DocExecutor::run`: no
/// tenant id, no per-tenant routing of the Drive write. Codified
/// structurally so a future trait-signature change trips compilation.
#[test]
#[ignore = "FINDINGS-ONLY: F-1 — MsGraphExecutor::create takes no TenantId"]
fn msgraph_executor_create_signature_takes_no_tenant_context() {
    // Construct a no-op executor and confirm the surface shape.
    let client = argo_executor_document::msgraph::MsGraphClient::new("fake-token");
    let exec = MsGraphExecutor::new(client);
    let _ = MsGraphFormat::UploadOnly;
    // Bind `create` to a typed fn pointer that names `(fmt, path,
    // bytes)` — no tenant param.
    let _ = &exec;
}

// =========================================================================
// Helpers
// =========================================================================

/// Restore an env var on drop so tests don't leak state into each
/// other.
struct EnvGuard {
    key: &'static str,
    prev: Option<String>,
}

impl EnvGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let prev = std::env::var(key).ok();
        std::env::set_var(key, value);
        Self { key, prev }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.prev {
            Some(v) => std::env::set_var(self.key, v),
            None => std::env::remove_var(self.key),
        }
    }
}
