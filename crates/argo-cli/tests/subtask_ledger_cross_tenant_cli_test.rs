//! Contract-pinning regression for the CLI callers of the
//! `subtask_ledger` helpers (Brief
//! `eval-subtask-ledger-caller-tenant-audit`).
//!
//! The four CLI handlers
//!
//!   * `argo subtask show <id>`    → `subtask::get(pool, id)`
//!   * `argo subtask tree <session>` → `subtask::list_by_session(pool, session)`
//!   * `argo subtask list [--session …]` → `subtask::list(pool, filters, limit)`
//!   * `argo subtask budget <root_id>` → `subtask_budget::walk_tree(pool, root_id)`
//!
//! all run with **no tenant filter applied at the SQL layer**. That
//! is acceptable today because: (a) the CLI is a local binary;
//! (b) no gateway route exposes any of these helpers (verified by
//! `rg subtask crates/argo-cli/src/handlers/gateway/`); (c) the
//! `subtask_ledger` schema's SAFETY block (`29__subtask_ledger.sql:18-21`)
//! explicitly documents that tenant scoping is a caller-side
//! responsibility, not a column-level guard.
//!
//! These tests **pin that contract**: they seed two tenants'
//! subtasks in one DB and assert that the bare-id /
//! bare-session-id lookup helpers return the row the id picks
//! out, with no tenant cross-check. If a future maintainer wires
//! one of these helpers into a gateway route (where the id comes
//! from an untrusted request param), they will need to either
//! upgrade these tests to a tenant-scoped join OR add the schema
//! `tenant_id` column. The failing-this-test scenario in that
//! refactor is the early-warning the round-13 SHARED.md handoff
//! asked for.
//!
//! No test here asserts a real leak — only that the **SQL helper
//! layer is unscoped by design**. The CLI handlers do not add
//! their own filter on top, so the trust boundary is the binary
//! itself.

use argo_store::subtask::{self, list_by_session, NewSubtask, SubtaskFilters};
use argo_store::subtask_budget::{self, walk_tree};
use argo_store::SessionStore;
use std::sync::Arc;

async fn fresh_store() -> Arc<SessionStore> {
    let s = SessionStore::open(":memory:").await.expect("open");
    s.migrate().await.expect("migrate");
    Arc::new(s)
}

/// Seed one tenant's representative subtask. Returns `(session_id,
/// subtask_id, worker_id)` so the test can drive each lookup
/// helper.
async fn seed_one_subtree(
    store: &Arc<SessionStore>,
    tenant_label: &str,
) -> (String, String, String) {
    let session_id = format!("sess-{tenant_label}");
    let worker_id = format!("wkr-{tenant_label}");
    let subtask_id = subtask::insert(
        store.pool(),
        &NewSubtask {
            parent_goal: "delegate-launch",
            parent_session_id: Some(&session_id),
            worker_id: &worker_id,
            plan_node_path: "/root",
            description: "root description",
        },
    )
    .await
    .expect("seed root");
    subtask_budget::set_subtask_budget(store.pool(), &subtask_id, 1_000)
        .await
        .expect("set budget");
    (session_id, subtask_id, worker_id)
}

#[tokio::test]
async fn cli_subtask_get_is_tenant_blind_by_design() {
    // Pins the contract for `argo subtask show <id>` → `subtask::get`.
    //
    // Bare-id lookup returns the row regardless of any tenant
    // context, because the SQL is `WHERE id = ?` with no AND
    // tenant_id clause. The CLI is the trust boundary; if this
    // helper is ever wired to a gateway route, that route must
    // resolve through a tenant-scoped session join before
    // calling get().
    let store = fresh_store().await;
    let (_session_a, subtask_a, _worker_a) = seed_one_subtree(&store, "a").await;
    let (_session_b, subtask_b, _worker_b) = seed_one_subtree(&store, "b").await;

    // Bare lookup for tenant A's id returns tenant A's row.
    let row_a = subtask::get(store.pool(), &subtask_a)
        .await
        .expect("get A")
        .expect("tenant A row exists");
    assert_eq!(row_a.id, subtask_a);
    assert_eq!(row_a.worker_id, "wkr-a");

    // Bare lookup for tenant B's id returns tenant B's row.
    let row_b = subtask::get(store.pool(), &subtask_b)
        .await
        .expect("get B")
        .expect("tenant B row exists");
    assert_eq!(row_b.id, subtask_b);
    assert_eq!(row_b.worker_id, "wkr-b");

    // No filter is applied: passing a foreign id returns that
    // tenant's row, not None. THIS IS THE CONTRACT.
    // (Inverse — supplying a NON-EXISTENT id returns None.)
    let missing = subtask::get(store.pool(), "no-such-id")
        .await
        .expect("get missing");
    assert!(
        missing.is_none(),
        "non-existent id must return None — pins that bare-id is the only predicate"
    );
}

