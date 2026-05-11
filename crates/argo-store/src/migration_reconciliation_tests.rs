use crate::SessionStore;
use sqlx::Row;

#[path = "migration_test_helpers.rs"]
mod helpers;
use helpers::{
    apply_raw, assert_columns, assert_schema_version_singleton, baseline_v11, table_exists,
};

#[tokio::test]
async fn fresh_migrate_reaches_v40_and_creates_reconciled_schema() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();

    assert_eq!(store.schema_version().await.unwrap(), 52);
    for table in [
        "persona",
        "worker",
        "duty",
        "worker_budget",
        "memory_entry_fts",
        "memory_embedding",
        "shared_team_memory",
        "handoff",
        "worker_circuit_history",
        "workflow_run",
        "worker_behavior_baseline",
        "worker_anomaly_event",
        "skill_ab_test_run",
        "skill_ab_test_outcome",
        // v22..v33 reconciled tables.
        "org_unit",
        "org_member",
        "org_member_alias",
        "org_reports_to",
        "roster",
        "roster_member",
        "handoff_routing_rule",
        "workflow_checkpoint",
        "operator_feedback",
        "skill_quality_score",
        "vertical_lifecycle",
        "policy_files",
        "data_classification_tag",
        "worker_spend_baseline",
        "worker_budget_alert",
        // v34..v39 reconciled tables (v40 is a no-op tombstone).
        "auction_log",
        "auction_bid",
        "worker_spend_anomaly",
        "recovery_cache",
        "role_duty",
        "duty_fired_session",
        "tenant_quota",
        "subtask_ledger",
    ] {
        assert!(table_exists(&store, table).await, "missing {table}");
    }
    assert_columns(
        &store,
        "role",
        &["worker_id", "charter", "kpis_json", "escalation"],
    )
    .await;
    assert_columns(
        &store,
        "memory_entry",
        &["layer", "tenant_id", "worker_id", "memory_scope"],
    )
    .await;
    assert_columns(
        &store,
        "session",
        &["status", "participants", "created_at", "deadline"],
    )
    .await;
    assert_columns(&store, "worker", &["fs_root"]).await;
    // v24 reconciles the 22__tenant_archived ALTERs.
    assert_columns(&store, "workspace", &["archived_at", "owner_email"]).await;
    assert_schema_version_singleton(&store).await;
}

#[tokio::test]
async fn raw_v12_to_v29_legacy_schema_migrates_without_duplicate_errors() {
    let store = baseline_v11().await;
    for raw in [
        include_str!("../migrations/12__worker_identity.sql"),
        include_str!("../migrations/13__memory_layers.sql"),
        include_str!("../migrations/14__worker_budget.sql"),
        include_str!("../migrations/15__shared_team_memory.sql"),
        include_str!("../migrations/16__orchestrator.sql"),
        include_str!("../migrations/17__worker_circuit_history.sql"),
        include_str!("../migrations/18__workflow_runs.sql"),
        include_str!("../migrations/19__anomaly.sql"),
        include_str!("../migrations/19__worker_fs_root.sql"),
        include_str!("../migrations/20__skill_ab_tests.sql"),
        include_str!("../migrations/21__org_chart.sql"),
        include_str!("../migrations/22__roster.sql"),
        include_str!("../migrations/22__tenant_archived.sql"),
        include_str!("../migrations/23__handoff_routing.sql"),
        include_str!("../migrations/23__workflow_checkpoints.sql"),
        include_str!("../migrations/24__operator_feedback.sql"),
        include_str!("../migrations/24__skill_quality_score.sql"),
        include_str!("../migrations/24__vertical_archived.sql"),
        include_str!("../migrations/25__policy_files.sql"),
        include_str!("../migrations/26__data_classification.sql"),
        include_str!("../migrations/26__spend_baseline.sql"),
        include_str!("../migrations/26__worker_budget_alert.sql"),
        include_str!("../migrations/27__handoff_auctions.sql"),
        include_str!("../migrations/27__spend_anomaly.sql"),
        include_str!("../migrations/28__recovery_cache.sql"),
        include_str!("../migrations/28__role_duties.sql"),
        include_str!("../migrations/28__tenant_quota.sql"),
        include_str!("../migrations/29__subtask_ledger.sql"),
    ] {
        apply_raw(&store, raw).await;
    }
    assert_eq!(store.schema_version().await.unwrap(), 11);

    store.migrate().await.unwrap();

    assert_eq!(store.schema_version().await.unwrap(), 52);
    for table in [
        "shared_team_memory",
        "handoff",
        "worker_circuit_history",
        "workflow_run",
        "worker_behavior_baseline",
        "worker_anomaly_event",
        "skill_ab_test_run",
        "skill_ab_test_outcome",
        "org_unit",
        "roster",
        "handoff_routing_rule",
        "workflow_checkpoint",
        "operator_feedback",
        "skill_quality_score",
        "vertical_lifecycle",
        "policy_files",
        "data_classification_tag",
        "worker_spend_baseline",
        "worker_budget_alert",
        // v34..v39 reconciled tables.
        "auction_log",
        "auction_bid",
        "worker_spend_anomaly",
        "recovery_cache",
        "role_duty",
        "duty_fired_session",
        "tenant_quota",
        "subtask_ledger",
    ] {
        assert!(table_exists(&store, table).await, "missing {table}");
    }
    assert_columns(
        &store,
        "session",
        &["status", "participants", "created_at", "deadline"],
    )
    .await;
    assert_columns(&store, "worker", &["fs_root"]).await;
    assert_columns(&store, "workspace", &["archived_at", "owner_email"]).await;
    assert_schema_version_singleton(&store).await;
}

