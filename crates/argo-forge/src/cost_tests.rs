use crate::{
    load_forge_cost, load_forge_status, persist_forge_run, ForgeOptions, ForgeRun, ForgeStatus,
};
use argo_store::SessionStore;
use std::sync::Arc;

#[tokio::test]
async fn concurrent_spend_commits_keep_highest_observed_total() {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();

    let opts = ForgeOptions::new("alpha", "0.1");
    let run = ForgeRun::pending(&opts);
    persist_forge_run(&store, &run).await.unwrap();

    let mut tasks = Vec::new();
    for i in 1..=100_u64 {
        let store = Arc::clone(&store);
        let mut update = run.clone();
        update.status = ForgeStatus::Running;
        update.iterations = u32::try_from(i).unwrap();
        update.spend_usd_micros = i * 1_000;
        update.cost.llm_input_tokens = i;
        update.cost.llm_cost_usd_micros = i * 1_000;
        update.cost.total_cost_usd_micros = i * 1_000;
        tasks.push(tokio::spawn(async move {
            persist_forge_run(&store, &update).await.unwrap();
        }));
    }

    for task in tasks {
        task.await.unwrap();
    }

    let cost = load_forge_cost(&store, run.id).await.unwrap().unwrap();
    assert_eq!(cost.total_cost_usd_micros, 100_000);
    assert_eq!(cost.llm_cost_usd_micros, 100_000);
    assert_eq!(cost.llm_input_tokens, 100);

    let spend: i64 = sqlx::query_scalar("SELECT spend_usd_micros FROM forge_run WHERE id = ?")
        .bind(run.id.0.as_bytes().as_slice())
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(spend, 100_000);
}

#[tokio::test]
async fn stale_running_commit_does_not_reopen_budget_exceeded_run() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();

    let mut opts = ForgeOptions::new("alpha", "0.1");
    opts.budget_usd_micros = 50_000;
    let mut run = ForgeRun::pending(&opts);
    persist_forge_run(&store, &run).await.unwrap();

    let mut exceeded = run.clone();
    exceeded.status = ForgeStatus::BudgetExceeded;
    exceeded.iterations = 50;
    exceeded.spend_usd_micros = 50_000;
    exceeded.cost.llm_cost_usd_micros = 50_000;
    exceeded.cost.total_cost_usd_micros = 50_000;
    persist_forge_run(&store, &exceeded).await.unwrap();

    run.status = ForgeStatus::Running;
    run.iterations = 49;
    run.spend_usd_micros = 49_000;
    run.cost.llm_cost_usd_micros = 49_000;
    run.cost.total_cost_usd_micros = 49_000;
    persist_forge_run(&store, &run).await.unwrap();

    assert_eq!(
        load_forge_status(&store, run.id).await.unwrap(),
        Some(ForgeStatus::BudgetExceeded)
    );
    let cost = load_forge_cost(&store, run.id).await.unwrap().unwrap();
    assert_eq!(cost.total_cost_usd_micros, 50_000);
}
