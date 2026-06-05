//! Tests for `routes_worker_scorecard` — pure scoring/classification +
//! DB-driven route tests over a seeded in-memory store on the
//! `pr-checker` vertical (whose `[success_metrics]` synthesize a
//! `median_response_seconds = 60` target — the acceptance scenario).

#![allow(clippy::too_many_lines)]
#![allow(deprecated)]

use super::score::{classify_source, comparator_symbol, format_value, median, score, ScoreSource};
use super::{humanize, show_response, team_scorecard_response};
use argo_audit::{AuditKind, AuditSink};
use argo_identity::{IdentityStore, Persona, Role, Worker, WorkerId};
use argo_store::{SessionCreate, SessionStore};
use argo_types::{SessionId, TenantId, WorkspaceId};
use argo_vertical::{KpiComparator, KpiDefinition, KpiGate, KpiKind};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::{Mutex, MutexGuard};
use uuid::Uuid;

const VERT: &str = "pr-checker";

// ---- pure scoring -------------------------------------------------------

#[test]
fn lte_target_beaten_reports_met_above_100_pct() {
    // "respond < 60s" (60_000ms) observed at 45_000ms → met, 133.3%.
    let (pct, met) = score(45_000.0, 60_000.0, KpiComparator::Lte);
    assert!(met);
    assert_eq!(pct, 133.3);
}

#[test]
fn lte_target_missed_reports_at_risk_below_100_pct() {
    // observed 90_000ms vs 60_000ms target → missed, 66.7%.
    let (pct, met) = score(90_000.0, 60_000.0, KpiComparator::Lte);
    assert!(!met);
    assert_eq!(pct, 66.7);
}

#[test]
fn gte_target_met_and_missed() {
    let (pct_ok, ok) = score(90.0, 80.0, KpiComparator::Gte);
    assert!(ok);
    assert_eq!(pct_ok, 112.5);
    let (pct_bad, bad) = score(40.0, 80.0, KpiComparator::Gte);
    assert!(!bad);
    assert_eq!(pct_bad, 50.0);
}

#[test]
fn eq_target_within_epsilon_is_met() {
    assert!(score(1.0, 1.0, KpiComparator::Eq).1);
    assert!(!score(0.0, 1.0, KpiComparator::Eq).1);
}

#[test]
fn classify_maps_only_known_sources() {
    assert_eq!(classify_source(&kpi("p95", KpiKind::DurationMs, 60_000.0, KpiComparator::Lte)), ScoreSource::ResponseLatencyMs);
    assert_eq!(classify_source(&kpi("messages_handled", KpiKind::Count, 10.0, KpiComparator::Gte)), ScoreSource::AuditCount("assistant_message"));
    assert_eq!(classify_source(&kpi("escalations", KpiKind::Count, 1.0, KpiComparator::Lte)), ScoreSource::AuditCount("escalation.required"));
    assert_eq!(classify_source(&kpi("tasks_done", KpiKind::Count, 5.0, KpiComparator::Gte)), ScoreSource::AuditCount("tool_call"));
    // Percentage / unrecognised count → unmeasured (NO THEATER).
    assert_eq!(classify_source(&kpi("deflection_rate", KpiKind::Percentage, 80.0, KpiComparator::Gte)), ScoreSource::Unmeasured);
    assert_eq!(classify_source(&kpi("required_audit_chain_intact", KpiKind::Count, 1.0, KpiComparator::Eq)), ScoreSource::Unmeasured);
}

#[test]
fn median_handles_empty_odd_even() {
    assert_eq!(median(&[]), None);
    assert_eq!(median(&[3.0, 1.0, 2.0]), Some(2.0));
    assert_eq!(median(&[10.0, 20.0, 30.0, 40.0]), Some(25.0));
}

#[test]
fn format_value_and_symbols() {
    assert_eq!(format_value(KpiKind::DurationMs, 60_000.0), "60000ms");
    assert_eq!(format_value(KpiKind::Percentage, 80.0), "80%");
    assert_eq!(format_value(KpiKind::Count, 5.0), "5");
    assert_eq!(comparator_symbol(KpiComparator::Lte), "≤");
    assert_eq!(comparator_symbol(KpiComparator::Gte), "≥");
}

#[test]
fn humanize_titlecases_snake_names() {
    assert_eq!(humanize("median_response_seconds"), "Median response seconds");
}

fn kpi(name: &str, kind: KpiKind, threshold: f64, comparator: KpiComparator) -> KpiDefinition {
    KpiDefinition {
        name: name.to_string(),
        kind,
        threshold,
        comparator,
        gate: KpiGate::Warn,
        description: None,
    }
}

// ---- DB-driven route tests ---------------------------------------------

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

fn set_test_env(tmp: &std::path::Path) {
    std::env::set_var("ARGO_HOME", tmp.join("home"));
    std::fs::create_dir_all(tmp.join("home")).expect("home dir");
    std::env::set_var("ARGO_VERTICALS_DIR", workspace_root().join("verticals"));
}

struct Env {
    _guard: MutexGuard<'static, ()>,
    _tmp: TempDir,
    store: Arc<SessionStore>,
    tenant: TenantId,
    ws: WorkspaceId,
}

async fn prep() -> Env {
    let guard = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = Arc::new(SessionStore::open(":memory:").await.expect("open"));
    store.migrate().await.expect("migrate");
    let cwd = std::env::current_dir().expect("cwd");
    let root = cwd.to_string_lossy();
    let (tenant, ws) = store
        .ensure_workspace("scorecard-test", &root)
        .await
        .expect("ensure workspace");
    Env {
        _guard: guard,
        _tmp: tmp,
        store,
        tenant,
        ws,
    }
}

