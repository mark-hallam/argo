//! Namespace-bound action tests for the `Memory` tool —
//! `query_scored` (indexed-layer search) and the shared-team
//! actions (`write_team` / `list_team` / `read_team`). Legacy
//! action tests live in `memory_legacy_tests.rs`.

use super::test_helpers::*;
use super::*;
use argo_types::TenantId;
use uuid::Uuid;

// ───────────────── query_scored ─────────────────

#[tokio::test]
async fn query_scored_default_namespace_returns_hits_ranked_by_score() {
    let (store, ws) = fresh_store().await;
    let ns = ns_for();
    seed_indexed(
        &store,
        &ns,
        ws,
        &[
            "alpha bravo charlie",
            "delta echo foxtrot",
            "alpha alpha alpha",
        ],
    )
    .await;
    let tool = Memory::with_namespace(store, ws, None, ns.clone());
    let r = tool
        .dispatch(
            serde_json::json!({"action": "query_scored", "query": "alpha"}),
            &ctx(),
        )
        .await
        .unwrap();
    assert!(!r.is_error, "got: {:?}", r.content);
    let body = first_text(&r);
    let hits: Vec<serde_json::Value> = serde_json::from_str(&body).expect("json");
    assert_eq!(hits.len(), 3, "all 3 entries match (token overlap)");
    // First hit should be the entry with the most `alpha` tokens.
    assert_eq!(hits[0]["snippet"].as_str().unwrap(), "alpha alpha alpha");
    let score0 = hits[0]["score"].as_f64().unwrap();
    let score1 = hits[1]["score"].as_f64().unwrap();
    assert!(
        score0 >= score1,
        "scores must be descending: {score0} {score1}"
    );
}

#[tokio::test]
async fn query_scored_explicit_namespace_defers_to_followup_brief() {
    let (store, ws) = fresh_store().await;
    let tool = Memory::with_namespace(store, ws, None, ns_for());
    let err = tool
        .dispatch(
            serde_json::json!({
                "action": "query_scored",
                "query": "alpha",
                "namespace": "worker:other-id",
            }),
            &ctx(),
        )
        .await
        .expect_err("explicit namespace not yet supported in v1");
    let msg = err.to_string();
    assert!(
        msg.contains("mission-memory-namespace-addressing"),
        "got: {msg}"
    );
    assert!(msg.contains("#43"), "got: {msg}");
}

#[tokio::test]
async fn query_scored_respects_limit() {
    let (store, ws) = fresh_store().await;
    let ns = ns_for();
    seed_indexed(
        &store,
        &ns,
        ws,
        &[
            "alpha one",
            "alpha two",
            "alpha three",
            "alpha four",
            "alpha five",
        ],
    )
    .await;
    let tool = Memory::with_namespace(store, ws, None, ns);
    let r = tool
        .dispatch(
            serde_json::json!({"action": "query_scored", "query": "alpha", "limit": 2}),
            &ctx(),
        )
        .await
        .unwrap();
    let hits: Vec<serde_json::Value> = serde_json::from_str(&first_text(&r)).unwrap();
    assert_eq!(hits.len(), 2);
}

#[tokio::test]
async fn query_scored_clamps_limit_to_20() {
    let (store, ws) = fresh_store().await;
    let ns = ns_for();
    let bodies: Vec<String> = (0..25).map(|i| format!("alpha row {i}")).collect();
    let refs: Vec<&str> = bodies.iter().map(String::as_str).collect();
    seed_indexed(&store, &ns, ws, &refs).await;
    let tool = Memory::with_namespace(store, ws, None, ns);
    let r = tool
        .dispatch(
            serde_json::json!({"action": "query_scored", "query": "alpha", "limit": 100}),
            &ctx(),
        )
        .await
        .unwrap();
    let hits: Vec<serde_json::Value> = serde_json::from_str(&first_text(&r)).unwrap();
    assert_eq!(hits.len(), 20, "limit clamps to 20, never errors");
}

#[tokio::test]
async fn query_scored_zero_limit_returns_empty_vec() {
    let (store, ws) = fresh_store().await;
    let ns = ns_for();
    seed_indexed(&store, &ns, ws, &["alpha", "beta"]).await;
    let tool = Memory::with_namespace(store, ws, None, ns);
    let r = tool
        .dispatch(
            serde_json::json!({"action": "query_scored", "query": "alpha", "limit": 0}),
            &ctx(),
        )
        .await
        .unwrap();
    assert!(!r.is_error, "k=0 is not an error");
    let hits: Vec<serde_json::Value> = serde_json::from_str(&first_text(&r)).unwrap();
    assert!(hits.is_empty(), "k=0 returns empty Vec");
}

#[tokio::test]
async fn query_scored_unwired_errors_with_historical_message() {
    let r = Memory::default()
        .dispatch(
            serde_json::json!({"action": "query_scored", "query": "alpha"}),
            &ctx(),
        )
        .await
        .unwrap();
    assert!(r.is_error);
    let body = first_text(&r);
    assert!(body.contains("memory store not wired"), "got: {body}");
}

#[tokio::test]
async fn query_scored_wired_without_namespace_explains_binding() {
    let (store, ws) = fresh_store().await;
    let tool = Memory::new(store, ws, None);
    let err = tool
        .dispatch(
            serde_json::json!({"action": "query_scored", "query": "alpha"}),
            &ctx(),
        )
        .await
        .expect_err("must reject when no namespace bound");
    let msg = err.to_string();
    assert!(
        msg.contains("not bound to a worker namespace"),
        "got: {msg}"
    );
    assert!(msg.contains("Memory::with_namespace"), "got: {msg}");
}

