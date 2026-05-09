use crate::SessionStore;
use sqlx::Row;

#[tokio::test]
async fn fresh_migrate_reaches_v33_and_creates_reconciled_schema() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();

    assert_eq!(store.schema_version().await.unwrap(), 33);
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
async fn raw_v12_to_v26_legacy_schema_migrates_without_duplicate_errors() {
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
    ] {
        apply_raw(&store, raw).await;
    }
    assert_eq!(store.schema_version().await.unwrap(), 11);

    store.migrate().await.unwrap();

    assert_eq!(store.schema_version().await.unwrap(), 33);
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
async fn partial_legacy_19_collision_pair_migrates_to_full_v33() {
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

    assert_eq!(store.schema_version().await.unwrap(), 33);
    assert!(table_exists(&store, "worker_anomaly_event").await);
    assert!(table_exists(&store, "worker_behavior_baseline").await);
    assert!(table_exists(&store, "skill_ab_test_run").await);
    assert_columns(&store, "worker", &["fs_root"]).await;
}

#[tokio::test]
async fn partial_legacy_22_collision_pair_migrates_to_full_v33() {
    // Lazy raw application of one half of each multi-prefix
    // collision (22, 23, 24, 26) — proves v23..v33 idempotency
    // when only some sibling files were applied historically.
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

    store.migrate().await.unwrap();

    assert_eq!(store.schema_version().await.unwrap(), 33);
    // Pre-applied raw siblings still present.
    assert!(table_exists(&store, "handoff_routing_rule").await);
    assert!(table_exists(&store, "skill_quality_score").await);
    assert!(table_exists(&store, "worker_budget_alert").await);
    assert_columns(&store, "workspace", &["archived_at", "owner_email"]).await;
    // The other half of each collision installed by migrate().
    assert!(table_exists(&store, "roster").await);
    assert!(table_exists(&store, "workflow_checkpoint").await);
    assert!(table_exists(&store, "operator_feedback").await);
    assert!(table_exists(&store, "vertical_lifecycle").await);
    assert!(table_exists(&store, "data_classification_tag").await);
    assert!(table_exists(&store, "worker_spend_baseline").await);
}

#[tokio::test]
async fn rollback_to_v16_drops_v17_through_v33_tables() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    assert_eq!(store.schema_version().await.unwrap(), 33);

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
async fn rollback_to_v22_drops_v23_through_v33_tables() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    assert_eq!(store.schema_version().await.unwrap(), 33);

    store.migrate_down_to(22).await.unwrap();

    assert_eq!(store.schema_version().await.unwrap(), 22);
    // v22 (org_chart) still present.
    assert!(table_exists(&store, "org_unit").await);
    assert!(table_exists(&store, "org_member").await);
    // v23..v33 unwound.
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
    // After v17..v33 unwound, v16 refuses and the schema_version
    // settles at 16 — the highest layer whose down() is supported.
    assert_eq!(store.schema_version().await.unwrap(), 16);
}

async fn baseline_v11() -> SessionStore {
    let store = SessionStore::open(":memory:").await.unwrap();
    crate::migrations::run_up_to(store.pool(), 11)
        .await
        .unwrap();
    store
}

async fn apply_raw(store: &SessionStore, body: &str) {
    // Strip `--` line comments before splitting on `;` so semicolons
    // inside SQL prose comments (e.g. `19__worker_fs_root.sql`) do
    // not split mid-statement.
    let stripped: String = body
        .lines()
        .map(|line| match line.find("--") {
            Some(idx) => &line[..idx],
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n");
    for stmt in split_with_trigger_awareness(&stripped) {
        match sqlx::query(stmt).execute(store.pool()).await {
            Ok(_) => {}
            Err(e)
                if stmt.contains("memory_vec_index")
                    && e.to_string().contains("no such module: vec0") => {}
            Err(e) => panic!("raw migration failed: {e}; stmt={stmt}"),
        }
    }
}

// Mirrors `tests/migration_chain.rs::split_sql_statements`: splits on
// `;` but treats a statement starting with `CREATE TRIGGER` as
// continuing until its `END;` so the inner `;` inside the
// `BEGIN ... END` body do not slice the trigger in half. Required
// for `22__tenant_archived.sql` once it is part of the v22..v33
// reconciliation walk.
fn split_with_trigger_awareness(sql: &str) -> Vec<&str> {
    let mut statements = Vec::new();
    let mut start = 0;
    let bytes = sql.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b';' {
            let stmt = sql[start..=index].trim();
            if !is_incomplete_trigger(stmt) {
                if !stmt.is_empty() {
                    statements.push(stmt);
                }
                start = index + 1;
            }
        }
        index += 1;
    }
    let tail = sql[start..].trim();
    if !tail.is_empty() {
        statements.push(tail);
    }
    statements
        .into_iter()
        .map(|s| s.trim_end_matches(';').trim())
        .filter(|s| !s.is_empty())
        .collect()
}

fn is_incomplete_trigger(stmt: &str) -> bool {
    stmt.trim_start()
        .get(..14)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("CREATE TRIGGER"))
        && !ends_with_end(stmt)
}

fn ends_with_end(stmt: &str) -> bool {
    let stmt = stmt.trim_end_matches(';').trim_end();
    stmt.get(stmt.len().saturating_sub(3)..)
        .is_some_and(|suffix| suffix.eq_ignore_ascii_case("END"))
}

async fn table_exists(store: &SessionStore, table: &str) -> bool {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type IN ('table','view') AND name = ?",
    )
    .bind(table)
    .fetch_optional(store.pool())
    .await
    .unwrap();
    row.is_some()
}

async fn assert_columns(store: &SessionStore, table: &str, columns: &[&str]) {
    let rows = sqlx::query(&format!("PRAGMA table_info({table})"))
        .fetch_all(store.pool())
        .await
        .unwrap();
    for column in columns {
        assert!(
            rows.iter()
                .any(|row| row.get::<String, _>("name") == *column),
            "missing {table}.{column}"
        );
    }
}

async fn assert_schema_version_singleton(store: &SessionStore) {
    let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM schema_version")
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(rows, 1);
}
