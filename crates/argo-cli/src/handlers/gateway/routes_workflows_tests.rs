//! Unit coverage for the workflow routes shipped in this gateway
//! module — currently the rollback POST path (`parse_route` smoke
//! tests) and the optimistic-concurrency `if_version` gate on the
//! create-version handler shipped in
//! `web-workflow-concurrent-edit-conflict-banner`. The full SPA flow
//! is exercised by the Playwright specs (`workflow-rollback.spec.ts`,
//! `workflow-concurrent-edit.spec.ts`) when a real gateway is running,
//! and the storage-layer behaviour by `workflow_definition_*_tests.rs`.

use super::super::routes_workflows_handlers as hh;
use super::{parse_route, Route};
use argo_store::workflow_definition::{self as wd, WorkflowLifecycleState};
use argo_store::SessionStore;
use argo_types::TenantId;
use serde_json::{json, Value};
use std::sync::Arc;

#[test]
fn parse_route_recognises_rollback_post() {
    let req = "POST /api/v1/verticals/pr_checker/workflows/standard_review/rollback HTTP/1.1\r\nHost: x\r\n\r\n";
    let route = parse_route(req, "POST ").expect("rollback route must parse");
    match route {
        Route::Rollback { vertical, name } => {
            assert_eq!(vertical, "pr_checker");
            assert_eq!(name, "standard_review");
        }
        other => panic!("expected Route::Rollback; got {other:?}"),
    }
}

#[test]
fn parse_route_rejects_rollback_on_non_post_verbs() {
    for verb in ["GET ", "PUT ", "DELETE "] {
        let req = format!(
            "{verb}/api/v1/verticals/v/workflows/n/rollback HTTP/1.1\r\nHost: x\r\n\r\n"
        );
        assert!(
            parse_route(&req, verb).is_none(),
            "{verb} on /rollback must not match the Rollback route"
        );
    }
}

#[test]
fn parse_route_still_recognises_existing_routes() {
    // Regression guard: the new Rollback arm sits beside the existing
    // SetHead / Archive arms — we re-confirm those still match.
    let put = "PUT /api/v1/verticals/v/workflows/n/head HTTP/1.1\r\nHost: x\r\n\r\n";
    assert!(matches!(parse_route(put, "PUT ").expect("set head"), Route::SetHead { .. }));
    let del = "DELETE /api/v1/verticals/v/workflows/n/versions/id-1 HTTP/1.1\r\nHost: x\r\n\r\n";
    assert!(matches!(parse_route(del, "DELETE ").expect("archive"), Route::Archive { .. }));
}

// ── Optimistic-concurrency tests ───────────────────────────────
//
// Drives `create_version` directly (bypassing the auth + tenant-
// resolution layers) so the optimistic-lock contract can be asserted
// without standing up a full HTTP harness. The dispatcher wiring is
// covered by the parse_route tests above + the live Playwright spec.

const VERTICAL: &str = "pr_checker";
const WORKFLOW: &str = "open_pr_review";

const TOML_V1: &str = r#"
schema_version = "1"

[workflow]
name = "open_pr_review"

[[step]]
name = "review"
kind = "skill_invoke"
skill = "reviewer"
"#;

const TOML_V2: &str = r#"
schema_version = "1"

[workflow]
name = "open_pr_review"

[[step]]
name = "review"
kind = "skill_invoke"
skill = "reviewer-v2"
"#;

async fn fixture_store_with_head(version_count: usize) -> (Arc<SessionStore>, TenantId) {
    let store = SessionStore::open(":memory:").await.expect("open mem db");
    store.migrate().await.expect("migrate");
    let store = Arc::new(store);
    let tenant = TenantId::new();
    let bodies: [&str; 2] = [TOML_V1, TOML_V2];
    let mut last_id = None;
    for &body in &bodies[..version_count] {
        let id = wd::create(
            store.pool(),
            tenant,
            VERTICAL,
            WORKFLOW,
            body,
            None,
            "seed",
        )
        .await
        .expect("seed version");
        last_id = Some(id);
    }
    if let Some(id) = last_id {
        wd::set_head(store.pool(), tenant, VERTICAL, WORKFLOW, &id)
            .await
            .expect("set head");
    }
    (store, tenant)
}

