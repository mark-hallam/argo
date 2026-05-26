//! Tests for `routes_workers_hire`'s Genesis-vertical lookup.
//!
//! Sibling to `routes_workers_hire_tests.rs`; keeps the parent file
//! under the 500 LOC ceiling. Covers the extension to `vertical_exists`
//! that lets the hire endpoint resolve verticals written under
//! `<verticals_root>/.genesis/<tenant_slug>/<vertical_id>/` by the SPA
//! `POST /api/v1/genesis` streaming handler. Brief:
//! `mission-genesis-spa-page-describe-and-watch`.

use super::tests::{env_lock, hire_req, mem_store, set_test_env, valid_draft};
use super::{hire_response, reset_idem_cache_for_tests};
use serde_json::{json, Value};

fn plant_genesis_vertical(verticals_root: &std::path::Path, tenant: &str, id: &str) {
    let dir = verticals_root.join(".genesis").join(tenant).join(id);
    std::fs::create_dir_all(&dir).expect("create genesis dir");
    std::fs::write(
        dir.join("vertical.toml"),
        format!(
            "schema_version = \"1\"\n\n[vertical]\nid = \"{id}\"\nname = \"Mariah\"\nversion = \"0.1.0\"\n"
        ),
    )
    .expect("write manifest");
}

#[tokio::test]
async fn hire_accepts_genesis_generated_vertical_under_dot_genesis() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let verticals_root = tmp.path().join("verticals");
    std::fs::create_dir_all(&verticals_root).expect("create verticals root");
    std::env::set_var("ARGO_VERTICALS_DIR", &verticals_root);
    plant_genesis_vertical(
        &verticals_root,
        "op-test",
        "customer-support-agent-mariah",
    );

    reset_idem_cache_for_tests();
    let store = mem_store().await;
    let mut draft = valid_draft();
    draft["suggestedVertical"] = json!("customer-support-agent-mariah");
    let body = json!({
        "worker_draft": draft,
        "problem": "Customer success lead who triages tier-1 tickets daily.",
        "answers": { "source": "genesis_stream" },
    })
    .to_string();
    let (status, body) = hire_response(&hire_req(&body), Some(&store)).await;
    assert_eq!(
        status, "201 Created",
        "expected hire to accept Genesis-generated vertical, body: {body}"
    );
}

#[tokio::test]
async fn hire_still_rejects_unknown_vertical_when_no_genesis_match() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let verticals_root = tmp.path().join("verticals");
    std::fs::create_dir_all(verticals_root.join(".genesis").join("op-test"))
        .expect("create empty tenant dir");
    std::env::set_var("ARGO_VERTICALS_DIR", &verticals_root);

    reset_idem_cache_for_tests();
    let store = mem_store().await;
    let mut draft = valid_draft();
    draft["suggestedVertical"] = json!("nope-not-here");
    let body = json!({
        "worker_draft": draft,
        "problem": "Customer success lead who triages tier-1 tickets daily.",
        "answers": {},
    })
    .to_string();
    let (status, body) = hire_response(&hire_req(&body), Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["error"]["code"], "bad_request");
    assert!(body.contains("unknown vertical"));
}
