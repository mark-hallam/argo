//! Unit tests for the SOC2 audit-trail report handler.
//!
//! Covers: empty window, all-five categories represented, each filter
//! kind (actor, action, tenant), markdown render shape, csv column
//! order. The full CLI path is exercised by
//! `tests/compliance_soc2_smoke.rs`.
use super::*;
use argo_audit::compute_chain_hash;
use argo_store::SessionStore;

async fn fresh_store() -> SessionStore {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    store
}

fn args(from: &str, to: &str) -> ComplianceSoc2Args {
    ComplianceSoc2Args {
        from: from.into(),
        to: to.into(),
        actor: None,
        action: None,
        tenant: None,
        format: "json".into(),
    }
}

#[tokio::test]
async fn empty_window_returns_empty_report_with_zeroed_categories() {
    let store = fresh_store().await;
    seed(&store, &[("auth.login", serde_json::json!({"actor":"a"}))]).await;
    let report = build_report(
        args("2000-01-01T00:00:00Z", "2000-01-02T00:00:00Z"),
        &store,
    )
    .await
    .unwrap();
    assert!(report.events.is_empty());
    for label in ["Access", "ChangeMgmt", "DataMgmt", "Security", "Other"] {
        let agg = report.categories.get(label).unwrap();
        assert_eq!(agg.count, 0);
        assert!(agg.first.is_none() && agg.last.is_none());
    }
}

#[tokio::test]
async fn one_event_per_category_aggregates_correctly() {
    let store = fresh_store().await;
    seed(
        &store,
        &[
            ("auth.login", serde_json::json!({"actor":"alice"})),
            (
                "worker.deploy",
                serde_json::json!({"actor":"bob","tenant_id":"t-1"}),
            ),
            ("compliance.export", serde_json::json!({"actor":"carol"})),
            ("policy.update", serde_json::json!({"actor":"dave"})),
            ("weird.unmapped", serde_json::json!({})),
        ],
    )
    .await;
    let (from, to) = window();
    let report = build_report(args(&from, &to), &store).await.unwrap();
    assert_eq!(report.events.len(), 5);
    for (cat, want) in [
        ("Access", 1),
        ("ChangeMgmt", 1),
        ("DataMgmt", 1),
        ("Security", 1),
        ("Other", 1),
    ] {
        assert_eq!(report.categories[cat].count, want, "{cat}");
        assert!(report.categories[cat].first.is_some());
        assert!(report.categories[cat].last.is_some());
    }
}

#[tokio::test]
async fn actor_filter_drops_non_matching_rows() {
    let store = fresh_store().await;
    seed(
        &store,
        &[
            ("auth.login", serde_json::json!({"actor":"alice"})),
            ("auth.login", serde_json::json!({"actor":"bob"})),
        ],
    )
    .await;
    let (from, to) = window();
    let mut a = args(&from, &to);
    a.actor = Some("bob".into());
    let report = build_report(a, &store).await.unwrap();
    assert_eq!(report.events.len(), 1);
    assert_eq!(report.events[0].actor.as_deref(), Some("bob"));
}

#[tokio::test]
async fn action_filter_drops_non_matching_rows() {
    let store = fresh_store().await;
    seed(
        &store,
        &[
            ("auth.login", serde_json::json!({"actor":"alice"})),
            ("auth.logout", serde_json::json!({"actor":"alice"})),
        ],
    )
    .await;
    let (from, to) = window();
    let mut a = args(&from, &to);
    a.action = Some("auth.login".into());
    let report = build_report(a, &store).await.unwrap();
    assert_eq!(report.events.len(), 1);
    assert_eq!(report.events[0].action, "auth.login");
}

#[tokio::test]
async fn tenant_filter_drops_non_matching_rows() {
    let store = fresh_store().await;
    seed(
        &store,
        &[
            (
                "auth.login",
                serde_json::json!({"actor":"alice","tenant_id":"t-1"}),
            ),
            (
                "auth.login",
                serde_json::json!({"actor":"bob","tenant_id":"t-2"}),
            ),
        ],
    )
    .await;
    let (from, to) = window();
    let mut a = args(&from, &to);
    a.tenant = Some("t-2".into());
    let report = build_report(a, &store).await.unwrap();
    assert_eq!(report.events.len(), 1);
    assert_eq!(report.events[0].tenant.as_deref(), Some("t-2"));
}

#[tokio::test]
async fn markdown_render_includes_every_category_and_event_table() {
    let store = fresh_store().await;
    seed(&store, &[("auth.login", serde_json::json!({"actor":"alice"}))]).await;
    let (from, to) = window();
    let report = build_report(args(&from, &to), &store).await.unwrap();
    let body = render_with_format(&report, ReportFormat::Markdown).unwrap();
    assert!(body.starts_with("# SOC2 audit trail report"));
    for cat in ["Access", "ChangeMgmt", "DataMgmt", "Security", "Other"] {
        assert!(body.contains(&format!("## {cat}")), "missing {cat}");
    }
    assert!(body.contains("## Events"));
    assert!(body.contains("auth.login") && body.contains("alice"));
}

#[tokio::test]
async fn csv_render_has_stable_column_order() {
    let store = fresh_store().await;
    seed(
        &store,
        &[(
            "auth.login",
            serde_json::json!({"actor":"alice","tenant_id":"t-1","target":"console"}),
        )],
    )
    .await;
    let (from, to) = window();
    let report = build_report(args(&from, &to), &store).await.unwrap();
    let body = render_with_format(&report, ReportFormat::Csv).unwrap();
    let mut lines = body.lines();
    assert_eq!(
        lines.next().unwrap(),
        "seq,id,ts,actor,tenant,action,category,target,session_id"
    );
    let cols: Vec<&str> = lines.next().unwrap().split(',').collect();
    assert_eq!(cols.len(), 9);
    assert_eq!(cols[3], "alice");
    assert_eq!(cols[4], "t-1");
    assert_eq!(cols[5], "auth.login");
    assert_eq!(cols[6], "Access");
    assert_eq!(cols[7], "console");
}

#[tokio::test]
async fn from_after_to_is_rejected() {
    let store = fresh_store().await;
    let err = build_report(
        args("2026-05-09T01:00:00Z", "2026-05-09T00:00:00Z"),
        &store,
    )
    .await
    .unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("--from") && msg.contains("--to"));
}

fn window() -> (String, String) {
    let now = chrono::Utc::now();
    (
        (now - chrono::Duration::hours(1)).to_rfc3339(),
        (now + chrono::Duration::hours(1)).to_rfc3339(),
    )
}

async fn seed(store: &SessionStore, rows: &[(&str, serde_json::Value)]) {
    let mut prev = [0_u8; 32];
    for (i, (kind, payload)) in rows.iter().enumerate() {
        let canonical = serde_json::to_string(payload).unwrap();
        let self_hash = compute_chain_hash(&prev, kind, &canonical);
        let id = uuid::Uuid::now_v7();
        let ts = chrono::Utc::now().timestamp_micros() + i as i64;
        sqlx::query(
            "INSERT INTO audit_record (id, session_id, kind, payload_json, parent_hash, self_hash, created_at) \
             VALUES (?, NULL, ?, ?, ?, ?, ?)",
        )
        .bind(id.as_bytes().to_vec())
        .bind(*kind)
        .bind(canonical)
        .bind(prev.as_slice())
        .bind(self_hash.as_slice())
        .bind(ts)
        .execute(store.pool())
        .await
        .unwrap();
        prev = self_hash;
    }
}
