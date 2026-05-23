//! Per-worker `MemoryNs` scope contract pins for `MemoryStore`.
//!
//! Verifies the SQL bind layer in `crate::layers::layer_rows` honors
//! `(tenant_id, worker_id, memory_scope)` — i.e. two workers inside
//! one tenant cannot read each other's `MemoryScope::WorkerSelf`
//! rows, a `WorkerSelf` write is invisible to a `SharedTeam` or
//! `Role` scope read, and the cross-tenant boundary still blocks
//! identical `worker_id`s across tenants.
//!
//! Brief: `eval-memory-per-worker-scope-honored` — per-worker scope
//! verification at the SQL bind layer (mission-claim-1 substrate).
//! Companion report:
//! `docs/skycot/evals/scheduled/2026-05-23/eval-memory-per-worker-scope-honored.md`.

use std::sync::Arc;

use argo_identity::{MemoryNs, MemoryScope, WorkerId};
use argo_memory::{
    shared_team, HaikuSummaryProvider, MemoryFile, MemoryLayer, MemoryStore,
};
use argo_store::SessionStore;
use argo_types::{TenantId, WorkspaceId};
use chrono::{TimeZone, Utc};

struct Fixture {
    memory: MemoryStore,
    session: Arc<SessionStore>,
    workspace: WorkspaceId,
    tenant: TenantId,
}

async fn fresh() -> Fixture {
    let store = SessionStore::open(":memory:").await.expect("open store");
    store.migrate().await.expect("migrate");
    let tenant = TenantId::new();
    let workspace = WorkspaceId::new();
    sqlx::query(
        "INSERT INTO workspace (id, tenant_id, name, root_path, created_at) \
         VALUES (?, ?, 'per-worker-scope-test', '.', strftime('%s','now'))",
    )
    .bind(workspace.0.as_bytes().as_slice())
    .bind(tenant.0.as_bytes().as_slice())
    .execute(store.pool())
    .await
    .expect("seed workspace");
    let session = Arc::new(store);
    let memory = MemoryStore::new(Arc::clone(&session));
    Fixture {
        memory,
        session,
        workspace,
        tenant,
    }
}

#[tokio::test]
async fn cross_worker_query_same_tenant_returns_empty() {
    // Pins `layer_rows` filter `worker_id = ?` — a second worker in
    // the SAME tenant cannot see WorkerSelf indexed rows from worker A.
    let fx = fresh().await;
    let worker_a = WorkerId::new();
    let worker_b = WorkerId::new();
    let ns_a = MemoryNs::new(fx.tenant, worker_a, MemoryScope::WorkerSelf);
    let ns_b = MemoryNs::new(fx.tenant, worker_b, MemoryScope::WorkerSelf);

    fx.memory
        .append_in_namespace_with_layer(
            &ns_a,
            fx.workspace,
            MemoryFile::Memory,
            "private refund escalation note".into(),
            None,
            MemoryLayer::Indexed,
        )
        .await
        .expect("seed indexed row for A");

    let a_hits = fx.memory.query(&ns_a, "refund", 10).await.expect("query A");
    assert_eq!(a_hits.len(), 1, "A must recover its own row");

    let b_hits = fx.memory.query(&ns_b, "refund", 10).await.expect("query B");
    assert!(
        b_hits.is_empty(),
        "worker B must not see A's WorkerSelf row, got {b_hits:?}",
    );
}