// ───────────────── shared-team actions ─────────────────

#[test]
fn write_team_action_requires_memory_write_team_cap() {
    let caps = Memory::default()
        .required_capabilities(&serde_json::json!({"action": "write_team", "content": "c"}))
        .unwrap();
    assert!(caps.covers(&Capability::MemoryWriteTeam));
    assert!(
        !caps.covers(&Capability::MemoryWrite),
        "write_team must NOT require MemoryWrite (private-namespace cap)",
    );
}

#[test]
fn list_team_and_read_team_actions_require_no_capability() {
    for action in ["list_team", "read_team"] {
        let caps = Memory::default()
            .required_capabilities(&serde_json::json!({
                "action": action, "entry": Uuid::nil().to_string(),
            }))
            .unwrap();
        assert!(
            !caps.covers(&Capability::MemoryWriteTeam),
            "{action} must NOT require write-team cap (reads are default-allowed)",
        );
        assert!(
            !caps.covers(&Capability::MemoryWrite),
            "{action} must NOT require private-namespace write cap",
        );
    }
}

#[tokio::test]
async fn list_team_returns_team_entries_in_chronological_order() {
    let (store, ws) = fresh_store().await;
    let ns = ns_for();
    let tool = Memory::with_namespace(store, ws, None, ns);
    let mut written_ids = Vec::new();
    for body in ["alpha-row", "bravo-row", "charlie-row"] {
        let w = tool
            .dispatch(
                serde_json::json!({"action": "write_team", "content": body}),
                &ctx(),
            )
            .await
            .unwrap();
        let body = first_text(&w);
        let id = body
            .split("id=")
            .nth(1)
            .and_then(|s| s.split(',').next())
            .map(str::to_string)
            .unwrap();
        written_ids.push(id);
    }
    let r = tool
        .dispatch(serde_json::json!({"action": "list_team"}), &ctx())
        .await
        .unwrap();
    assert!(!r.is_error, "got: {:?}", r.content);
    let body = first_text(&r);
    let pos0 = body.find(&written_ids[0]).unwrap();
    let pos1 = body.find(&written_ids[1]).unwrap();
    let pos2 = body.find(&written_ids[2]).unwrap();
    assert!(
        pos0 < pos1 && pos1 < pos2,
        "insertion order preserved:\n{body}"
    );
}

#[tokio::test]
async fn read_team_reads_by_entry_id_uuid() {
    let (store, ws) = fresh_store().await;
    let ns = ns_for();
    let tool = Memory::with_namespace(store, ws, None, ns);
    let w = tool
        .dispatch(
            serde_json::json!({"action": "write_team", "content": "shared note"}),
            &ctx(),
        )
        .await
        .unwrap();
    let written = first_text(&w);
    let id = written
        .split("id=")
        .nth(1)
        .and_then(|s| s.split(',').next())
        .unwrap();
    let r = tool
        .dispatch(
            serde_json::json!({"action": "read_team", "entry": id}),
            &ctx(),
        )
        .await
        .unwrap();
    assert_eq!(first_text(&r), "shared note");
}

#[tokio::test]
async fn list_team_defaults_to_worker_bound_tenant() {
    let (store, ws) = fresh_store().await;
    let ns = ns_for();
    let tool = Memory::with_namespace(store.clone(), ws, None, ns.clone());
    tool.dispatch(
        serde_json::json!({"action": "write_team", "content": "x"}),
        &ctx(),
    )
    .await
    .unwrap();
    // No team_id → must resolve to the worker's bound tenant and see the row.
    let r = tool
        .dispatch(serde_json::json!({"action": "list_team"}), &ctx())
        .await
        .unwrap();
    assert!(
        first_text(&r).contains("bytes=1"),
        "got: {}",
        first_text(&r)
    );
    // Explicit matching team_id → same result.
    let r = tool
        .dispatch(
            serde_json::json!({"action": "list_team", "team_id": ns.tenant_id.to_string()}),
            &ctx(),
        )
        .await
        .unwrap();
    assert!(first_text(&r).contains("bytes=1"));
}

#[tokio::test]
async fn read_team_cross_tenant_team_id_is_rejected() {
    let (store, ws) = fresh_store().await;
    let ns = ns_for();
    let tool = Memory::with_namespace(store, ws, None, ns);
    let other_tenant = TenantId::new();
    let err = tool
        .dispatch(
            serde_json::json!({
                "action": "read_team",
                "entry": Uuid::nil().to_string(),
                "team_id": other_tenant.to_string(),
            }),
            &ctx(),
        )
        .await
        .expect_err("cross-tenant team_id must be rejected");
    let msg = err.to_string();
    assert!(msg.contains("cross-team"), "got: {msg}");
}

#[tokio::test]
async fn team_actions_without_namespace_binding_explain_requirement() {
    let (store, ws) = fresh_store().await;
    let tool = Memory::new(store, ws, None);
    let err = tool
        .dispatch(serde_json::json!({"action": "list_team"}), &ctx())
        .await
        .expect_err("must require a bound namespace");
    let msg = err.to_string();
    assert!(
        msg.contains("not bound to a worker namespace"),
        "got: {msg}"
    );
    assert!(msg.contains("team actions"), "got: {msg}");
}
