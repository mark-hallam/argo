//! Tests for `routes_worker_overview` — `GET /api/v1/workers/<id>/overview` (J8).

#![allow(clippy::too_many_lines)]

use super::super::routes_worker_detail::PrincipalRole;
use super::{
    classify_source, label_from_template, next_fire_for, parse_role_header, show_response,
    slugify, trend_for, SourceKind,
};
use argo_audit::{AuditKind, AuditSink};
use argo_identity::{IdentityStore, Persona, Role, Worker, WorkerId};
use argo_store::{SessionCreate, SessionStore};
use argo_team_headline::bullets_for_vertical;
use argo_types::{SessionId, TenantId, WorkspaceId};
use chrono::{Duration, Utc};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::{Mutex, MutexGuard};
use uuid::Uuid;

const VERT: &str = "customer-support";

async fn env_lock() -> MutexGuard<'static, ()> {
    static M: Mutex<()> = Mutex::const_new(());
    M.lock().await
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().and_then(|p| p.parent()).expect("workspace root").to_path_buf()
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
    let name = cwd.file_name().and_then(|s| s.to_str()).unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, ws) = store.ensure_workspace(name, &root).await.expect("ensure workspace");
    Env { _guard: guard, _tmp: tmp, store, tenant, ws }
}

fn req_for(id: &str, role: Option<&str>, principal: Option<&str>) -> String {
    let mut req = format!("GET /api/v1/workers/{id}/overview HTTP/1.1\r\nHost: x\r\n");
    if let Some(r) = role { req.push_str(&format!("X-Argo-Role: {r}\r\n")); }
    if let Some(p) = principal { req.push_str(&format!("X-Argo-Principal: {p}\r\n")); }
    req.push_str("\r\n");
    req
}

async fn add_worker(store: &Arc<SessionStore>, tenant: TenantId, builder: &str) -> Worker {
    let mut worker = Worker::draft(
        tenant, VERT,
        Persona::named("Maya", builder),
        Role::new("Resolve customer questions."),
    );
    worker.charter = Some("Be the most helpful triage agent on the floor.".into());
    IdentityStore::new(store).create_worker(&worker).await.expect("create worker");
    worker
}

async fn seed_session(env: &Env, worker: WorkerId) -> SessionId {
    env.store
        .create_session_with_worker(SessionCreate {
            tenant_id: env.tenant, workspace_id: env.ws,
            model: "test-model", provider: "test-provider",
            title: Some("seed"), vertical_id: Some(VERT),
            worker_id: Some(argo_store::WorkerId(worker.0)),
        })
        .await
        .expect("create session")
}

async fn write_audit(sink: &AuditSink, session: SessionId, kind: AuditKind, payload: Value) {
    sink.write(Some(session), kind, payload, None).await.expect("audit write");
    sink.flush_pending().await.expect("flush audit");
}

async fn parse_ok(req: &str, id: &str, store: &Arc<SessionStore>) -> Value {
    let (status, body) = show_response(id, req, Some(store)).await;
    assert_eq!(status, "200 OK", "body: {body}");
    serde_json::from_str(&body).expect("json body")
}

#[tokio::test]
async fn happy_path_returns_all_top_level_fields() {
    let env = prep().await;
    let worker = add_worker(&env.store, env.tenant, "human:mark").await;
    let req = req_for(&worker.id.to_string(), Some("operator"), Some("human:mark"));
    let v = parse_ok(&req, &worker.id.to_string(), &env.store).await;
    for key in ["charter", "duties", "kpis", "recent_activity", "permissions"] {
        assert!(v.get(key).is_some(), "missing key {key} in {v}");
    }
    assert_eq!(v["charter"]["version"], 1);
    assert_eq!(v["charter"]["text"], "Be the most helpful triage agent on the floor.");
    assert_eq!(v["charter"]["last_edited_by"], "human:mark");
    assert!(v["charter"]["last_edited_at"].is_string());
}

#[tokio::test]
async fn invalid_worker_id_returns_400() {
    let env = prep().await;
    let req = req_for("not-a-uuid", None, None);
    let (status, body) = show_response("not-a-uuid", &req, Some(&env.store)).await;
    assert_eq!(status, "400 Bad Request");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["error"]["code"], "bad_request");
}

