//! Tests for `eval_generate_cases`. Seeds an in-memory store via the
//! canonical demo seed (9 decided approvals — 2 rejected — + 5
//! committed retrains), reads the judged history through the shared
//! reader, generates cases, and asserts the JSONL is non-empty,
//! well-formed, tenant-scoped, PII-safe, and loads via the real
//! `argo_eval::load_jsonl` harness loader.
use super::*;
use argo_demo_seed::{run_canonical_v1_seed, SeedOptions};
use argo_eval::load_jsonl;
use argo_store::SessionStore;
use argo_types::TenantId;
use std::sync::Arc;

async fn seeded() -> (Arc<SessionStore>, TenantId) {
    let store = SessionStore::open(":memory:").await.expect("open mem db");
    store.migrate().await.expect("migrate");
    let store = Arc::new(store);
    let tenant = TenantId::new();
    let mut opts = SeedOptions::canonical_v1(tenant);
    opts.skip_pass_a_turns = true; // no provider key needed
    opts.dressing_audit_rows = 0;
    run_canonical_v1_seed(&store, &opts).await.expect("seed");
    (store, tenant)
}

/// Read + transform path the handler runs, sans the live-store open.
async fn inputs_for(store: &SessionStore, tenant: TenantId) -> (JudgedInputs, usize) {
    let approvals = jh::decided_approvals(store, tenant).await.unwrap();
    let retrains = jh::committed_retrains(store, tenant).await.unwrap();
    let mut redacted = 0usize;
    let inputs = build_inputs(approvals, retrains, &mut redacted);
    (inputs, redacted)
}

#[tokio::test]
async fn generates_cases_from_rejected_approvals_and_retrains() {
    let (store, tenant) = seeded().await;
    let (inputs, _redacted) = inputs_for(&store, tenant).await;

    // Of the 9 decided approvals, exactly 2 are rejections (the
    // above-ceiling discount + the warm rejection email held to revise
    // wording). The canonical seed's own tests assert the 7-granted /
    // 2-denied split among the 9 decided.
    assert_eq!(inputs.rejected.len(), 2, "two rejected approvals");
    // All 5 committed retrains become correction inputs.
    assert_eq!(inputs.corrections.len(), 5, "five committed retrains");

    let cases = generate_cases(&inputs);
    // 2 rejected + 5 retrains (every seeded classification carries
    // recoverable text) = 7 cases.
    assert_eq!(cases.len(), 7, "one case per usable judged event");
    assert!(cases.iter().any(|c| c.name.starts_with("judged/rejected-action-")));
    assert!(cases
        .iter()
        .any(|c| c.name.starts_with("judged/correction-")));
}

#[tokio::test]
async fn generated_jsonl_is_well_formed_and_loads_via_load_jsonl() {
    let (store, tenant) = seeded().await;
    let (inputs, _redacted) = inputs_for(&store, tenant).await;
    let cases = generate_cases(&inputs);
    let lines = serialize(&cases).unwrap();

    let tmp = tempfile::NamedTempFile::new().unwrap();
    let written = jh::write_lines(tmp.path(), &lines).unwrap();
    assert_eq!(written, cases.len());
    assert!(written > 0, "non-empty");

    // Every line is a JSON object with the eval-task keys.
    let body = std::fs::read_to_string(tmp.path()).unwrap();
    let line_count = body.lines().filter(|l| !l.trim().is_empty()).count();
    assert_eq!(line_count, written, "one JSON object per line");
    for line in body.lines().filter(|l| !l.trim().is_empty()) {
        let v: serde_json::Value = serde_json::from_str(line).expect("valid json line");
        for key in ["name", "input", "expected", "metric"] {
            assert!(v.get(key).is_some(), "line missing `{key}`: {line}");
        }
    }

    // Loads through the real harness loader → guaranteed runnable.
    let loaded = load_jsonl(tmp.path()).await.expect("load_jsonl parses generated cases");
    assert_eq!(loaded.len(), written);
}

#[tokio::test]
async fn generation_is_tenant_scoped() {
    let (store, _tenant) = seeded().await;
    // A different tenant sees none of the seeded judged history.
    let other = TenantId::new();
    let (inputs, redacted) = inputs_for(&store, other).await;
    assert!(inputs.rejected.is_empty(), "other tenant: no rejected approvals");
    assert!(inputs.corrections.is_empty(), "other tenant: no retrains");
    assert_eq!(redacted, 0);
    assert!(
        generate_cases(&inputs).is_empty(),
        "other tenant generates no cases"
    );
}

#[tokio::test]
async fn pii_in_a_rejected_draft_is_redacted_before_the_case() {
    // Inject an approval whose draft carries an email address, decide
    // it as a rejection via the canonical store API (so the id-bytes
    // conversion is handled), then confirm the generated case carries
    // the redaction, not the raw PII.
    use argo_store::approvals::{
        create_request, decide_request, ApprovalDecision, DefaultDecision, NewApprovalRequest,
    };
    let (store, tenant) = seeded().await;
    std::env::set_var("ARGO_PII_AUTOREDACT", "1");

    let action_id = uuid::Uuid::now_v7().as_bytes().to_vec();
    let req = NewApprovalRequest {
        action_id,
        tenant_id: tenant,
        requested_by: "tester",
        deadline_at: chrono::Utc::now().timestamp_micros(),
        default_decision: DefaultDecision::Deny,
        draft_content: Some("Email the customer at alice@example.com about the refund"),
    };
    let approval_id = create_request(store.pool(), &req).await.unwrap();
    // Reject it so it becomes a rejected-action case.
    let flipped = decide_request(
        store.pool(),
        &approval_id,
        tenant,
        "tester",
        ApprovalDecision::Reject,
    )
    .await
    .unwrap();
    assert!(flipped, "the injected approval was decided");

    let (inputs, redacted) = inputs_for(&store, tenant).await;
    assert!(redacted >= 1, "at least the injected email was redacted");
    let cases = generate_cases(&inputs);
    let joined: String = cases.iter().map(|c| c.input.clone()).collect();
    assert!(
        !joined.contains("alice@example.com"),
        "raw PII must not survive into a generated case"
    );
    std::env::remove_var("ARGO_PII_AUTOREDACT");
}
