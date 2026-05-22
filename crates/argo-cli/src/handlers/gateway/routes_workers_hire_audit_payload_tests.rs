//! Regression tests for the `WorkerHired` audit-payload shape +
//! PII redaction in the paired `HttpRequest` row + cross-tenant
//! audit-row isolation when drives go through `POST /api/v1/workers`.
//!
//! Sourced from `mission-hire-audit-payload-shape-validation`
//! (evaluator round-2 P1 — `docs/skycot/evals/scheduled/2026-05-22/
//! teammate-hire-five-distinct-personas.md`):
//!
//! > "Verify the WorkerHired row actually contains the un-redacted
//! >  worker_id, redacted problem text (no PII), and actor principal_id.
//! >  This eval did not inspect the row directly — sqlite3 doesn't work
//! >  (SQLCipher) and there's no admin endpoint to read raw
//! >  audit_record rows."
//!
//! The hire handler in `routes_workers_hire.rs` writes two audit rows
//! per successful hire:
//!   1. `AuditKind::HttpRequest` — carries the (redacted) `problem`
//!      text plus the full hire context.
//!   2. `AuditKind::WorkerHired` — the named domain event, carrying
//!      `worker_id`, `name`, `actor`, `tenant_id`, `vertical_id`,
//!      `duty_count`, `hired_at`.
//!
//! These tests pin the shape of those rows so a future refactor that
//! drops a field or skips redaction surfaces here loudly. The
//! `Auditable` impl on the hire handler is out of brief scope — these
//! tests assert the EXISTING contract.

#![allow(clippy::too_many_lines)]

use super::tests::{env_lock, hire_req, mem_store, set_test_env, valid_body};
use super::{hire_response, reset_idem_cache_for_tests};
use argo_store::SessionStore;
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

