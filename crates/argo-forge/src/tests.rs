use super::*;
use argo_store::SessionStore;

#[tokio::test]
async fn persist_round_trips_a_pending_run() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let opts = ForgeOptions::new("alpha", "0.1");
    let run = ForgeRun::pending(&opts);
    persist_forge_run(&store, &run).await.unwrap();
    assert_eq!(count_forge_runs(&store).await.unwrap(), 1);
    let status = load_forge_status(&store, run.id).await.unwrap();
    assert_eq!(status, Some(ForgeStatus::Pending));
}

#[tokio::test]
async fn persist_overwrites_run_on_status_update() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let opts = ForgeOptions::new("alpha", "0.1");
    let mut run = ForgeRun::pending(&opts);
    persist_forge_run(&store, &run).await.unwrap();
    run.status = ForgeStatus::Completed;
    run.final_score = Some(0.92);
    run.candidate_version = Some("0.2-c1".into());
    persist_forge_run(&store, &run).await.unwrap();
    assert_eq!(count_forge_runs(&store).await.unwrap(), 1);
    assert_eq!(
        load_forge_status(&store, run.id).await.unwrap(),
        Some(ForgeStatus::Completed)
    );
}

#[tokio::test]
async fn budget_exceeded_run_persists_partial_state() {
    // §26.5 "hitting the budget cleanly aborts and persists partial
    // state" — exercise the end-to-end path.
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let mut opts = ForgeOptions::new("alpha", "0.1");
    opts.budget_usd_micros = 1_000;
    let run = ForgeRun::pending(&opts);
    let events = vec![
        ForgeEvent::Iteration {
            spend_delta_usd_micros: 700,
        },
        ForgeEvent::Iteration {
            spend_delta_usd_micros: 800,
        },
    ];
    let final_run = finalize_run(run, &events, opts.budget_usd_micros);
    assert_eq!(final_run.status, ForgeStatus::BudgetExceeded);
    persist_forge_run(&store, &final_run).await.unwrap();
    let status = load_forge_status(&store, final_run.id).await.unwrap();
    assert_eq!(status, Some(ForgeStatus::BudgetExceeded));
}

#[tokio::test]
async fn stub_forge_optimize_returns_provider_error() {
    let f = StubForge::new();
    let opts = ForgeOptions::new("alpha", "0.1");
    assert!(f.optimize(opts).await.is_err());
}