#[tokio::test]
async fn cross_worker_compose_context_same_tenant_excludes_other_worker_rows() {
    // Pins compose_context: summary + recent + indexed layers all
    // filter by `(tenant_id, worker_id, memory_scope)`. Worker B's
    // pack must be empty even though A has seeded all three layers.
    let fx = fresh().await;
    let ns_a = MemoryNs::new(fx.tenant, WorkerId::new(), MemoryScope::WorkerSelf);
    let ns_b = MemoryNs::new(fx.tenant, WorkerId::new(), MemoryScope::WorkerSelf);

    for layer in [MemoryLayer::Recent, MemoryLayer::Indexed, MemoryLayer::Summary] {
        fx.memory
            .append_in_namespace_with_layer(
                &ns_a,
                fx.workspace,
                MemoryFile::Memory,
                format!("worker A {} row", layer.as_str()),
                None,
                layer,
            )
            .await
            .expect("seed A row");
    }

    let pack_a = fx
        .memory
        .compose_context(&ns_a, "worker")
        .await
        .expect("compose A");
    assert!(
        pack_a.entries.len() >= 3,
        "A must see its own three rows, got {}",
        pack_a.entries.len(),
    );

    let pack_b = fx
        .memory
        .compose_context(&ns_b, "worker")
        .await
        .expect("compose B");
    assert!(
        pack_b.entries.is_empty(),
        "worker B compose_context must be empty, got entries={:?}",
        pack_b
            .entries
            .iter()
            .map(|e| e.entry.body.clone())
            .collect::<Vec<_>>(),
    );
    assert_eq!(pack_b.token_estimate, 0);
}

#[tokio::test]
async fn cross_tenant_worker_self_query_returns_empty_even_when_worker_id_repeats() {
    // Re-pin the cross-tenant boundary at the per-worker layer: even
    // if tenant_b's worker accidentally shares an id with tenant_a's
    // worker, the `tenant_id = ?` bind keeps the row invisible.
    let fx = fresh().await;
    let tenant_b = TenantId::new();
    let shared_worker = WorkerId::new();
    let ns_a = MemoryNs::new(fx.tenant, shared_worker, MemoryScope::WorkerSelf);
    let ns_b = MemoryNs::new(tenant_b, shared_worker, MemoryScope::WorkerSelf);

    fx.memory
        .append_in_namespace_with_layer(
            &ns_a,
            fx.workspace,
            MemoryFile::Memory,
            "tenant-A confidential".into(),
            None,
            MemoryLayer::Indexed,
        )
        .await
        .expect("seed A row");

    let b_hits = fx
        .memory
        .query(&ns_b, "confidential", 10)
        .await
        .expect("query B");
    assert!(
        b_hits.is_empty(),
        "tenant B must not see tenant A's WorkerSelf row, got {b_hits:?}",
    );
}

#[tokio::test]
async fn worker_self_writes_invisible_to_shared_team_scope_read_on_same_ns() {
    // The `layer_rows` SQL binds `memory_scope = ?`. A WorkerSelf
    // write must not surface under a SharedTeam read for the same
    // tenant+worker triple — the scopes occupy disjoint memory.
    let fx = fresh().await;
    let worker = WorkerId::new();
    let ns_self = MemoryNs::new(fx.tenant, worker, MemoryScope::WorkerSelf);
    let ns_shared = MemoryNs::new(fx.tenant, worker, MemoryScope::SharedTeam);

    fx.memory
        .append_in_namespace_with_layer(
            &ns_self,
            fx.workspace,
            MemoryFile::Memory,
            "private worker_self note".into(),
            None,
            MemoryLayer::Indexed,
        )
        .await
        .expect("seed worker_self row");

    let shared_hits = fx
        .memory
        .query(&ns_shared, "private", 10)
        .await
        .expect("query shared");
    assert!(
        shared_hits.is_empty(),
        "shared_team scope must not surface a worker_self row, got {shared_hits:?}",
    );
    let pack = fx
        .memory
        .compose_context(&ns_shared, "note")
        .await
        .expect("compose shared");
    assert!(pack.entries.is_empty());
}

#[tokio::test]
async fn worker_self_writes_invisible_to_role_scope_read_on_same_ns() {
    // Same proof as the shared_team test, against the third scope.
    let fx = fresh().await;
    let worker = WorkerId::new();
    let ns_self = MemoryNs::new(fx.tenant, worker, MemoryScope::WorkerSelf);
    let ns_role = MemoryNs::new(fx.tenant, worker, MemoryScope::Role);

    fx.memory
        .append_in_namespace_with_layer(
            &ns_self,
            fx.workspace,
            MemoryFile::Memory,
            "worker_self role-leak probe".into(),
            None,
            MemoryLayer::Indexed,
        )
        .await
        .expect("seed worker_self row");

    let role_hits = fx
        .memory
        .query(&ns_role, "probe", 10)
        .await
        .expect("query role");
    assert!(
        role_hits.is_empty(),
        "role scope must not surface a worker_self row, got {role_hits:?}",
    );
}

