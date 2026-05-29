//! Tests for `eval_export_training_data`. Seeds an in-memory store via
//! the canonical demo seed (8 approvals — 3 decided — + 5 committed
//! retrains), then exercises the collectors + JSONL writer directly.

use super::*;
use argo_demo_seed::{run_canonical_v1_seed, SeedOptions};
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

#[tokio::test]
async fn exports_approval_and_retrain_examples_as_jsonl() {
    let (store, tenant) = seeded().await;

    // 3 of the 8 seeded approvals are decided (2 granted + 1 denied).
    let mut examples = collect_approval_examples(&store, tenant).await.unwrap();
    let from_approvals = examples.len();
    assert_eq!(from_approvals, 3, "decided approvals");
    assert!(examples
        .iter()
        .all(|e| e.source == "human_approval" && !e.completion.is_empty()));
    assert!(examples.iter().any(|e| e.completion == "approve"));
    assert!(examples.iter().any(|e| e.completion == "reject"));

    // 5 committed retrains, one per classification, each worker-bound.
    let from_retrains = append_retrain_examples(&store, tenant, &mut examples)
        .await
        .unwrap();
    assert_eq!(from_retrains, 5, "committed retrains");
    assert_eq!(examples.len(), 8);
    assert!(examples
        .iter()
        .any(|e| e.source == "human_correction" && e.worker_id.is_some()));

    // Writes well-formed JSONL with the SFT-ish shape.
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let (written, _redacted) = write_jsonl(tmp.path(), examples).unwrap();
    assert_eq!(written, 8);
    let body = std::fs::read_to_string(tmp.path()).unwrap();
    let lines: Vec<&str> = body.lines().collect();
    assert_eq!(lines.len(), 8, "one JSON object per line");
    for line in lines {
        let v: serde_json::Value = serde_json::from_str(line).expect("valid json line");
        for key in ["source", "prompt", "completion", "label"] {
            assert!(v.get(key).is_some(), "line missing `{key}`: {line}");
        }
    }
}

#[tokio::test]
async fn export_is_tenant_scoped() {
    let (store, _tenant) = seeded().await;
    // A DIFFERENT tenant sees none of the seeded judged history.
    let other = TenantId::new();
    let mut examples = collect_approval_examples(&store, other).await.unwrap();
    assert_eq!(examples.len(), 0, "other tenant sees no decided approvals");
    let from_retrains = append_retrain_examples(&store, other, &mut examples)
        .await
        .unwrap();
    assert_eq!(from_retrains, 0, "other tenant sees no retrains");
}

#[tokio::test]
async fn invalid_tenant_id_is_rejected() {
    assert!(parse_tenant("not-a-uuid").is_err());
    assert!(parse_tenant(&TenantId::new().to_string()).is_ok());
}