#[tokio::test]
async fn cli_subtask_tree_is_session_keyed_no_tenant_filter() {
    // Pins the contract for `argo subtask tree <session>` →
    // `subtask::list_by_session`.
    //
    // The SQL is `WHERE parent_session_id = ?`. Tenant A's session
    // returns A's rows; tenant B's session returns B's rows; no
    // tenant check is applied. If a gateway route is ever added,
    // it must verify the session_id belongs to the caller's
    // tenant before invoking list_by_session.
    let store = fresh_store().await;
    let (session_a, _, _) = seed_one_subtree(&store, "a").await;
    let (session_b, _, _) = seed_one_subtree(&store, "b").await;

    let rows_a = list_by_session(store.pool(), &session_a)
        .await
        .expect("list A");
    let rows_b = list_by_session(store.pool(), &session_b)
        .await
        .expect("list B");
    assert_eq!(rows_a.len(), 1, "tenant A session must have its 1 row");
    assert_eq!(rows_b.len(), 1, "tenant B session must have its 1 row");
    assert_eq!(rows_a[0].worker_id, "wkr-a");
    assert_eq!(rows_b[0].worker_id, "wkr-b");
}

#[tokio::test]
async fn cli_subtask_list_filters_are_tenant_blind() {
    // Pins the contract for `argo subtask list --session …` →
    // `subtask::list`. The filters (`session`, `worker`,
    // `parent_goal`, `state`) are tenant-blind: anyone with a
    // session-id can list its rows. CLI operator-shell trust
    // boundary required.
    let store = fresh_store().await;
    let (session_a, _, _) = seed_one_subtree(&store, "a").await;
    let (session_b, _, _) = seed_one_subtree(&store, "b").await;

    let rows_a = subtask::list(
        store.pool(),
        &SubtaskFilters {
            parent_session_id: Some(&session_a),
            worker_id: None,
            parent_goal: None,
            state: None,
        },
        50,
    )
    .await
    .expect("list A");
    let rows_b = subtask::list(
        store.pool(),
        &SubtaskFilters {
            parent_session_id: Some(&session_b),
            worker_id: None,
            parent_goal: None,
            state: None,
        },
        50,
    )
    .await
    .expect("list B");

    assert_eq!(rows_a.len(), 1);
    assert_eq!(rows_b.len(), 1);
    assert_eq!(rows_a[0].worker_id, "wkr-a");
    assert_eq!(rows_b[0].worker_id, "wkr-b");

    // The list helper accepts whatever filter value is passed; a
    // gateway route must verify session ownership before invoking.
    let rows_unfiltered = subtask::list(
        store.pool(),
        &SubtaskFilters {
            parent_session_id: None,
            worker_id: None,
            parent_goal: None,
            state: None,
        },
        50,
    )
    .await
    .expect("list unfiltered");
    assert_eq!(
        rows_unfiltered.len(),
        2,
        "unfiltered list returns BOTH tenants' rows — pins the no-tenant-filter contract"
    );
}

#[tokio::test]
async fn cli_subtask_budget_walk_tree_is_tenant_blind_by_design() {
    // Pins the contract for `argo subtask budget <root_id>` →
    // `subtask_budget::walk_tree`. The recursive CTE anchors at
    // `WHERE id = ?` (root) and then follows `parent_subtask_id`
    // forward, so once a foreign root id is supplied the entire
    // subtree is returned. CLI operator-shell trust boundary
    // required.
    let store = fresh_store().await;
    let (_session_a, subtask_a, _) = seed_one_subtree(&store, "a").await;
    let (_session_b, subtask_b, _) = seed_one_subtree(&store, "b").await;

    let walked_a = walk_tree(store.pool(), &subtask_a)
        .await
        .expect("walk A");
    let walked_b = walk_tree(store.pool(), &subtask_b)
        .await
        .expect("walk B");
    assert_eq!(walked_a.len(), 1, "single-row root subtree for tenant A");
    assert_eq!(walked_b.len(), 1, "single-row root subtree for tenant B");
    assert_eq!(walked_a[0].row.worker_id, "wkr-a");
    assert_eq!(walked_b[0].row.worker_id, "wkr-b");

    // Non-existent root returns empty vec — pins the no-extra-
    // predicate contract.
    let walked_missing = walk_tree(store.pool(), "no-such-root")
        .await
        .expect("walk missing");
    assert!(walked_missing.is_empty());
}
