//! CLI smoke for `argo worker costs`.

mod common;

use common::product_e2e_helpers::{assert_success, run_argo};
use sqlx::SqlitePool;
use std::path::Path;
use uuid::Uuid;

const WORKER_BUDGET_SCHEMA: &str =
    include_str!("../../argo-store/migrations/14__worker_budget.sql");

async fn seed_charge(home: &Path, tenant: Uuid, worker: &str, action: &str, micros: i64) {
    let url = format!("sqlite://{}", home.join("sessions.db").display());
    let pool = SqlitePool::connect(&url).await.expect("open sessions.db");
    for stmt in WORKER_BUDGET_SCHEMA.split_terminator(';') {
        let sql = stmt.trim();
        if !sql.is_empty() {
            sqlx::query(sql).execute(&pool).await.expect("worker_budget schema");
        }
    }
    sqlx::query(
        "INSERT INTO worker_budget \
         (tenant_id, worker_id, action_id, usd_micros, charged_at) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(tenant.as_bytes().as_slice())
    .bind(worker)
    .bind(action)
    .bind(micros)
    .bind(1_778_256_000_i64)
    .execute(&pool)
    .await
    .expect("insert worker_budget charge");
}

#[tokio::test]
async fn worker_costs_json_reports_total_and_breakdown() {
    let home = tempfile::tempdir().expect("tempdir for ARGO_HOME");
    let work = tempfile::tempdir().expect("tempdir for workspace");
    let init = run_argo(home.path(), work.path(), &["workspace", "init"]);
    assert_success("workspace init", &init);
    let tenant = Uuid::from_u128(42);
    seed_charge(home.path(), tenant, "worker-a", "act-1", 2_500_000).await;

    let out = run_argo(
        home.path(),
        work.path(),
        &[
            "worker",
            "costs",
            "worker-a",
            "--tenant",
            &tenant.to_string(),
            "--breakdown",
            "action",
            "--json",
        ],
    );
    assert_success("worker costs", &out);
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("worker costs JSON");
    assert_eq!(v["worker_id"], "worker-a");
    assert_eq!(v["total_usd_micros"], 2_500_000);
    assert_eq!(v["row_count"], 1);
    assert_eq!(v["breakdown_rows"][0]["key"], "act-1");
    assert_eq!(v["recent_charges"][0]["action_id"], "act-1");
}