#[tokio::test]
async fn save_draft_with_no_if_version_succeeds_and_bumps_version() {
    let (store, tenant) = fixture_store_with_head(1).await;
    let resp = hh::create_version(
        &store,
        tenant,
        VERTICAL,
        WORKFLOW,
        TOML_V2,
        "actor-1",
        None,
    )
    .await;
    assert!(resp.0.starts_with("201"), "status: {} body: {}", resp.0, resp.1);
    let parsed: Value = serde_json::from_str(&resp.1).expect("parse body");
    assert!(parsed["version_id"].as_str().is_some(), "version_id present");
    // Head must remain pinned to v1 (set_head was not called) — the
    // contract is "draft created, head unchanged". List versions to
    // confirm there are now 2 rows under the (tenant, vertical, name).
    let versions = wd::list_versions(store.pool(), tenant, VERTICAL, WORKFLOW)
        .await
        .expect("list versions");
    assert_eq!(versions.len(), 2, "draft should have created a second row");
}

#[tokio::test]
async fn save_draft_with_matching_if_version_succeeds() {
    let (store, tenant) = fixture_store_with_head(1).await;
    // Head is v1; asserting if_version=1 is current → create proceeds.
    let resp = hh::create_version(
        &store,
        tenant,
        VERTICAL,
        WORKFLOW,
        TOML_V2,
        "actor-1",
        Some(1),
    )
    .await;
    assert!(resp.0.starts_with("201"), "status: {} body: {}", resp.0, resp.1);
    let parsed: Value = serde_json::from_str(&resp.1).expect("parse body");
    assert!(parsed["version_id"].as_str().is_some(), "version_id present");
}

// ── Transactional audit-write tests ─────────────────────────────
//
// `mission-workflow-audit-write-transactional` — every mutating
// workflow handler must wrap its workflow-row write and its
// `audit_record` insert in ONE sqlx transaction so they commit
// together or roll back together. Failure-injection (drop the
// `audit_record` table mid-flight) is the load-bearing assertion:
// without it, the contract isn't pinned.

async fn audit_row_count(store: &SessionStore, kind: &str) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM audit_record WHERE kind = ?")
        .bind(kind)
        .fetch_one(store.pool())
        .await
        .expect("count audit_record rows")
}

async fn drop_audit_record_table(store: &SessionStore) {
    // Force every subsequent INSERT INTO audit_record to fail with a
    // "no such table" sqlx error. The handler's transactional path
    // must rollback the workflow-row mutation when this fires.
    sqlx::query("DROP TABLE audit_record")
        .execute(store.pool())
        .await
        .expect("drop audit_record");
}

async fn fixture_two_versions_head_v1() -> (
    Arc<SessionStore>,
    TenantId,
    wd::WorkflowDefinitionId,
    wd::WorkflowDefinitionId,
) {
    let store = SessionStore::open(":memory:").await.expect("open mem db");
    store.migrate().await.expect("migrate");
    let store = Arc::new(store);
    let tenant = TenantId::new();
    let v1 = wd::create(store.pool(), tenant, VERTICAL, WORKFLOW, TOML_V1, None, "seed")
        .await
        .expect("seed v1");
    let v2 = wd::create(
        store.pool(),
        tenant,
        VERTICAL,
        WORKFLOW,
        TOML_V2,
        Some(v1.clone()),
        "seed",
    )
    .await
    .expect("seed v2");
    wd::set_head(store.pool(), tenant, VERTICAL, WORKFLOW, &v1)
        .await
        .expect("seed head=v1");
    (store, tenant, v1, v2)
}

#[tokio::test]
async fn set_head_commits_workflow_row_and_audit_row_atomically() {
    let (store, tenant, v1, v2) = fixture_two_versions_head_v1().await;
    let before = audit_row_count(&store, "workflow_head_changed").await;
    let body = json!({ "version_id": v2.0 }).to_string();
    let resp = hh::set_head(&store, tenant, VERTICAL, WORKFLOW, &body, "actor-1").await;
    assert!(resp.0.starts_with("204"), "status: {} body: {}", resp.0, resp.1);
    let head = wd::get_head(store.pool(), tenant, VERTICAL, WORKFLOW)
        .await
        .expect("get head")
        .expect("head row");
    assert_eq!(head.id, v2, "head must advance to v2");
    let after = audit_row_count(&store, "workflow_head_changed").await;
    assert_eq!(after, before + 1, "exactly one audit row added");
    drop(v1);
}

