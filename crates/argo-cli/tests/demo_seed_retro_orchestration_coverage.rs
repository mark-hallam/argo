//! Acceptance test for brief
//! `rebase-fix-demo-seed-retro-and-orchestration-empty-state`
//! (supersedes the original `fix-…` brief / closed PR #2587).
//!
//! Closes UX audit `whole-site F08`: `/retro` and `/orchestration`
//! rendered empty-state dead-ends in the authed demo session because
//! the seed wrote zero rows for either. This test exercises the same
//! SQL paths the gateway endpoints
//! (`crates/argo-cli/src/handlers/gateway/routes_retros.rs` index +
//! `crates/argo-cli/src/handlers/gateway/routes_orchestration.rs`
//! list) use, so green here means the SPA pages render real rows.

use argo_demo_seed::{run_canonical_v1_seed, SeedOptions};
use argo_store::retro::RetroStore;
use argo_store::SessionStore;
use argo_types::TenantId;
use sqlx::Row;
use std::sync::Arc;

async fn open_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    Arc::new(store)
}

fn skip_pass_a(tenant: TenantId) -> SeedOptions {
    let mut opts = SeedOptions::canonical_v1(tenant);
    opts.skip_pass_a_turns = true;
    opts.dressing_audit_rows = 0;
    opts
}

#[tokio::test]
async fn demo_seed_writes_three_retros_visible_via_index_endpoint_path() {
    let store = open_store().await;
    let tenant = TenantId::new();

    let report = run_canonical_v1_seed(&store, &skip_pass_a(tenant))
        .await
        .unwrap();
    assert_eq!(report.retros_created, 3);

    // Exercise the same RetroStore::list_all path the
    // `/api/v1/retros` index endpoint uses.
    let rows = RetroStore::new(store.pool())
        .list_all(tenant.0.as_bytes(), None, 100)
        .await
        .unwrap();
    assert!(
        rows.len() >= 3,
        "expected >= 3 retros via list_all; got {}",
        rows.len()
    );
    for r in &rows {
        for key in [
            "surface_metrics",
            "what_went_well",
            "surprises",
            "what_to_change",
            "action_items",
        ] {
            assert!(
                r.sections_json[key].as_str().is_some_and(|s| !s.is_empty()),
                "retro section `{key}` empty for the card preview (id {})",
                r.id
            );
        }
    }
}

#[tokio::test]
async fn demo_seed_writes_two_orchestration_runs_visible_via_provider_filter() {
    let store = open_store().await;
    let tenant = TenantId::new();

    let report = run_canonical_v1_seed(&store, &skip_pass_a(tenant))
        .await
        .unwrap();
    assert_eq!(report.orchestration_runs_created, 2);

    // Exercise the same SQL the `/api/v1/orchestration` list endpoint
    // uses (provider = 'orchestrator' filter, started_at DESC).
    let rows = sqlx::query(
        "SELECT id, title, status, started_at FROM session \
         WHERE tenant_id = ? AND provider = 'orchestrator' \
         ORDER BY started_at DESC LIMIT 200",
    )
    .bind(tenant.0.as_bytes().as_slice())
    .fetch_all(store.pool())
    .await
    .unwrap();
    assert!(
        rows.len() >= 2,
        "expected >= 2 orchestrator-provider session rows; got {}",
        rows.len()
    );
    let titles: Vec<String> = rows
        .iter()
        .map(|r| r.try_get::<String, _>("title").unwrap())
        .collect();
    assert!(
        titles.iter().any(|t| t.contains("Marketplace listing")),
        "Mariah-led re-price-weekly session missing; got {titles:?}"
    );
    assert!(
        titles.iter().any(|t| t.contains("project-coordination")),
        "Peta-led onboarding-handoff session missing; got {titles:?}"
    );
}

#[tokio::test]
async fn sibling_tenant_sees_zero_retros_and_zero_orchestration() {
    let store = open_store().await;
    let tenant = TenantId::new();
    let sibling = TenantId::new();

    run_canonical_v1_seed(&store, &skip_pass_a(tenant))
        .await
        .unwrap();

    let other_retros = RetroStore::new(store.pool())
        .list_all(sibling.0.as_bytes(), None, 100)
        .await
        .unwrap();
    assert!(
        other_retros.is_empty(),
        "sibling tenant must not see seeded retros"
    );

    let other_orch_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM session \
         WHERE tenant_id = ? AND provider = 'orchestrator'",
    )
    .bind(sibling.0.as_bytes().as_slice())
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert_eq!(
        other_orch_count, 0,
        "sibling tenant must not see seeded orchestration runs"
    );
}
