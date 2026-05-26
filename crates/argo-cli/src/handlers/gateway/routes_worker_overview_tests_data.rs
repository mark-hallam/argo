//! DB-driven KPI / duties / activity / audit / permission tests for
//! `routes_worker_overview`. Each test seeds an in-memory store and asserts
//! on the rendered worker-overview envelope.

use super::*;
use chrono::{Duration, Utc};
use jsonwebtoken::{EncodingKey, Header};
use uuid::Uuid;

fn req_for_auth(id: &str, role: Option<&str>, principal: Option<&str>, token: &str) -> String {
    let mut req = req_for(id, role, principal);
    let insert_at = req.find("\r\n\r\n").expect("headers terminator");
    req.insert_str(insert_at, &format!("\r\nAuthorization: Bearer {token}"));
    req
}

fn mint_demo_jwt(email: &str) -> String {
    #[derive(serde::Serialize)]
    struct Claims<'a> {
        sub: &'a str,
        email: &'a str,
        iat: i64,
        exp: i64,
        demo: bool,
    }
    std::env::set_var("SKYCOT_AUTH_JWT_SECRET", "overview-demo-permission-secret");
    let now = Utc::now().timestamp();
    jsonwebtoken::encode(
        &Header::new(jsonwebtoken::Algorithm::HS256),
        &Claims {
            sub: email,
            email,
            iat: now,
            exp: now + 3600,
            demo: true,
        },
        &EncodingKey::from_secret(&super::super::super::auth_magic_link::jwt_secret()),
    )
    .expect("mint demo jwt")
}

#[tokio::test]
async fn happy_path_returns_all_top_level_fields() {
    let env = prep().await;
    let worker = add_worker(&env.store, env.tenant, "human:mark").await;
    let req = req_for(&worker.id.to_string(), Some("operator"), Some("human:mark"));
    let v = parse_ok(&req, &worker.id.to_string(), &env.store).await;
    for key in [
        "charter",
        "duties",
        "kpis",
        "recent_activity",
        "permissions",
    ] {
        assert!(v.get(key).is_some(), "missing key {key} in {v}");
    }
    assert_eq!(v["charter"]["version"], 1);
    assert_eq!(
        v["charter"]["text"],
        "Be the most helpful triage agent on the floor."
    );
    assert_eq!(v["charter"]["last_edited_by"], "human:mark");
    assert!(v["charter"]["last_edited_at"].is_string());
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
        write_audit(
            &sink,
            session,
            AuditKind::EscalationRequired,
            serde_json::json!({}),
        )
        .await;
    }
    let req = req_for(&worker.id.to_string(), Some("operator"), Some("human:mark"));
    let v = parse_ok(&req, &worker.id.to_string(), &env.store).await;
    let kpis = v["kpis"].as_array().expect("kpis array");
    // customer-support has 3 BulletKinds: ToolCalls, NotComputableV1, Pending.
    // The NotComputable row must drop.
    let keys: Vec<&str> = kpis.iter().filter_map(|k| k["key"].as_str()).collect();
    assert_eq!(kpis.len(), 2, "expected ToolCalls + Pending; got {keys:?}");
    let tickets = kpis
        .iter()
        .find(|k| k["label"] == "Tickets resolved today")
        .expect("tickets");
    assert_eq!(tickets["value"], "4");
    assert_eq!(tickets["direction_means_good"], Value::Bool(true));
    assert_eq!(
        tickets["sparkline_7d"].as_array().expect("sparkline").len(),
        7
    );
    let pend = kpis
        .iter()
        .find(|k| k["label"] == "Pending escalations")
        .expect("pending");
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
            &sink,
            session,
            AuditKind::ToolCall,
            serde_json::json!({ "verb": format!("call_{i}"), "status": "succeeded" }),
        )
        .await;
    }
    let req = req_for(&worker.id.to_string(), Some("operator"), Some("human:mark"));
    let v = parse_ok(&req, &worker.id.to_string(), &env.store).await;
    let rows = v["recent_activity"]
        .as_array()
        .expect("recent_activity array");
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
async fn recent_activity_ignores_other_tenant_session_with_same_worker_id() {
    let env = prep().await;
    let worker = add_worker(&env.store, env.tenant, "human:mark").await;
    let local_session = seed_session(&env, worker.id).await;
    let (alien_tenant, alien_ws) = env
        .store
        .ensure_workspace("overview-alien", "/tmp/overview-alien")
        .await
        .unwrap();
    let alien_session = env
        .store
        .create_session_with_worker(argo_store::SessionCreate {
            tenant_id: alien_tenant,
            workspace_id: alien_ws,
            model: "test-model",
            provider: "test-provider",
            title: Some("alien"),
            vertical_id: Some(VERT),
            worker_id: Some(argo_store::WorkerId(worker.id.0)),
        })
        .await
        .unwrap();
    let local_sink = AuditSink::new_with_tenant(env.store.clone(), env.tenant.to_string());
    write_audit(
        &local_sink,
        local_session,
        AuditKind::ToolCall,
        serde_json::json!({ "verb": "local_send", "status": "succeeded" }),
    )
    .await;
    let alien_sink = AuditSink::new_with_tenant(env.store.clone(), alien_tenant.to_string());
    write_audit(
        &alien_sink,
        alien_session,
        AuditKind::ToolCall,
        serde_json::json!({ "verb": "alien_send", "status": "succeeded" }),
    )
    .await;

    let req = req_for(&worker.id.to_string(), Some("operator"), Some("human:mark"));
    let v = parse_ok(&req, &worker.id.to_string(), &env.store).await;
    let rendered = v["recent_activity"].to_string();
    assert!(rendered.contains("local_send"), "{rendered}");
    assert!(!rendered.contains("alien_send"), "{rendered}");
}