#[tokio::test]
async fn partial_legacy_19_collision_pair_migrates_to_full_v40() {
    let store = baseline_v11().await;
    apply_raw(
        &store,
        include_str!("../migrations/12__worker_identity.sql"),
    )
    .await;
    apply_raw(&store, include_str!("../migrations/19__anomaly.sql")).await;
    apply_raw(
        &store,
        include_str!("../migrations/19__worker_fs_root.sql"),
    )
    .await;

    store.migrate().await.unwrap();

    assert_eq!(store.schema_version().await.unwrap(), 52);
    assert!(table_exists(&store, "worker_anomaly_event").await);
    assert!(table_exists(&store, "worker_behavior_baseline").await);
    assert!(table_exists(&store, "skill_ab_test_run").await);
    assert_columns(&store, "worker", &["fs_root"]).await;
}

#[tokio::test]
async fn partial_legacy_28_collision_trio_migrates_to_full_v40() {
    // Lazy raw application of one half of each multi-prefix
    // collision (22, 23, 24, 26, 27, 28) — proves v23..v40 idempotency
    // when only some sibling files were applied historically. The 28
    // group adds three siblings: recovery_cache (v36) preapplied,
    // role_duties (v37) and tenant_quota (v38) installed by migrate().
    let store = baseline_v11().await;
    apply_raw(
        &store,
        include_str!("../migrations/12__worker_identity.sql"),
    )
    .await;
    apply_raw(
        &store,
        include_str!("../migrations/22__tenant_archived.sql"),
    )
    .await;
    apply_raw(
        &store,
        include_str!("../migrations/23__handoff_routing.sql"),
    )
    .await;
    apply_raw(
        &store,
        include_str!("../migrations/24__skill_quality_score.sql"),
    )
    .await;
    apply_raw(
        &store,
        include_str!("../migrations/26__worker_budget_alert.sql"),
    )
    .await;
    apply_raw(
        &store,
        include_str!("../migrations/27__handoff_auctions.sql"),
    )
    .await;
    apply_raw(
        &store,
        include_str!("../migrations/28__recovery_cache.sql"),
    )
    .await;

    store.migrate().await.unwrap();

    assert_eq!(store.schema_version().await.unwrap(), 52);
    // Pre-applied raw siblings still present.
    assert!(table_exists(&store, "handoff_routing_rule").await);
    assert!(table_exists(&store, "skill_quality_score").await);
    assert!(table_exists(&store, "worker_budget_alert").await);
    assert!(table_exists(&store, "auction_log").await);
    assert!(table_exists(&store, "recovery_cache").await);
    assert_columns(&store, "workspace", &["archived_at", "owner_email"]).await;
    // The other half of each collision installed by migrate().
    assert!(table_exists(&store, "roster").await);
    assert!(table_exists(&store, "workflow_checkpoint").await);
    assert!(table_exists(&store, "operator_feedback").await);
    assert!(table_exists(&store, "vertical_lifecycle").await);
    assert!(table_exists(&store, "data_classification_tag").await);
    assert!(table_exists(&store, "worker_spend_baseline").await);
    assert!(table_exists(&store, "worker_spend_anomaly").await);
    assert!(table_exists(&store, "role_duty").await);
    assert!(table_exists(&store, "tenant_quota").await);
    assert!(table_exists(&store, "subtask_ledger").await);
}

