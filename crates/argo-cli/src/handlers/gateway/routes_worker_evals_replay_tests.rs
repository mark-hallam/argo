//! Tests for `routes_worker_evals_replay` — the store-backed teammate
//! historical-replay scorecard (`GET /api/v1/workers/<id>/evals/replay`).
//!
//! Mirrors the `routes_evals_tests` harness: env-lock, tempdir-pinned
//! `ARGO_HOME` + `ARGO_VERTICALS_DIR`, cwd-derived workspace tenant. Adds
//! `ARGO_EVAL_SCENARIOS_DIR` so synthetic goldens load from a tempdir,
//! and seeds production interactions as `replay_outcome` audit rows via
//! [`AuditSink`] — exactly the rows the live producer will write.

#![allow(clippy::too_many_lines)]

use super::replay_response;
use argo_audit::{AuditKind, AuditSink};
use argo_eval::replay::ReplayedInteraction;
use argo_eval::teammate::{ScenarioOutcome, TeammateScenario};
use argo_eval::teammate_runner::GoldenCase;
use argo_identity::{IdentityStore, Persona, Role, Worker};
use argo_store::SessionStore;
use argo_types::TenantId;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{Mutex, MutexGuard};

async fn env_lock() -> MutexGuard<'static, ()> {
    static M: Mutex<()> = Mutex::const_new(());
    M.lock().await
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

/// Set the test env and return the scenarios root the handler will read.
fn set_test_env(tmp: &Path) -> PathBuf {
    std::env::set_var("ARGO_HOME", tmp.join("home"));
    std::fs::create_dir_all(tmp.join("home")).expect("home dir");
    std::env::set_var("ARGO_VERTICALS_DIR", workspace_root().join("verticals"));
    let scenarios = tmp.join("scenarios");
    std::env::set_var("ARGO_EVAL_SCENARIOS_DIR", &scenarios);
    scenarios
}

async fn mem_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("open mem db");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

async fn workspace_tenant(store: &Arc<SessionStore>) -> TenantId {
    let cwd = std::env::current_dir().expect("cwd");
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, _) = store
        .ensure_workspace(name, &root)
        .await
        .expect("ensure workspace");
    tenant
}

async fn add_worker(store: &Arc<SessionStore>, tenant: TenantId, name: &str) -> Worker {
    let mut worker = Worker::draft(
        tenant,
        "support",
        Persona::named(name, "human:mark"),
        Role::new("Resolve customer questions."),
    );
    worker.trust_grade = 3;
    IdentityStore::new(store)
        .create_worker(&worker)
        .await
        .expect("create worker");
    worker
}

fn scenario(name: &str) -> TeammateScenario {
    TeammateScenario {
        name: name.to_string(),
        situation: "A customer asks for a refund outside policy.".to_string(),
        expected_substrings: vec!["policy".to_string()],
        should_escalate: true,
        required_skills: vec![],
        forbidden_claim: None,
        budget_ms: 5_000,
        budget_usd_micros: 20_000,
    }
}

/// A fully-passing outcome for [`scenario`] (every axis at or above
/// threshold): mentions "policy", escalates, honest, inside budget.
fn passing_outcome() -> ScenarioOutcome {
    ScenarioOutcome {
        output: "Our refund policy is 30 days; I'll escalate this exception.".to_string(),
        escalated: true,
        skills_used: vec![],
        honest: true,
        elapsed_ms: 900,
        cost_usd_micros: 4_000,
    }
}

/// A failing production outcome: no "policy" substring, did not escalate,
/// and dishonest — drives correctness/escalation/honesty below threshold.
fn failing_outcome() -> ScenarioOutcome {
    ScenarioOutcome {
        output: "Sure, money is already back in your account.".to_string(),
        escalated: false,
        skills_used: vec![],
        honest: false,
        elapsed_ms: 900,
        cost_usd_micros: 4_000,
    }
}

/// Write the worker's synthetic goldens (one `GoldenCase` per line) to
/// `<root>/<teammate>/cases.jsonl`.
fn write_goldens(root: &Path, teammate: &str, cases: &[GoldenCase]) {
    let dir = root.join(teammate);
    std::fs::create_dir_all(&dir).expect("scenarios dir");
    let body = cases
        .iter()
        .map(|c| serde_json::to_string(c).expect("serialize golden"))
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(dir.join("cases.jsonl"), body).expect("write goldens");
}

/// Seed one production interaction as a `replay_outcome` audit row under
/// `tenant`, tagged with the worker id so the handler's payload-LIKE
/// query finds it.
async fn seed_production_outcome(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    worker_id: &str,
    interaction: &ReplayedInteraction,
) {
    let payload = json!({
        "worker_id": worker_id,
        "endpoint": "teammate.interaction.recorded",
        "replay_outcome": interaction,
    });
    AuditSink::new_with_tenant(Arc::clone(store), tenant.to_string())
        .write(None, AuditKind::TurnEnd, payload, None)
        .await
        .expect("seed production outcome");
}

fn get_req(path: &str) -> String {
    format!("GET {path} HTTP/1.1\r\nHost: x\r\n\r\n")
}

// =========================================================================
// Merge: synthetic goldens + production rows → one scorecard, and the
// production rows shift the score vs synthetic-only.
// =========================================================================