async fn add_worker(store: &Arc<SessionStore>, tenant: TenantId) -> Worker {
    let worker = Worker::draft(
        tenant,
        VERT,
        Persona::named("Petra", "human:mark"),
        Role::new("Review inbound PRs."),
    );
    IdentityStore::new(store)
        .create_worker(&worker)
        .await
        .expect("create worker");
    worker
}

async fn seed_session(env: &Env, worker: WorkerId) -> SessionId {
    env.store
        .create_session_with_worker(SessionCreate {
            tenant_id: env.tenant,
            workspace_id: env.ws,
            model: "test-model",
            provider: "test-provider",
            title: Some("seed"),
            vertical_id: Some(VERT),
            worker_id: Some(argo_store::WorkerId(worker.0)),
        })
        .await
        .expect("create session")
}

async fn write_latency(sink: &AuditSink, session: SessionId, latency_ms: i64) {
    sink.write(
        Some(session),
        AuditKind::AssistantMessage,
        serde_json::json!({ "latency_ms": latency_ms }),
        None,
    )
    .await
    .expect("audit write");
    sink.flush_pending().await.expect("flush audit");
}

fn req_for(id: &str) -> String {
    format!("GET /api/v1/workers/{id}/scorecard HTTP/1.1\r\nHost: x\r\n\r\n")
}

async fn parse_ok(id: &str, store: &Arc<SessionStore>) -> Value {
    let (status, body) = show_response(id, &req_for(id), Some(store)).await;
    assert_eq!(status, "200 OK", "body: {body}");
    serde_json::from_str(&body).expect("json body")
}

fn find_goal<'a>(v: &'a Value, name: &str) -> &'a Value {
    v["goals"]
        .as_array()
        .expect("goals array")
        .iter()
        .find(|g| g["name"] == name)
        .unwrap_or_else(|| panic!("goal {name} missing in {v}"))
}

#[tokio::test]
async fn fast_responses_render_met_above_100_pct() {
    let env = prep().await;
    let worker = add_worker(&env.store, env.tenant).await;
    let session = seed_session(&env, worker.id).await;
    let sink = AuditSink::new(env.store.clone());
    for _ in 0..3 {
        write_latency(&sink, session, 45_000).await;
    }
    let v = parse_ok(&worker.id.to_string(), &env.store).await;
    let goal = find_goal(&v, "median_response_seconds");
    assert_eq!(goal["measured"], true);
    assert_eq!(goal["status"], "met");
    assert!(
        goal["attainment_pct"].as_f64().expect("pct") > 100.0,
        "expected >100% in {goal}"
    );
    assert_eq!(goal["actual"]["display"], "45000ms");
    assert_eq!(v["summary"]["goals_met"], 1);
    assert_eq!(v["summary"]["goals_measured"], 1);
}

#[tokio::test]
async fn slow_responses_render_at_risk() {
    let env = prep().await;
    let worker = add_worker(&env.store, env.tenant).await;
    let session = seed_session(&env, worker.id).await;
    let sink = AuditSink::new(env.store.clone());
    write_latency(&sink, session, 90_000).await;
    let v = parse_ok(&worker.id.to_string(), &env.store).await;
    let goal = find_goal(&v, "median_response_seconds");
    assert_eq!(goal["status"], "at_risk");
    assert_eq!(v["summary"]["goals_met"], 0);
}

#[tokio::test]
async fn unmeasured_targets_render_honest_note_not_a_number() {
    let env = prep().await;
    let worker = add_worker(&env.store, env.tenant).await;
    let v = parse_ok(&worker.id.to_string(), &env.store).await;
    // deflection_target_pct (Percentage) has no countable audit source.
    let goal = find_goal(&v, "deflection_target_pct");
    assert_eq!(goal["measured"], false);
    assert_eq!(goal["note"], "target set — measurement not wired");
    assert!(goal.get("actual").is_none(), "must not fabricate an actual: {goal}");
    assert!(goal.get("attainment_pct").is_none());
}

#[tokio::test]
async fn other_tenant_cannot_read_scorecard() {
    let env = prep().await;
    let worker = add_worker(&env.store, env.tenant).await;
    // A fixed valid UUID that is not the cwd-resolved env tenant.
    let other = Uuid::from_u128(0x1234_5678_9abc_def0_1234_5678_9abc_def0);
    let req = format!(
        "GET /api/v1/workers/{id}/scorecard HTTP/1.1\r\nHost: x\r\nX-Tenant-Id: {other}\r\n\r\n",
        id = worker.id
    );
    let (status, _) = show_response(&worker.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "404 Not Found");
}

#[tokio::test]
async fn team_rollup_counts_goals_hit() {
    let env = prep().await;
    let worker = add_worker(&env.store, env.tenant).await;
    let session = seed_session(&env, worker.id).await;
    let sink = AuditSink::new(env.store.clone());
    write_latency(&sink, session, 30_000).await;
    let req = "GET /api/v1/team/scorecard HTTP/1.1\r\nHost: x\r\n\r\n";
    let (status, body) = team_scorecard_response(req, Some(&env.store)).await;
    assert_eq!(status, "200 OK", "body: {body}");
    let v: Value = serde_json::from_str(&body).expect("json");
    assert_eq!(v["summary"]["goals_met"], 1);
    assert_eq!(v["summary"]["goals_measured"], 1);
    assert!(v["workers"].as_array().expect("workers").iter().any(|w| w["worker_id"] == worker.id.to_string()));
}

#[tokio::test]
async fn store_unavailable_returns_503() {
    let (status, _) = show_response("x", &req_for("x"), None).await;
    assert_eq!(status, "503 Service Unavailable");
}
