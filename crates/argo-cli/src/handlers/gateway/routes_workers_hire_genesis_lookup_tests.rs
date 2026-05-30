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

/// Plant a thin Genesis vertical (manifest only, no eval suite) — the kind
/// the Genesis hire gate must REFUSE to activate.
fn plant_thin_genesis_vertical(verticals_root: &std::path::Path, tenant: &str, id: &str) {
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

/// Plant a FULLY-generated Genesis vertical (real manifest + prompts + eval
/// seeds) via the pipeline writer, under `.genesis/<tenant>/<id>/`.
fn plant_full_genesis_vertical(verticals_root: &std::path::Path, tenant: &str, role: &str, persona: &str) -> String {
    let p = argo_genesis::plan(role, persona).expect("plan builds");
    let tenant_dir = verticals_root.join(".genesis").join(tenant);
    std::fs::create_dir_all(&tenant_dir).expect("create tenant dir");
    let out = tenant_dir.join(&p.vertical_id);
    argo_genesis::write_vertical(&p, &out).expect("write vertical");
    p.vertical_id
}

#[tokio::test]
async fn hire_gate_blocks_a_thin_genesis_vertical_with_no_eval_suite() {
    // DEC-025 / Genesis P0: a Genesis teammate with no starter eval suite is
    // a thin worker — the hire gate must HARD-block it (422), never 201.
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let verticals_root = tmp.path().join("verticals");
    std::fs::create_dir_all(&verticals_root).expect("create verticals root");
    std::env::set_var("ARGO_VERTICALS_DIR", &verticals_root);
    std::env::set_var("ARGO_GENESIS_LLM_PROVIDER", "definitely-not-a-real-provider-xyz");
    plant_thin_genesis_vertical(&verticals_root, "op-test", "customer-support-agent-mariah");

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
    std::env::remove_var("ARGO_GENESIS_LLM_PROVIDER");
    assert_eq!(
        status, "422 Unprocessable Entity",
        "thin genesis teammate must be gate-blocked, not hired; body: {body}"
    );
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["error"]["code"], "genesis_gate_failed");
    assert_eq!(v["gate"]["state"], "blocked", "no eval suite → hard blocked");
    assert_eq!(v["gate"]["hireable"], false);
}

#[tokio::test]
async fn hire_gate_holds_a_full_genesis_vertical_for_review_without_a_live_judge() {
    // A fully-generated teammate clears the deterministic eval floor but,
    // with no live safety judge, the gate degrades to needs_review and the
    // hire is HELD (422) — never auto-activated, never a silent pass.
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let verticals_root = tmp.path().join("verticals");
    std::fs::create_dir_all(&verticals_root).expect("create verticals root");
    std::env::set_var("ARGO_VERTICALS_DIR", &verticals_root);
    std::env::set_var("ARGO_GENESIS_LLM_PROVIDER", "definitely-not-a-real-provider-xyz");
    let vertical_id = plant_full_genesis_vertical(
        &verticals_root,
        "op-test",
        "be my marketing intern, run weekly campaigns to cold leads and draft newsletters",
        "maya",
    );

    reset_idem_cache_for_tests();
    let store = mem_store().await;
    let mut draft = valid_draft();
    draft["suggestedVertical"] = json!(vertical_id);
    let body = json!({
        "worker_draft": draft,
        "problem": "Marketing intern who runs weekly campaigns to cold leads.",
        "answers": { "source": "genesis_stream" },
    })
    .to_string();
    let (status, body) = hire_response(&hire_req(&body), Some(&store)).await;
    std::env::remove_var("ARGO_GENESIS_LLM_PROVIDER");
    assert_eq!(
        status, "422 Unprocessable Entity",
        "full genesis teammate must be held for review without a live judge; body: {body}"
    );
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["gate"]["state"], "needs_review");
    assert_eq!(v["gate"]["hireable"], false);
    // The eval floor genuinely ran and cleared (10 seeds, >=5 passed).
    assert_eq!(v["gate"]["eval_total"], 10);
    assert!(v["gate"]["eval_passed"].as_u64().unwrap() >= 5);
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