/// Same hire request shape as `hire_req`, but stamps an
/// `X-Argo-Principal` header so the audit row's `actor` /
/// `principal_id` column carries a known value the test can assert
/// on. Anonymous principals would land NULL — defeating the "actor
/// is present + populated" assertion the brief demands.
fn hire_req_with_principal(body: &str, principal: &str) -> String {
    format!(
        "POST /api/v1/workers HTTP/1.1\r\nHost: x\r\nX-Argo-Principal: {principal}\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

/// The PII-rich problem string from the brief. Lifted verbatim so the
/// assertions below can quote the specific tokens that must not leak.
const PII_PROBLEM: &str =
    "Hire me a marketing manager. My email is alice@acmeco.com and \
     my phone is 555-555-0123 and I work at ACME Corp at 123 Main St, San Francisco.";

fn body_with_problem(problem: &str) -> String {
    let mut v: Value = serde_json::from_str(&valid_body()).expect("valid_body json");
    v["problem"] = json!(problem);
    v.to_string()
}

async fn fetch_one_payload(store: &SessionStore, kind: &str) -> Value {
    let raw: String = sqlx::query_scalar(
        "SELECT payload_json FROM audit_record \
         WHERE kind = ? \
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(kind)
    .fetch_one(store.pool())
    .await
    .expect("audit row present");
    serde_json::from_str(&raw).expect("payload parses as json")
}

/// The WorkerHired domain row contains every field the eval needs to
/// audit: un-redacted `worker_id` (canonical UUID), `name` (persona),
/// `actor` (resolved principal), `tenant_id` (stamped by the sink),
/// `vertical_id`, `duty_count`. A future regression that drops a
/// field — or quietly renames `actor`→`principal` — breaks here
/// before it ships.
#[tokio::test]
async fn worker_hired_audit_payload_has_complete_shape() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    reset_idem_cache_for_tests();
    let store = mem_store().await;

    let principal = "human:alice@acmeco.com";
    let (status, response_body) =
        hire_response(&hire_req_with_principal(&valid_body(), principal), Some(&store)).await;
    assert_eq!(status, "201 Created", "hire failed: {response_body}");
    let expected_worker_id = serde_json::from_str::<Value>(&response_body).expect("resp json")
        ["worker_id"]
        .as_str()
        .expect("worker_id in response")
        .to_string();

    let payload = fetch_one_payload(&store, "worker.hired").await;

    // worker_id: present, non-empty, canonical UUID, matches handler return.
    let worker_id = payload["worker_id"].as_str().expect("worker_id field");
    Uuid::parse_str(worker_id).expect("worker_id parses as a UUID — must NOT be redacted");
    assert_eq!(worker_id, expected_worker_id, "audit worker_id != response");

    // name (persona): matches the draft's suggestedName.
    assert_eq!(
        payload["name"].as_str(),
        Some("Maya"),
        "persona name missing or wrong on WorkerHired row: {payload}",
    );

    // actor: resolved principal, NOT null, NOT empty.
    let actor = payload["actor"].as_str().expect("actor field present");
    assert!(!actor.is_empty(), "actor field empty on WorkerHired row");
    assert_eq!(
        actor, principal,
        "actor must echo the resolved X-Argo-Principal value",
    );

    // tenant_id stamped by the sink — present + non-empty, parseable UUID.
    let tenant_id = payload["tenant_id"]
        .as_str()
        .expect("tenant_id stamped on payload");
    Uuid::parse_str(tenant_id).expect("tenant_id is a canonical UUID");

    // vertical_id + duty_count carry-through from the draft.
    assert_eq!(payload["vertical_id"].as_str(), Some("gtm-engineer"));
    assert_eq!(payload["duty_count"].as_i64(), Some(1));

    // hired_at: RFC3339 timestamp, non-empty.
    assert!(
        payload["hired_at"]
            .as_str()
            .is_some_and(|s| !s.is_empty()),
        "hired_at missing or empty: {payload}",
    );
}

/// The paired HttpRequest row carries the (redacted) problem text. The
/// hire handler's `redact_text` rewrites email addresses to `[EMAIL]`
/// and US-standard-format phones (3-3-4 with `-`/`.`/space separators)
/// to `[PHONE]`. This pins that contract — a future change to the
/// redactor that breaks `[EMAIL]`/`[PHONE]` sentinels would surface
/// here.
#[tokio::test]
async fn hire_audit_redacts_email_and_us_standard_phone_in_problem_text() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    reset_idem_cache_for_tests();
    let store = mem_store().await;

    let body = body_with_problem(PII_PROBLEM);
    let (status, _body) = hire_response(&hire_req(&body), Some(&store)).await;
    assert_eq!(status, "201 Created");

    let payload = fetch_one_payload(&store, "http.request").await;
    let redacted_problem = payload["problem"]
        .as_str()
        .expect("problem field present on HttpRequest row");

    // Raw PII tokens must not be present anywhere in the redacted
    // problem text — the load-bearing assertions for the brief.
    assert!(
        !redacted_problem.contains("alice@acmeco.com"),
        "raw email leaked into audit row: {redacted_problem}",
    );
    assert!(
        !redacted_problem.contains("555-555-0123"),
        "raw US phone leaked into audit row: {redacted_problem}",
    );

    // Redaction markers must be present so consumers can tell
    // "redacted token" apart from "no PII in the input".
    assert!(
        redacted_problem.contains("[EMAIL]"),
        "expected [EMAIL] redaction marker: {redacted_problem}",
    );
    assert!(
        redacted_problem.contains("[PHONE]"),
        "expected [PHONE] redaction marker: {redacted_problem}",
    );
}

/// International-prefix phones like `+1-555-0123` (1-3-4 grouping)
/// must redact alongside the strict 3-3-4 US format. Originally shipped
/// as `#[ignore]` per #2067's FINDINGS-ONLY handling; un-ignored by
/// `argo-security-pii-detect-broaden-phone-and-address` after the hire
/// handler started delegating to `argo_redact::redact_text` (which now
/// matches the `+CC` shape via the broadened phone regex).
#[tokio::test]
async fn findings_hire_audit_should_redact_international_phone() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    reset_idem_cache_for_tests();
    let store = mem_store().await;

    let problem = "Reach me at +1-555-0123 if anything blocks the hire.";
    let body = body_with_problem(problem);
    let (status, _body) = hire_response(&hire_req(&body), Some(&store)).await;
    assert_eq!(status, "201 Created");

    let payload = fetch_one_payload(&store, "http.request").await;
    let redacted = payload["problem"].as_str().unwrap_or("");
    assert!(
        !redacted.contains("+1-555-0123") && !redacted.contains("555-0123"),
        "international-format phone leaked into audit row: {redacted}",
    );
}

/// Street addresses (`123 Main St`) must redact alongside email and
/// phone. Originally shipped as `#[ignore]` per #2067's FINDINGS-ONLY
/// handling; un-ignored once `argo_redact::Category::Address` and its
/// `[ADDR]` sentinel landed and the hire handler delegated to the
/// canonical redactor.
#[tokio::test]
async fn findings_hire_audit_should_redact_street_address() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    reset_idem_cache_for_tests();
    let store = mem_store().await;

    let problem = "We're at 123 Main St, San Francisco — please ship the welcome kit.";
    let body = body_with_problem(problem);
    let (status, _body) = hire_response(&hire_req(&body), Some(&store)).await;
    assert_eq!(status, "201 Created");

    let payload = fetch_one_payload(&store, "http.request").await;
    let redacted = payload["problem"].as_str().unwrap_or("");
    assert!(
        !redacted.contains("123 Main St"),
        "raw street address leaked into audit row: {redacted}",
    );
}

