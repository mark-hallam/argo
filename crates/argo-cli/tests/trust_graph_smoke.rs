//! Integration smoke test for the cross-tenant trust graph
//! (Stream `mission-cross-tenant-trust-graph`).
//!
//! Seeds the `trust_observation` table with a 3-tenant A→B→C chain
//! of quality observations, runs the `argo trust composed` CLI, and
//! asserts the composed score is positive AND strictly less than the
//! A→B direct edge (the path A→B→C is the product of two weights,
//! both ≤ 1.0, so it must be strictly smaller than either factor).

mod common;

use argo_store::trust_observation::{record_observation, NewTrustObservation};
use argo_store::SessionStore;
use argo_types::{SkillId, TenantId};
use common::argo_bin;
use serde_json::Value;
use std::process::{Command, Output};

#[test]
fn three_tenant_chain_composes_strictly_below_direct() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (a, b, c) = rt.block_on(seed(home.path()));

    let direct = run_composed_json(&home, &work, &a, &b);
    let composed = run_composed_json(&home, &work, &a, &c);

    let direct_score = direct["score"].as_f64().expect("direct score");
    let composed_score = composed["score"].as_f64().expect("composed score");
    assert!(direct_score > 0.0, "direct A→B should be positive, got {direct_score}");
    assert!(composed_score > 0.0, "composed A→C should be positive, got {composed_score}");
    assert!(
        composed_score < direct_score,
        "A→C composed ({composed_score}) must be < A→B direct ({direct_score})"
    );

    let path = composed["path"]
        .as_array()
        .expect("path present on success");
    assert_eq!(path.len(), 3, "expected 3-tenant path A→B→C, got {path:?}");
    assert_eq!(path[0].as_str().unwrap(), a);
    assert_eq!(path[2].as_str().unwrap(), c);

    // Also exercise `argo trust graph --from <a> --json`.
    let neighbors = run_graph_json(&home, &work, &a);
    let arr = neighbors.as_array().expect("graph emits an array");
    assert_eq!(arr.len(), 1, "A has one outgoing neighbor (B): got {arr:?}");
    assert_eq!(arr[0]["to_tenant"].as_str().unwrap(), b);
}

async fn seed(home: &std::path::Path) -> (String, String, String) {
    let store = SessionStore::open(home.join("sessions.db")).await.unwrap();
    store.migrate().await.unwrap();
    let a = TenantId::new();
    let b = TenantId::new();
    let c = TenantId::new();
    let skill = SkillId::new();
    let now = chrono::Utc::now();
    // A observes B's skill at quality 0.8 → edge A→B = 0.8.
    record_observation(
        &store,
        &NewTrustObservation {
            source_tenant: b,
            consumer_tenant: a,
            skill_id: skill,
            quality_score: 0.8,
            observed_at: now,
        },
    )
    .await
    .unwrap();
    // B observes C's skill at quality 0.5 → edge B→C = 0.5.
    record_observation(
        &store,
        &NewTrustObservation {
            source_tenant: c,
            consumer_tenant: b,
            skill_id: skill,
            quality_score: 0.5,
            observed_at: now,
        },
    )
    .await
    .unwrap();
    (a.to_string(), b.to_string(), c.to_string())
}

fn run_composed_json(
    home: &tempfile::TempDir,
    work: &tempfile::TempDir,
    from: &str,
    to: &str,
) -> Value {
    let out = run(
        home,
        work,
        &["trust", "composed", "--from", from, "--to", to, "--json"],
    );
    assert_success(&out);
    serde_json::from_slice(&out.stdout).expect("valid JSON")
}

fn run_graph_json(home: &tempfile::TempDir, work: &tempfile::TempDir, from: &str) -> Value {
    let out = run(home, work, &["trust", "graph", "--from", from, "--json"]);
    assert_success(&out);
    serde_json::from_slice(&out.stdout).expect("valid JSON")
}

fn run(home: &tempfile::TempDir, work: &tempfile::TempDir, args: &[&str]) -> Output {
    Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .current_dir(work.path())
        .args(args)
        .output()
        .unwrap()
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "status: {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