#[tokio::test]
async fn shared_team_table_isolates_by_tenant_independent_of_worker_self_scope() {
    // The `shared_team_memory` table is a parallel surface — its
    // `tenant_id` filter is the only isolation. Pin that the
    // shared-team `tenant_id = ?` bind keeps tenants apart even
    // when worker A has also written private `worker_self` rows.
    let fx = fresh().await;
    let tenant_b = TenantId::new();
    let worker_a = WorkerId::new();
    let ns_a = MemoryNs::new(fx.tenant, worker_a, MemoryScope::WorkerSelf);

    fx.memory
        .append_in_namespace_with_layer(
            &ns_a,
            fx.workspace,
            MemoryFile::Memory,
            "private worker_self handoff".into(),
            None,
            MemoryLayer::Recent,
        )
        .await
        .expect("seed worker_self row");
    fx.memory
        .write_shared_team(
            &shared_team(fx.tenant),
            worker_a,
            "team-A shared handoff".into(),
        )
        .await
        .expect("seed shared row for A");

    let shared_a = fx
        .memory
        .read_shared_team(&shared_team(fx.tenant))
        .await
        .expect("read shared A");
    assert_eq!(shared_a.len(), 1, "shared-team table has only the shared row");
    assert_eq!(shared_a[0].content, "team-A shared handoff");

    let shared_b = fx
        .memory
        .read_shared_team(&shared_team(tenant_b))
        .await
        .expect("read shared B");
    assert!(
        shared_b.is_empty(),
        "tenant B must not see tenant A's shared row",
    );
}

#[tokio::test]
async fn nightly_summary_groups_one_summary_per_worker_in_same_tenant() {
    // Pins `summary_namespaces` GROUP BY `(tenant_id, worker_id,
    // memory_scope)` + `summary_exists` per-worker filter: two
    // workers in one tenant get two distinct summary rows.
    let fx = fresh().await;
    let ns_a = MemoryNs::new(fx.tenant, WorkerId::new(), MemoryScope::WorkerSelf);
    let ns_b = MemoryNs::new(fx.tenant, WorkerId::new(), MemoryScope::WorkerSelf);
    let prior_month = Utc.with_ymd_and_hms(2026, 4, 10, 8, 0, 0).unwrap();

    for ns in [&ns_a, &ns_b] {
        let row = fx
            .memory
            .append_in_namespace(
                ns,
                fx.workspace,
                MemoryFile::Memory,
                format!("april handoff for worker {}", ns.worker_id),
                None,
            )
            .await
            .expect("seed recent row");
        // `append_in_namespace` stamps learned_at = now(); the
        // nightly job looks for rows inside the *previous* calendar
        // month. Backdate via the shared SessionStore pool so the
        // seeded row falls inside `summary_namespaces`'s window.
        sqlx::query("UPDATE memory_entry SET learned_at = ? WHERE id = ?")
            .bind(prior_month.timestamp_micros())
            .bind(row.id.as_bytes().as_slice())
            .execute(fx.session.pool())
            .await
            .expect("backdate row");
    }

    let now = Utc.with_ymd_and_hms(2026, 5, 6, 2, 0, 0).unwrap();
    let report = fx
        .memory
        .run_nightly_summary_job(fx.workspace, now, &HaikuSummaryProvider)
        .await
        .expect("nightly summary");
    assert_eq!(report.month.label(), "2026-04");
    assert_eq!(report.workers_seen, 2, "summary GROUP BY must surface both workers");
    assert_eq!(report.summaries_written, 2);
}