#[tokio::test]
async fn unknown_worker_id_returns_404() {
    let env = prep().await;
    let ghost = WorkerId::new().to_string();
    let req = req_for(&ghost, None, None);
    let (status, body) = show_response(&ghost, &req, Some(&env.store)).await;
    assert_eq!(status, "404 Not Found", "body: {body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["error"]["code"], "not_found");
}

#[tokio::test]
async fn cross_tenant_worker_returns_403() {
    let env = prep().await;
    let alien_tenant = TenantId::new();
    let alien = add_worker(&env.store, alien_tenant, "human:mark").await;
    let req = req_for(&alien.id.to_string(), None, None);
    let (status, body) = show_response(&alien.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "403 Forbidden", "body: {body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["error"]["code"], "forbidden");
}

#[tokio::test]
async fn archived_worker_returns_410_with_recoverable_until() {
    let env = prep().await;
    let worker = add_worker(&env.store, env.tenant, "human:mark").await;
    IdentityStore::new(&env.store).archive_worker(env.tenant, worker.id).await.expect("archive");
    let req = req_for(&worker.id.to_string(), None, None);
    let (status, body) = show_response(&worker.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "410 Gone", "body: {body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["error"]["code"], "gone");
    assert!(v["archived_at"].is_string());
    assert!(v["recoverable_until_at"].is_string());
    assert!(v["retry_after_seconds"].is_number());
}

#[tokio::test]
async fn handler_returns_503_when_store_missing() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let id = WorkerId::new().to_string();
    let req = req_for(&id, None, None);
    let (status, body) = show_response(&id, &req, None).await;
    assert_eq!(status, "503 Service Unavailable");
    assert!(body.contains("session store unavailable"));
}

#[tokio::test]
async fn dispatch_path_ignores_query_string() {
    let env = prep().await;
    let worker = add_worker(&env.store, env.tenant, "human:mark").await;
    let id_q = format!("{}?ts=1", worker.id);
    let req = format!(
        "GET /api/v1/workers/{id_q}/overview HTTP/1.1\r\nHost: x\r\nX-Argo-Role: operator\r\n\r\n"
    );
    let (status, body) = show_response(&id_q, &req, Some(&env.store)).await;
    assert_eq!(status, "200 OK", "body: {body}");
}

#[tokio::test]
async fn kpis_reflect_seeded_audit_rows_and_drop_not_computable() {
    let env = prep().await;
    let worker = add_worker(&env.store, env.tenant, "human:mark").await;
    let session = seed_session(&env, worker.id).await;
    let sink = AuditSink::new(env.store.clone());
    for _ in 0..4 {
        write_audit(&sink, session, AuditKind::ToolCall, serde_json::json!({})).await;
    }
    for _ in 0..2 {
        write_audit(&sink, session, AuditKind::EscalationRequired, serde_json::json!({})).await;
    }
    let req = req_for(&worker.id.to_string(), Some("operator"), Some("human:mark"));
    let v = parse_ok(&req, &worker.id.to_string(), &env.store).await;
    let kpis = v["kpis"].as_array().expect("kpis array");
    // customer-support has 3 BulletKinds: ToolCalls, NotComputableV1, Pending.
    // The NotComputable row must drop.
    let keys: Vec<&str> = kpis.iter().filter_map(|k| k["key"].as_str()).collect();
    assert_eq!(kpis.len(), 2, "expected ToolCalls + Pending; got {keys:?}");
    let tickets = kpis.iter().find(|k| k["label"] == "Tickets resolved today").expect("tickets");
    assert_eq!(tickets["value"], "4");
    assert_eq!(tickets["direction_means_good"], Value::Bool(true));
    assert_eq!(tickets["sparkline_7d"].as_array().expect("sparkline").len(), 7);
    let pend = kpis.iter().find(|k| k["label"] == "Pending escalations").expect("pending");
    assert_eq!(pend["value"], "2");
    assert_eq!(pend["direction_means_good"], Value::Bool(false));
}

#[tokio::test]
async fn recent_activity_capped_at_five_newest_first() {
    let env = prep().await;
    let worker = add_worker(&env.store, env.tenant, "human:mark").await;
    let session = seed_session(&env, worker.id).await;
    let sink = AuditSink::new(env.store.clone());
    for i in 0..8 {
        write_audit(
            &sink, session, AuditKind::ToolCall,
            serde_json::json!({ "verb": format!("call_{i}"), "status": "succeeded" }),
        ).await;
    }
    let req = req_for(&worker.id.to_string(), Some("operator"), Some("human:mark"));
    let v = parse_ok(&req, &worker.id.to_string(), &env.store).await;
    let rows = v["recent_activity"].as_array().expect("recent_activity array");
    assert_eq!(rows.len(), 5);
    let verbs: Vec<&str> = rows.iter().filter_map(|r| r["verb"].as_str()).collect();
    assert!(verbs.iter().all(|v| v.starts_with("call_")));
    for row in rows {
        assert!(row["timestamp"].is_string());
        assert!(row["status"].is_string());
        assert!(row["id"].is_string());
    }
}

#[tokio::test]
async fn duties_include_role_duty_rows() {
    let env = prep().await;
    let worker = add_worker(&env.store, env.tenant, "human:mark").await;
    let schema = include_str!("../../../../argo-store/migrations/28__role_duties.sql");
    for stmt in schema.split_terminator(';').map(str::trim).filter(|s| !s.is_empty()) {
        sqlx::query(stmt).execute(env.store.pool()).await.expect("role_duty schema");
    }
    sqlx::query(
        "INSERT INTO role_duty (id, tenant_id, worker_id, cron, entry_action, description, \
         enabled, paused_until, last_fired_at, created_at) \
         VALUES (?, ?, ?, '0 0 6 * * *', 'morning_routine', 'Morning routine', 1, NULL, NULL, ?)",
    )
    .bind(Uuid::now_v7().to_string())
    .bind(env.tenant.0.as_bytes().as_slice())
    .bind(worker.id.to_string())
    .bind(Utc::now().timestamp_micros())
    .execute(env.store.pool())
    .await
    .expect("insert duty");
    let req = req_for(&worker.id.to_string(), Some("operator"), Some("human:mark"));
    let v = parse_ok(&req, &worker.id.to_string(), &env.store).await;
    let duties = v["duties"].as_array().expect("duties array");
    assert_eq!(duties.len(), 1);
    let d = &duties[0];
    assert_eq!(d["label"], "Morning routine");
    assert_eq!(d["schedule"]["kind"], "cron");
    assert_eq!(d["schedule"]["spec"], "0 0 6 * * *");
    assert_eq!(d["enabled"], Value::Bool(true));
    assert!(d["next_fire_at"].is_string(), "expected next_fire_at, got {d}");
}

#[tokio::test]
async fn audit_row_written_per_view() {
    let env = prep().await;
    let worker = add_worker(&env.store, env.tenant, "human:mark").await;
    let req = req_for(&worker.id.to_string(), Some("operator"), Some("human:auditor"));
    let _ = parse_ok(&req, &worker.id.to_string(), &env.store).await;
    let sink = AuditSink::new(env.store.clone());
    sink.flush_pending().await.unwrap();
    let payload: String = sqlx::query_scalar(
        "SELECT payload_json FROM audit_record WHERE kind = 'http.request' \
         ORDER BY created_at DESC LIMIT 1",
    )
    .fetch_one(env.store.pool())
    .await
    .expect("audit row");
    assert!(payload.contains("worker.overview.viewed"));
    assert!(payload.contains("\"viewer_role\":\"operator\""));
}

#[tokio::test]
async fn permissions_resolve_per_role_table() {
    let env = prep().await;
    let mine = add_worker(&env.store, env.tenant, "human:mark").await;
    let theirs = add_worker(&env.store, env.tenant, "human:other").await;
    let v_op = parse_ok(
        &req_for(&mine.id.to_string(), Some("operator"), Some("human:other")),
        &mine.id.to_string(),
        &env.store,
    )
    .await;
    let p = &v_op["permissions"];
    for key in [
        "can_edit_charter", "can_edit_duties", "can_pause_resume",
        "can_archive", "can_delete", "can_view_audit", "can_view_settings",
    ] {
        assert_eq!(p[key], Value::Bool(true), "operator must have {key}");
    }
    let v_mine = parse_ok(
        &req_for(&mine.id.to_string(), Some("delegator"), Some("human:mark")),
        &mine.id.to_string(),
        &env.store,
    ).await;
    let v_theirs = parse_ok(
        &req_for(&theirs.id.to_string(), Some("delegator"), Some("human:mark")),
        &theirs.id.to_string(),
        &env.store,
    ).await;
    assert_eq!(v_mine["permissions"]["can_edit_charter"], Value::Bool(true));
    assert_eq!(v_theirs["permissions"]["can_edit_charter"], Value::Bool(false));
    assert_eq!(v_mine["permissions"]["can_delete"], Value::Bool(false));
}

// Pure-function tests — no DB so no env_lock needed.
#[test]
fn classify_source_recovers_bullet_source_via_format_probe() {
    let cs = bullets_for_vertical("customer-support");
    assert_eq!(cs.len(), 3);
    let kinds: Vec<SourceKind> = cs.iter().map(classify_source).collect();
    assert_eq!(kinds[0], SourceKind::ToolCalls);
    assert_eq!(kinds[1], SourceKind::NotComputable);
    assert_eq!(kinds[2], SourceKind::Pending);
    let fallback = bullets_for_vertical("totally-unknown-vertical");
    assert!(fallback.iter().map(classify_source).any(|k| k == SourceKind::AnyAction));
}

#[test]
fn label_from_template_strips_count_prefix_and_capitalises() {
    assert_eq!(label_from_template("{n} replies sent"), "Replies sent");
    assert_eq!(label_from_template("{n} pending escalations"), "Pending escalations");
    assert_eq!(label_from_template("revenue this week"), "Revenue this week");
}

#[test]
fn slugify_lowercases_and_collapses_non_alnum() {
    assert_eq!(slugify("Replies sent"), "replies_sent");
    assert_eq!(slugify("Avg response time"), "avg_response_time");
    assert_eq!(slugify("Pending escalations"), "pending_escalations");
}

#[test]
fn trend_for_up_down_flat_and_prev_zero_special_cases() {
    let (dir, pct) = trend_for(120.0, 100.0);
    assert_eq!(dir, "up");
    assert!((pct - 20.0).abs() < 0.05);
    let (dir, pct) = trend_for(80.0, 100.0);
    assert_eq!(dir, "down");
    assert!((pct + 20.0).abs() < 0.05);
    let (dir, pct) = trend_for(101.0, 100.0);
    assert_eq!(dir, "flat");
    assert!(pct.abs() < 2.0);
    let (dir, _) = trend_for(5.0, 0.0);
    assert_eq!(dir, "up");
    let (dir, pct) = trend_for(0.0, 0.0);
    assert_eq!(dir, "flat");
    assert!(pct.abs() < 0.01);
}

#[test]
fn next_fire_for_returns_a_future_instant_when_cron_is_valid() {
    let now = Utc::now();
    let fire = next_fire_for("0 6 * * * *", None, now).expect("cron parsable");
    assert!(fire >= now);
    assert!(fire <= now + Duration::days(1));
    assert!(next_fire_for("not a cron", None, now).is_none());
}

#[test]
fn parse_role_header_recognises_canonical_roles_and_aliases() {
    fn req(value: &str) -> String {
        format!("GET / HTTP/1.1\r\nHost: x\r\nX-Argo-Role: {value}\r\n\r\n")
    }
    assert_eq!(parse_role_header(&req("operator")), Some(PrincipalRole::Operator));
    assert_eq!(parse_role_header(&req("delegator")), Some(PrincipalRole::Delegator));
    assert_eq!(parse_role_header(&req("auditor")), Some(PrincipalRole::Auditor));
    assert_eq!(
        parse_role_header(&req("compliance-officer")),
        Some(PrincipalRole::ComplianceOfficer)
    );
    assert_eq!(
        parse_role_header(&req("compliance_officer")),
        Some(PrincipalRole::ComplianceOfficer)
    );
    assert!(parse_role_header(&req("nonsense")).is_none());
    assert!(parse_role_header("GET / HTTP/1.1\r\n\r\n").is_none());
}