#[tokio::test]
async fn replay_merges_synthetic_and_production_outcomes() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    let scenarios_root = set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Refunda").await;
    let teammate = worker.id.to_string();

    // Two synthetic goldens, both passing → synthetic pass rate 1.0.
    let goldens = vec![
        GoldenCase {
            scenario: scenario("support/refund-001"),
            outcome: passing_outcome(),
        },
        GoldenCase {
            scenario: scenario("support/refund-002"),
            outcome: passing_outcome(),
        },
    ];
    write_goldens(&scenarios_root, &teammate, &goldens);

    // One real production interaction for refund-001 that FAILED.
    seed_production_outcome(
        &store,
        tenant,
        &teammate,
        &ReplayedInteraction {
            scenario_name: "support/refund-001".to_string(),
            outcome: failing_outcome(),
        },
    )
    .await;

    let path = format!("/api/v1/workers/{}/evals/replay", worker.id);
    let (status, body) = replay_response(&teammate, &get_req(&path), Some(&store)).await;
    assert_eq!(status, "200 OK", "body: {body}");
    let v: Value = serde_json::from_str(&body).expect("parse json");

    assert_eq!(v["workerId"], teammate);
    // Synthetic-only: 2 scenarios, both pass.
    assert_eq!(v["synthetic"]["total"], 2, "two synthetic goldens: {v}");
    assert_eq!(v["synthetic"]["passed"], 2);
    let synthetic_rate = v["synthetic"]["passRate"].as_f64().expect("synthetic rate");
    assert!((synthetic_rate - 1.0).abs() < 1e-9, "synthetic all pass: {v}");

    // Production: the store-backed source was consumed (NOT an empty vec).
    assert_eq!(
        v["production"]["scored"], 1,
        "replay_report consumed the one seeded production row: {v}"
    );

    // Merged: 2 synthetic + 1 production = 3 scored, only the 2 synthetic
    // pass → the production failure SHIFTS the score below synthetic-only.
    assert_eq!(v["merged"]["total"], 3, "merged counts both sources: {v}");
    assert_eq!(v["merged"]["passed"], 2);
    let merged_rate = v["merged"]["passRate"].as_f64().expect("merged rate");
    assert!(
        merged_rate < synthetic_rate,
        "production rows must shift the merged score down: merged={merged_rate} synthetic={synthetic_rate} ({v})"
    );
    assert!((merged_rate - 2.0 / 3.0).abs() < 1e-9, "2/3 pass: {v}");
}

/// With zero production rows the replay is honest-empty (production
/// `scored` 0) and the merged score equals synthetic-only — NO THEATER.
#[tokio::test]
async fn replay_with_no_production_rows_is_synthetic_only() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    let scenarios_root = set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Solo").await;
    let teammate = worker.id.to_string();

    write_goldens(
        &scenarios_root,
        &teammate,
        &[GoldenCase {
            scenario: scenario("support/refund-001"),
            outcome: passing_outcome(),
        }],
    );

    let path = format!("/api/v1/workers/{}/evals/replay", worker.id);
    let (status, body) = replay_response(&teammate, &get_req(&path), Some(&store)).await;
    assert_eq!(status, "200 OK", "body: {body}");
    let v: Value = serde_json::from_str(&body).expect("parse json");
    assert_eq!(v["production"]["scored"], 0, "no production rows: {v}");
    assert_eq!(v["merged"]["total"], v["synthetic"]["total"], "merged == synthetic: {v}");
    assert_eq!(v["merged"]["passed"], v["synthetic"]["passed"]);
}

// =========================================================================
// Tenant scoping + arg validation
// =========================================================================

/// A worker in another tenant must surface as 404 (no existence leak),
/// and its production replay_outcome rows must never reach the response.
#[tokio::test]
async fn cross_tenant_worker_returns_404() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    let scenarios_root = set_test_env(tmp.path());
    let store = mem_store().await;
    let _cwd_tenant = workspace_tenant(&store).await;
    let alien_tenant = TenantId::new();
    let alien = add_worker(&store, alien_tenant, "Alien").await;
    let teammate = alien.id.to_string();

    write_goldens(
        &scenarios_root,
        &teammate,
        &[GoldenCase {
            scenario: scenario("support/refund-001"),
            outcome: passing_outcome(),
        }],
    );
    seed_production_outcome(
        &store,
        alien_tenant,
        &teammate,
        &ReplayedInteraction {
            scenario_name: "support/refund-001".to_string(),
            outcome: failing_outcome(),
        },
    )
    .await;

    let path = format!("/api/v1/workers/{}/evals/replay", alien.id);
    let (status, _) = replay_response(&teammate, &get_req(&path), Some(&store)).await;
    assert_eq!(
        status, "404 Not Found",
        "cross-tenant worker must surface as 404 (same as unknown)"
    );
}

#[tokio::test]
async fn invalid_worker_id_returns_400() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let _ = workspace_tenant(&store).await;
    let (status, _) = replay_response("not-a-uuid", "GET /\r\n\r\n", Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
}

#[tokio::test]
async fn missing_store_returns_503() {
    let _g = env_lock().await;
    let ghost = uuid::Uuid::now_v7();
    let path = format!("/api/v1/workers/{ghost}/evals/replay");
    let (status, _) = replay_response(&ghost.to_string(), &get_req(&path), None).await;
    assert!(status.starts_with("503"), "no store → 503, got {status}");
}