#[tokio::test]
async fn set_head_rolls_back_workflow_when_audit_insert_fails() {
    let (store, tenant, v1, v2) = fixture_two_versions_head_v1().await;
    drop_audit_record_table(&store).await;
    let body = json!({ "version_id": v2.0 }).to_string();
    let resp = hh::set_head(&store, tenant, VERTICAL, WORKFLOW, &body, "actor-1").await;
    assert!(resp.0.starts_with("500"), "status: {} body: {}", resp.0, resp.1);
    let head = wd::get_head(store.pool(), tenant, VERTICAL, WORKFLOW)
        .await
        .expect("get head")
        .expect("head row");
    assert_eq!(
        head.id, v1,
        "head must remain v1 — audit-insert failure rolls back the workflow tx"
    );
    let v2_row = wd::get(store.pool(), &v2)
        .await
        .expect("get v2")
        .expect("v2 row");
    assert_eq!(
        v2_row.lifecycle_state,
        WorkflowLifecycleState::Draft,
        "v2 must remain Draft — set_head's UPDATE rolls back too"
    );
}

#[tokio::test]
async fn archive_commits_workflow_row_and_audit_row_atomically() {
    let (store, tenant, v1, _v2) = fixture_two_versions_head_v1().await;
    let before = audit_row_count(&store, "workflow_version_archived").await;
    let resp = hh::archive_version(&store, tenant, VERTICAL, WORKFLOW, &v1.0, "actor-1").await;
    assert!(resp.0.starts_with("204"), "status: {} body: {}", resp.0, resp.1);
    let v1_row = wd::get(store.pool(), &v1)
        .await
        .expect("get v1")
        .expect("v1 row");
    assert_eq!(v1_row.lifecycle_state, WorkflowLifecycleState::Archived);
    assert!(
        wd::get_head(store.pool(), tenant, VERTICAL, WORKFLOW)
            .await
            .expect("get head")
            .is_none(),
        "head pointer cleared when archived row was head"
    );
    let after = audit_row_count(&store, "workflow_version_archived").await;
    assert_eq!(after, before + 1, "exactly one audit row added");
}

#[tokio::test]
async fn archive_rolls_back_workflow_when_audit_insert_fails() {
    let (store, tenant, v1, _v2) = fixture_two_versions_head_v1().await;
    drop_audit_record_table(&store).await;
    let resp = hh::archive_version(&store, tenant, VERTICAL, WORKFLOW, &v1.0, "actor-1").await;
    assert!(resp.0.starts_with("500"), "status: {} body: {}", resp.0, resp.1);
    let v1_row = wd::get(store.pool(), &v1)
        .await
        .expect("get v1")
        .expect("v1 row");
    assert_eq!(
        v1_row.lifecycle_state,
        WorkflowLifecycleState::Active,
        "v1 must remain Active — audit-insert failure rolls back the archive tx"
    );
    let head = wd::get_head(store.pool(), tenant, VERTICAL, WORKFLOW)
        .await
        .expect("get head")
        .expect("head row");
    assert_eq!(head.id, v1, "head pointer must still point at v1");
}

#[tokio::test]
async fn rollback_commits_workflow_row_and_audit_row_atomically() {
    // Head=v1, v2 exists as draft. To rollback we point at v2 (not the
    // current head v1) → store creates v3 with v2's TOML and flips head
    // to v3.
    let (store, tenant, v1, v2) = fixture_two_versions_head_v1().await;
    // Flip head to v2 so rollback to v1 is a valid downgrade.
    wd::set_head(store.pool(), tenant, VERTICAL, WORKFLOW, &v2)
        .await
        .expect("flip head to v2");
    let before = audit_row_count(&store, "workflow_rolled_back").await;
    let body = json!({ "target_version_id": v1.0 }).to_string();
    let resp = hh::rollback(&store, tenant, VERTICAL, WORKFLOW, &body, "actor-1").await;
    assert!(resp.0.starts_with("200"), "status: {} body: {}", resp.0, resp.1);
    let parsed: Value = serde_json::from_str(&resp.1).expect("parse body");
    let new_version = parsed["version"].as_i64().expect("version present");
    assert_eq!(new_version, 3, "rollback creates a brand new version row");
    let head = wd::get_head(store.pool(), tenant, VERTICAL, WORKFLOW)
        .await
        .expect("get head")
        .expect("head row");
    assert_eq!(head.version, 3, "head must advance to the new v3 row");
    let after = audit_row_count(&store, "workflow_rolled_back").await;
    assert_eq!(after, before + 1, "exactly one audit row added");
}