/// Cross-tenant isolation: when two hires are driven under two
/// different workspaces (the hire handler's tenant binding is
/// `ensure_workspace(cwd_basename, cwd_path)`), each hire's
/// `WorkerHired` and `HttpRequest` rows MUST stamp the driving
/// tenant's tenant_id — never the other workspace's. A regression
/// that, say, cached the first tenant in a process-global would
/// surface as a mismatch in the column or the payload-stamped
/// `tenant_id`.
///
/// Setup mirrors the cwd-binding the handler uses in production:
/// we `set_current_dir` to a per-tenant tempdir before each hire,
/// then restore the original cwd before the test returns so other
/// tests sharing the env_lock keep their cwd assumptions intact.
#[tokio::test]
async fn hire_audit_rows_isolate_across_workspaces() {
    let _g = env_lock().await;
    let original_cwd = std::env::current_dir().expect("read cwd");
    let env_tmp = tempfile::tempdir().expect("env tmp");
    set_test_env(env_tmp.path());
    reset_idem_cache_for_tests();
    let store = mem_store().await;

    let dir_a = tempfile::tempdir().expect("dir a");
    let dir_b = tempfile::tempdir().expect("dir b");

    // Always restore cwd, regardless of assertion outcome.
    let restore = |cwd: &std::path::Path| {
        let _ = std::env::set_current_dir(cwd);
    };

    // -- Tenant A hire ----------------------------------------------------
    std::env::set_current_dir(dir_a.path()).expect("chdir a");
    // On macOS, `/tmp` symlinks to `/private/tmp` — `current_dir()`
    // returns the resolved path, which is what the handler stamps into
    // the workspace row. Capture the post-`set_current_dir` value so
    // the post-hoc tenant lookup uses the same path the handler did.
    let resolved_a = std::env::current_dir().expect("cwd after chdir a");
    let (status_a, resp_a) = hire_response(&hire_req(&valid_body()), Some(&store)).await;
    if status_a != "201 Created" {
        restore(&original_cwd);
        panic!("tenant_a hire failed: {resp_a}");
    }
    let worker_a = serde_json::from_str::<Value>(&resp_a).expect("resp a")["worker_id"]
        .as_str()
        .expect("worker_id a")
        .to_string();
    let tenant_a = lookup_tenant_for_root(&store, &resolved_a).await;

    // -- Tenant B hire ----------------------------------------------------
    std::env::set_current_dir(dir_b.path()).expect("chdir b");
    let resolved_b = std::env::current_dir().expect("cwd after chdir b");
    let (status_b, resp_b) = hire_response(&hire_req(&valid_body()), Some(&store)).await;
    if status_b != "201 Created" {
        restore(&original_cwd);
        panic!("tenant_b hire failed: {resp_b}");
    }
    let worker_b = serde_json::from_str::<Value>(&resp_b).expect("resp b")["worker_id"]
        .as_str()
        .expect("worker_id b")
        .to_string();
    let tenant_b = lookup_tenant_for_root(&store, &resolved_b).await;

    restore(&original_cwd);

    assert_ne!(
        tenant_a, tenant_b,
        "two distinct workspaces must derive distinct tenant_ids",
    );

    // Pull every WorkerHired row and verify each row's tenant_id
    // column + payload-stamped tenant_id agree, and that each
    // workspace's hire landed under its own tenant — no leakage.
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT tenant_id, payload_json FROM audit_record \
         WHERE kind = 'worker.hired' \
         ORDER BY created_at ASC",
    )
    .fetch_all(store.pool())
    .await
    .expect("query worker.hired rows");
    assert_eq!(rows.len(), 2, "expected exactly 2 WorkerHired rows");

    let (col_tenant_a, payload_a_raw) = &rows[0];
    let (col_tenant_b, payload_b_raw) = &rows[1];
    let payload_a: Value = serde_json::from_str(payload_a_raw).expect("payload a json");
    let payload_b: Value = serde_json::from_str(payload_b_raw).expect("payload b json");

    assert_eq!(col_tenant_a, &tenant_a, "row 1 column tenant_id mismatch");
    assert_eq!(col_tenant_b, &tenant_b, "row 2 column tenant_id mismatch");
    assert_eq!(
        payload_a["tenant_id"].as_str().unwrap_or_default(),
        tenant_a,
        "row 1 payload tenant_id mismatch",
    );
    assert_eq!(
        payload_b["tenant_id"].as_str().unwrap_or_default(),
        tenant_b,
        "row 2 payload tenant_id mismatch",
    );

    assert_eq!(payload_a["worker_id"].as_str(), Some(worker_a.as_str()));
    assert_eq!(payload_b["worker_id"].as_str(), Some(worker_b.as_str()));

    // The load-bearing negative: tenant_a's row's tenant_id is NOT
    // tenant_b — and vice versa. A regression that wires either side
    // through a shared mutable would collide here.
    assert_ne!(col_tenant_a, &tenant_b, "tenant_b leaked onto tenant_a row");
    assert_ne!(col_tenant_b, &tenant_a, "tenant_a leaked onto tenant_b row");
}

async fn lookup_tenant_for_root(store: &Arc<SessionStore>, root: &std::path::Path) -> String {
    let name = root
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root_str = root.to_string_lossy();
    let (tenant, _ws) = store
        .ensure_workspace(name, &root_str)
        .await
        .expect("ensure_workspace");
    tenant.to_string()
}
