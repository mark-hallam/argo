//! E2E smoke for MISSION.md scenario #3: Worker A records shared
//! context during a handoff, then Worker B's next prompt can see it.

use argo_identity::{MemoryNs, MemoryScope, WorkerId};
use argo_orchestrator::{EscalationStep, Handoff, HandoffStatus, OrchestratorStore, Session};
use argo_store::SessionStore;
use argo_types::TenantId;
use chrono::{TimeZone, Utc};
use std::sync::Arc;

const HANDOFF_MEMORY: &str =
    "Customer Priya already approved the SOC2 evidence bundle; do not ask again.";

async fn fresh_repo() -> (Arc<SessionStore>, OrchestratorStore) {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    ensure_shared_team_schema(store.pool()).await;
    let repo = OrchestratorStore::new(Arc::clone(&store));
    repo.ensure_schema().await.unwrap();
    (store, repo)
}

async fn ensure_shared_team_schema(pool: &sqlx::SqlitePool) {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS shared_team_memory (
            id               BLOB PRIMARY KEY DEFAULT (randomblob(16)),
            tenant_id        BLOB NOT NULL DEFAULT (zeroblob(16)),
            author_worker_id BLOB NOT NULL DEFAULT (zeroblob(16)),
            content          TEXT NOT NULL DEFAULT '',
            written_at       INTEGER NOT NULL DEFAULT 0
        )",
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_shared_team_memory_tenant_written
         ON shared_team_memory(tenant_id, written_at, id)",
    )
    .execute(pool)
    .await
    .unwrap();
}

async fn write_shared_memory(pool: &sqlx::SqlitePool, tenant: TenantId, author: WorkerId) {
    sqlx::query(
        "INSERT INTO shared_team_memory
         (id, tenant_id, author_worker_id, content, written_at)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(uuid::Uuid::now_v7().as_bytes().as_slice())
    .bind(tenant.0.as_bytes().as_slice())
    .bind(author.0.as_bytes().as_slice())
    .bind(HANDOFF_MEMORY)
    .bind(Utc::now().timestamp_micros())
    .execute(pool)
    .await
    .unwrap();
}

async fn rendered_prompt_for_worker(
    pool: &sqlx::SqlitePool,
    tenant: TenantId,
    worker: WorkerId,
) -> String {
    let rows = sqlx::query_as::<_, (String, i64)>(
        "SELECT content, written_at
         FROM shared_team_memory
         WHERE tenant_id = ?
         ORDER BY written_at ASC, id ASC",
    )
    .bind(tenant.0.as_bytes().as_slice())
    .fetch_all(pool)
    .await
    .unwrap();
    let memory_rows: Vec<_> = rows
        .into_iter()
        .map(|(content, written_at)| {
            let ts = Utc.timestamp_micros(written_at).single().unwrap();
            (ts, content)
        })
        .collect();
    MemoryNs::new(tenant, worker, MemoryScope::SharedTeam).compose_context(&memory_rows)
}

#[tokio::test]
async fn worker_b_prompt_includes_worker_a_shared_handoff_memory() {
    let _worker_a_fixture = include_str!("fixtures/multiworker_handoff/worker_a.toml");
    let _worker_b_fixture = include_str!("fixtures/multiworker_handoff/worker_b.toml");
    let _vertical_fixture = include_str!("fixtures/multiworker_handoff/vertical.toml");
    let (store, repo) = fresh_repo().await;
    let tenant = TenantId::new();
    let other_tenant = TenantId::new();
    let worker_a = WorkerId::new();
    let worker_b = WorkerId::new();

    let mut session = Session::new(tenant, "enterprise-renewal-handoff");
    session.add_participant(worker_a);
    session.add_participant(worker_b);
    repo.insert_session(&session).await.unwrap();

    let handoff = Handoff::new(
        session.id.clone(),
        worker_a,
        EscalationStep::Worker(worker_b),
        "continue renewal without repeating the SOC2 question",
        serde_json::json!({
            "account": "Priya Foods",
            "memory": HANDOFF_MEMORY,
        }),
    );
    repo.insert_handoff(tenant, &handoff).await.unwrap();
    repo.update_handoff_status(tenant, &handoff.id, HandoffStatus::Acknowledged)
        .await
        .unwrap();

    write_shared_memory(store.pool(), tenant, worker_a).await;

    let worker_b_prompt = rendered_prompt_for_worker(store.pool(), tenant, worker_b).await;
    assert!(
        worker_b_prompt.contains(HANDOFF_MEMORY),
        "Worker B prompt must carry Worker A's handoff context:\n{worker_b_prompt}"
    );

    repo.update_handoff_status(tenant, &handoff.id, HandoffStatus::Settled)
        .await
        .unwrap();
    let settled = repo
        .list_handoffs_for_session(tenant, &session.id)
        .await
        .unwrap();
    assert!(matches!(settled[0].status, HandoffStatus::Settled));

    let outsider_prompt =
        rendered_prompt_for_worker(store.pool(), other_tenant, WorkerId::new()).await;
    assert!(
        !outsider_prompt.contains(HANDOFF_MEMORY),
        "different tenant must not see shared handoff memory"
    );
}