#[tokio::test]
async fn rollback_rolls_back_workflow_when_audit_insert_fails() {
    let (store, tenant, v1, v2) = fixture_two_versions_head_v1().await;
    wd::set_head(store.pool(), tenant, VERTICAL, WORKFLOW, &v2)
        .await
        .expect("flip head to v2");
    drop_audit_record_table(&store).await;
    let body = json!({ "target_version_id": v1.0 }).to_string();
    let resp = hh::rollback(&store, tenant, VERTICAL, WORKFLOW, &body, "actor-1").await;
    assert!(resp.0.starts_with("500"), "status: {} body: {}", resp.0, resp.1);
    let versions = wd::list_versions(store.pool(), tenant, VERTICAL, WORKFLOW)
        .await
        .expect("list versions");
    assert_eq!(
        versions.len(),
        2,
        "v3 must NOT exist — audit failure rolls back the rollback INSERT"
    );
    let head = wd::get_head(store.pool(), tenant, VERTICAL, WORKFLOW)
        .await
        .expect("get head")
        .expect("head row");
    assert_eq!(head.id, v2, "head must remain v2");
}

#[tokio::test]
async fn save_draft_with_stale_if_version_returns_409_with_current_head() {
    // Seed twice + set_head to v2 → current head version is 2.
    let (store, tenant) = fixture_store_with_head(2).await;
    let resp = hh::create_version(
        &store,
        tenant,
        VERTICAL,
        WORKFLOW,
        TOML_V2,
        "actor-1",
        Some(1), // caller's view is stale (still on v1).
    )
    .await;
    assert!(resp.0.starts_with("409"), "status: {} body: {}", resp.0, resp.1);
    let parsed: Value = serde_json::from_str(&resp.1).expect("parse body");
    assert_eq!(parsed["kind"], "conflict");
    assert_eq!(parsed["current_version"].as_i64(), Some(2));
    let toml = parsed["current_head_definition_toml"]
        .as_str()
        .expect("current_head_definition_toml present");
    assert!(toml.contains("reviewer-v2"), "current head TOML echoed back");
    // No new row should have been inserted on the conflict path.
    let versions = wd::list_versions(store.pool(), tenant, VERTICAL, WORKFLOW)
        .await
        .expect("list versions");
    assert_eq!(versions.len(), 2, "conflict must not insert a new draft");
}

#[tokio::test]
async fn concurrent_create_handler_returns_201_and_409_not_500() {
    // Brief `claude-workflow-create-unique-collision-as-409` — evaluator
    // round-7 caught the gateway returning `500 Internal Server Error`
    // for the loser of a concurrent-create race instead of the canonical
    // `409 Conflict`. Spawn N concurrent handler invocations behind a
    // barrier so the SELECT MAX + INSERT race opens for every racer;
    // assert no `5xx` slips through and that the 409 body names the
    // colliding triple per the brief's contract.
    use tokio::sync::Barrier;

    let store = SessionStore::open(":memory:").await.expect("open mem db");
    store.migrate().await.expect("migrate");
    let store = Arc::new(store);
    let tenant = TenantId::new();

    const N: usize = 8;
    let barrier = Arc::new(Barrier::new(N));
    let mut handles = Vec::with_capacity(N);
    for i in 0..N {
        let store = Arc::clone(&store);
        let barrier = Arc::clone(&barrier);
        let actor = format!("actor-{i}");
        handles.push(tokio::spawn(async move {
            barrier.wait().await;
            hh::create_version(&store, tenant, VERTICAL, WORKFLOW, TOML_V1, &actor, None).await
        }));
    }
    let mut resps = Vec::with_capacity(N);
    for h in handles {
        resps.push(h.await.expect("join"));
    }

    let mut got_201 = false;
    let mut got_409 = false;
    for (status, body) in &resps {
        assert!(
            !status.starts_with("5"),
            "no 5xx on concurrent create: {status} {body}"
        );
        if status.starts_with("201") {
            got_201 = true;
        } else if status.starts_with("409") {
            got_409 = true;
            let parsed: Value = serde_json::from_str(body).expect("parse 409 body");
            assert_eq!(
                parsed["error"], "workflow already exists",
                "409 body shape: {body}"
            );
            assert_eq!(parsed["vertical"], VERTICAL, "vertical in body: {body}");
            assert_eq!(parsed["name"], WORKFLOW, "name in body: {body}");
        } else {
            panic!("unexpected status from concurrent create: {status} {body}");
        }
    }
    assert!(got_201, "at least one 201 expected: {resps:?}");
    assert!(
        got_409,
        "at least one 409 expected (collision must surface as Conflict, not 500): {resps:?}"
    );
}