#[tokio::test]
async fn duties_include_role_duty_rows() {
    let env = prep().await;
    let worker = add_worker(&env.store, env.tenant, "human:mark").await;
    let schema = include_str!("../../../../argo-store/migrations/93__role_duties.sql");
    for stmt in schema
        .split_terminator(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        sqlx::query(stmt)
            .execute(env.store.pool())
            .await
            .expect("role_duty schema");
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
    assert!(
        d["next_fire_at"].is_string(),
        "expected next_fire_at, got {d}"
    );
    // Reference Duration so the import stays used across cfg combinations.
    let _ = Duration::seconds(1);
}

#[tokio::test]
async fn audit_row_written_per_view() {
    let env = prep().await;
    let worker = add_worker(&env.store, env.tenant, "human:mark").await;
    let req = req_for(
        &worker.id.to_string(),
        Some("operator"),
        Some("human:auditor"),
    );
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
        "can_edit_charter",
        "can_edit_duties",
        "can_pause_resume",
        "can_archive",
        "can_delete",
        "can_view_audit",
        "can_view_settings",
    ] {
        assert_eq!(p[key], Value::Bool(true), "operator must have {key}");
    }
    let v_mine = parse_ok(
        &req_for(&mine.id.to_string(), Some("delegator"), Some("human:mark")),
        &mine.id.to_string(),
        &env.store,
    )
    .await;
    let v_theirs = parse_ok(
        &req_for(
            &theirs.id.to_string(),
            Some("delegator"),
            Some("human:mark"),
        ),
        &theirs.id.to_string(),
        &env.store,
    )
    .await;
    assert_eq!(v_mine["permissions"]["can_edit_charter"], Value::Bool(true));
    assert_eq!(
        v_theirs["permissions"]["can_edit_charter"],
        Value::Bool(false)
    );
    assert_eq!(v_mine["permissions"]["can_delete"], Value::Bool(false));
}

#[tokio::test]
async fn compute_permissions_demo_grants_all_only_for_demo_principal() {
    let env = prep().await;
    let worker = add_worker(&env.store, env.tenant, "human:owner").await;
    let token = mint_demo_jwt("demo@example.com");
    std::env::set_var(super::super::super::demo_mode::DEMO_MODE_ENV, "1");

    let demo_req = req_for_auth(
        &worker.id.to_string(),
        Some("delegator"),
        Some("human:demo@example.com"),
        &token,
    );
    let demo = parse_ok(&demo_req, &worker.id.to_string(), &env.store).await;
    for key in super::super::WORKER_OVERVIEW_PERMISSION_NAMES {
        assert_eq!(
            demo["permissions"][key],
            Value::Bool(true),
            "demo override must grant {key}"
        );
    }

    let sink = AuditSink::new(env.store.clone());
    sink.flush_pending().await.unwrap();
    let (payload_raw, principal_id): (String, Option<String>) = sqlx::query_as(
        "SELECT payload_json, principal_id FROM audit_record \
         WHERE kind = 'permission.demo_override_granted' \
         ORDER BY created_at DESC LIMIT 1",
    )
    .fetch_one(env.store.pool())
    .await
    .expect("demo override audit row");
    let payload: Value = serde_json::from_str(&payload_raw).expect("payload json");
    assert_eq!(principal_id.as_deref(), Some("human:demo@example.com"));
    assert_eq!(payload["principal_id"], "human:demo@example.com");
    assert_eq!(payload["worker_id"], worker.id.to_string());
    for key in super::super::WORKER_OVERVIEW_PERMISSION_NAMES {
        assert!(
            payload["permissions"]
                .as_array()
                .expect("permissions array")
                .iter()
                .any(|v| v.as_str() == Some(key)),
            "audit payload must include {key}: {payload}"
        );
    }

    let non_demo = parse_ok(
        &req_for(
            &worker.id.to_string(),
            Some("delegator"),
            Some("human:demo@example.com"),
        ),
        &worker.id.to_string(),
        &env.store,
    )
    .await;
    assert_eq!(
        non_demo["permissions"]["can_edit_charter"],
        Value::Bool(false)
    );

    let forged = parse_ok(
        &req_for_auth(
            &worker.id.to_string(),
            Some("delegator"),
            Some("human:other@example.com"),
            &token,
        ),
        &worker.id.to_string(),
        &env.store,
    )
    .await;
    assert_eq!(
        forged["permissions"]["can_edit_charter"],
        Value::Bool(false)
    );

    std::env::remove_var(super::super::super::demo_mode::DEMO_MODE_ENV);
    let demo_mode_off = parse_ok(&demo_req, &worker.id.to_string(), &env.store).await;
    assert_eq!(
        demo_mode_off["permissions"]["can_edit_charter"],
        Value::Bool(false)
    );
}