#[tokio::test]
async fn rollback_to_v16_drops_v17_through_v40_tables() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    assert_eq!(store.schema_version().await.unwrap(), 52);

    store.migrate_down_to(16).await.unwrap();

    assert_eq!(store.schema_version().await.unwrap(), 16);
    assert!(table_exists(&store, "handoff").await);
    assert_columns(
        &store,
        "session",
        &["status", "participants", "created_at", "deadline"],
    )
    .await;
    for dropped in [
        "worker_circuit_history",
        "workflow_run",
        "worker_behavior_baseline",
        "worker_anomaly_event",
        "skill_ab_test_run",
        "skill_ab_test_outcome",
        "org_unit",
        "org_member",
        "org_member_alias",
        "org_reports_to",
        "roster",
        "roster_member",
        "handoff_routing_rule",
        "workflow_checkpoint",
        "operator_feedback",
        "skill_quality_score",
        "vertical_lifecycle",
        "policy_files",
        "data_classification_tag",
        "worker_spend_baseline",
        "worker_budget_alert",
        // v34..v39 reconciled tables.
        "auction_log",
        "auction_bid",
        "worker_spend_anomaly",
        "recovery_cache",
        "role_duty",
        "duty_fired_session",
        "tenant_quota",
        "subtask_ledger",
    ] {
        assert!(
            !table_exists(&store, dropped).await,
            "expected {dropped} dropped after roll_back_to(v16)"
        );
    }
    let worker_columns = sqlx::query("PRAGMA table_info(worker)")
        .fetch_all(store.pool())
        .await
        .unwrap();
    assert!(
        !worker_columns
            .iter()
            .any(|row| row.get::<String, _>("name") == "fs_root"),
        "expected worker.fs_root dropped after roll_back_to(v16)"
    );
    let workspace_columns = sqlx::query("PRAGMA table_info(workspace)")
        .fetch_all(store.pool())
        .await
        .unwrap();
    for dropped_col in ["archived_at", "owner_email"] {
        assert!(
            !workspace_columns
                .iter()
                .any(|row| row.get::<String, _>("name") == dropped_col),
            "expected workspace.{dropped_col} dropped after roll_back_to(v16)"
        );
    }
}

#[tokio::test]
async fn rollback_to_v22_drops_v23_through_v40_tables() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    assert_eq!(store.schema_version().await.unwrap(), 52);

    store.migrate_down_to(22).await.unwrap();

    assert_eq!(store.schema_version().await.unwrap(), 22);
    // v22 (org_chart) still present.
    assert!(table_exists(&store, "org_unit").await);
    assert!(table_exists(&store, "org_member").await);
    // v23..v40 unwound.
    for dropped in [
        "roster",
        "roster_member",
        "handoff_routing_rule",
        "workflow_checkpoint",
        "operator_feedback",
        "skill_quality_score",
        "vertical_lifecycle",
        "policy_files",
        "data_classification_tag",
        "worker_spend_baseline",
        "worker_budget_alert",
        // v34..v39 reconciled tables.
        "auction_log",
        "auction_bid",
        "worker_spend_anomaly",
        "recovery_cache",
        "role_duty",
        "duty_fired_session",
        "tenant_quota",
        "subtask_ledger",
    ] {
        assert!(
            !table_exists(&store, dropped).await,
            "expected {dropped} dropped after roll_back_to(v22)"
        );
    }
    let workspace_columns = sqlx::query("PRAGMA table_info(workspace)")
        .fetch_all(store.pool())
        .await
        .unwrap();
    for dropped_col in ["archived_at", "owner_email"] {
        assert!(
            !workspace_columns
                .iter()
                .any(|row| row.get::<String, _>("name") == dropped_col),
            "expected workspace.{dropped_col} dropped after roll_back_to(v22)"
        );
    }
}

#[tokio::test]
async fn rollback_to_v11_returns_clear_unsupported_error() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();

    let err = store.migrate_down_to(11).await.expect_err("unsupported");
    assert!(err.to_string().contains("rollback unsupported"));
    // After v17..v40 unwound, v16 refuses and the schema_version
    // settles at 16 — the highest layer whose down() is supported.
    assert_eq!(store.schema_version().await.unwrap(), 16);
}

