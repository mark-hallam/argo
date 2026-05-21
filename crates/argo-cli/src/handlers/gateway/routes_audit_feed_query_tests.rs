//! Query-shape tests for `routes_audit_feed` — filters, pagination,
//! limit/range/cursor validation, redaction, and the audit-emit row
//! the feed writes per query. Tenant-scoping and SQL-path tests live
//! in `routes_audit_feed_tenant_tests.rs`.

#![allow(clippy::too_many_lines)]
#![allow(deprecated)]

use super::list_response;
use super::test_helpers::*;

#[tokio::test]
async fn returns_200_with_items_and_no_filter() {
    let store = mem_store().await;
    seed_minimal_two(&store).await;
    let (status, body) = parse_body(list_response(&get("/api/v1/audit"), Some(&store)).await).await;
    assert_eq!(status, "200 OK");
    let items = body["items"].as_array().expect("items array");
    // Two seeded rows + the audit.feed.queried row this call emits.
    assert!(items.len() >= 2);
}

#[tokio::test]
async fn returns_503_when_store_missing() {
    let (status, body) = list_response(&get("/api/v1/audit"), None).await;
    assert_eq!(status, "503 Service Unavailable");
    assert!(body.contains("session store unavailable"));
}

#[tokio::test]
async fn filters_by_worker_id() {
    let store = mem_store().await;
    seed_minimal_two(&store).await;
    let (status, body) =
        parse_body(list_response(&get("/api/v1/audit?worker=w1"), Some(&store)).await).await;
    assert_eq!(status, "200 OK");
    let items = body["items"].as_array().unwrap();
    assert!(!items.is_empty());
    for it in items {
        assert_eq!(it["worker"]["id"], "w1", "row leaked: {it}");
    }
}

#[tokio::test]
async fn filters_by_channel() {
    let store = mem_store().await;
    seed_minimal_two(&store).await;
    let (_, body) =
        parse_body(list_response(&get("/api/v1/audit?channel=slack"), Some(&store)).await).await;
    let items = body["items"].as_array().unwrap();
    assert!(!items.is_empty());
    for it in items {
        assert_eq!(it["channel"], "slack");
    }
}

#[tokio::test]
async fn filters_by_action_type() {
    let store = mem_store().await;
    seed_minimal_two(&store).await;
    let (_, body) = parse_body(
        list_response(&get("/api/v1/audit?action_type=message.sent"), Some(&store)).await,
    )
    .await;
    let items = body["items"].as_array().unwrap();
    assert!(!items.is_empty());
    for it in items {
        assert_eq!(it["action_type"], "message.sent");
    }
}

#[tokio::test]
async fn filters_by_status() {
    let store = mem_store().await;
    seed_minimal_two(&store).await;
    let (_, body) =
        parse_body(list_response(&get("/api/v1/audit?status=failed"), Some(&store)).await).await;
    let items = body["items"].as_array().unwrap();
    assert!(!items.is_empty());
    for it in items {
        assert_eq!(it["status"], "failed");
    }
}

#[tokio::test]
async fn filters_by_approver() {
    let store = mem_store().await;
    seed_minimal_two(&store).await;
    let (_, body) =
        parse_body(list_response(&get("/api/v1/audit?approver=a1"), Some(&store)).await).await;
    let items = body["items"].as_array().unwrap();
    assert!(!items.is_empty());
    for it in items {
        assert_eq!(it["approver"]["id"], "a1");
    }
}

#[tokio::test]
async fn filters_by_cost_band() {
    let store = mem_store().await;
    seed_minimal_two(&store).await;
    // w1 had cost 0.5 → from001to10
    let (_, body) =
        parse_body(list_response(&get("/api/v1/audit?cost_band=from001to10"), Some(&store)).await)
            .await;
    let items = body["items"].as_array().unwrap();
    assert!(items.iter().any(|it| it["worker"]["id"] == "w1"));
    assert!(items.iter().all(|it| it["worker"]["id"] != "w2"));
}

#[tokio::test]
async fn filters_by_classification() {
    let store = mem_store().await;
    seed_minimal_two(&store).await;
    let (_, body) =
        parse_body(list_response(&get("/api/v1/audit?classification=public"), Some(&store)).await)
            .await;
    let items = body["items"].as_array().unwrap();
    assert!(!items.is_empty());
    for it in items {
        assert_eq!(it["data_classification"], "public");
    }
}

#[tokio::test]
async fn multi_value_dimension_is_or() {
    let store = mem_store().await;
    seed_minimal_two(&store).await;
    let (_, body) = parse_body(
        list_response(
            &get("/api/v1/audit?status=succeeded,failed&worker=w1,w2"),
            Some(&store),
        )
        .await,
    )
    .await;
    let items = body["items"].as_array().unwrap();
    let ids: Vec<&str> = items
        .iter()
        .filter_map(|it| it["worker"]["id"].as_str())
        .collect();
    assert!(ids.contains(&"w1"));
    assert!(ids.contains(&"w2"));
}

#[tokio::test]
async fn substring_query_q_matches_verb_case_insensitive() {
    let store = mem_store().await;
    seed_minimal_two(&store).await;
    let (_, body) =
        parse_body(list_response(&get("/api/v1/audit?q=APPROVED"), Some(&store)).await).await;
    let items = body["items"].as_array().unwrap();
    assert!(!items.is_empty(), "case-insensitive q should match");
    for it in items {
        assert!(it["verb"]
            .as_str()
            .unwrap()
            .to_lowercase()
            .contains("approved"));
    }
}

#[tokio::test]
async fn empty_result_when_no_match() {
    let store = mem_store().await;
    seed_minimal_two(&store).await;
    let (status, body) =
        parse_body(list_response(&get("/api/v1/audit?worker=nope"), Some(&store)).await).await;
    assert_eq!(status, "200 OK");
    assert!(body["items"].as_array().unwrap().is_empty());
    assert!(body.get("next_cursor").is_none());
}

#[tokio::test]
async fn pagination_emits_cursor_and_continues() {
    let store = mem_store().await;
    for i in 0..6 {
        seed_row(
            &store,
            row(
                &format!("w{i}"),
                "web",
                "msg",
                "succeeded",
                None,
                None,
                "internal",
                "did x",
            ),
        )
        .await;
    }
    let (_, body) =
        parse_body(list_response(&get("/api/v1/audit?limit=2"), Some(&store)).await).await;
    let first_items = body["items"].as_array().unwrap().clone();
    assert_eq!(first_items.len(), 2);
    let cursor = body["next_cursor"].as_str().expect("cursor present");
    let path = format!("/api/v1/audit?limit=2&cursor={cursor}");
    let (_, body2) = parse_body(list_response(&get(&path), Some(&store)).await).await;
    let second_items = body2["items"].as_array().unwrap().clone();
    assert_eq!(second_items.len(), 2);
    let first_ids: Vec<&str> = first_items
        .iter()
        .map(|it| it["id"].as_str().unwrap())
        .collect();
    for it in &second_items {
        assert!(!first_ids.contains(&it["id"].as_str().unwrap()));
    }
}

#[tokio::test]
async fn rejects_limit_out_of_range_400() {
    let store = mem_store().await;
    let (status, body) = list_response(&get("/api/v1/audit?limit=0"), Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
    assert!(body.contains("limit"));
    let (status, body) = list_response(&get("/api/v1/audit?limit=999"), Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
    assert!(body.contains("limit"));
}

#[tokio::test]
async fn rejects_invalid_limit_400() {
    let store = mem_store().await;
    let (status, body) = list_response(&get("/api/v1/audit?limit=abc"), Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
    assert!(body.contains("limit"));
}

#[tokio::test]
async fn limit_caps_default_to_fifty() {
    let store = mem_store().await;
    for i in 0..60 {
        seed_row(
            &store,
            row(
                &format!("w{i}"),
                "web",
                "msg",
                "succeeded",
                None,
                None,
                "internal",
                "did x",
            ),
        )
        .await;
    }
    let (_, body) = parse_body(list_response(&get("/api/v1/audit"), Some(&store)).await).await;
    let items = body["items"].as_array().unwrap();
    assert_eq!(items.len(), 50);
    assert!(body["next_cursor"].as_str().is_some());
}

#[tokio::test]
async fn range_1h_filters_to_recent_only() {
    let store = mem_store().await;
    seed_minimal_two(&store).await;
    let (status, body) =
        parse_body(list_response(&get("/api/v1/audit?range=1h"), Some(&store)).await).await;
    assert_eq!(status, "200 OK");
    // Both seeded rows + the audit-write itself are within the 1h window.
    assert!(!body["items"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn rejects_invalid_range_400() {
    let store = mem_store().await;
    let (status, body) = list_response(&get("/api/v1/audit?range=zzz"), Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
    assert!(body.contains("range"));
}

#[tokio::test]
async fn rejects_invalid_cursor_400() {
    let store = mem_store().await;
    let (status, body) =
        list_response(&get("/api/v1/audit?cursor=not_base64!!!"), Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
    assert!(body.contains("cursor"));
}

#[tokio::test]
async fn redacts_email_in_verb_by_default() {
    let store = mem_store().await;
    seed_row(
        &store,
        row(
            "w1",
            "web",
            "msg",
            "succeeded",
            None,
            None,
            "internal",
            "ping sam@acmeco.com to confirm",
        ),
    )
    .await;
    let (_, body) =
        parse_body(list_response(&get("/api/v1/audit?worker=w1"), Some(&store)).await).await;
    let items = body["items"].as_array().unwrap();
    let verbs: Vec<&str> = items.iter().filter_map(|it| it["verb"].as_str()).collect();
    assert!(verbs.iter().any(|v| v.contains("[EMAIL]")));
    assert!(verbs.iter().all(|v| !v.contains("sam@acmeco.com")));
}

#[tokio::test]
async fn emits_audit_row_per_query() {
    let store = mem_store().await;
    let before: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_record WHERE payload_json LIKE '%audit.feed.queried%'",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    let (status, _) = list_response(&get("/api/v1/audit"), Some(&store)).await;
    assert_eq!(status, "200 OK");
    let after: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_record WHERE payload_json LIKE '%audit.feed.queried%'",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert_eq!(
        after,
        before + 1,
        "expected exactly one audit.feed.queried row"
    );
}

#[tokio::test]
async fn audit_row_payload_lists_dims_used() {
    let store = mem_store().await;
    seed_minimal_two(&store).await;
    let (status, _) = list_response(
        &get("/api/v1/audit?worker=w1&q=approved&range=7d"),
        Some(&store),
    )
    .await;
    assert_eq!(status, "200 OK");
    let payload: String = sqlx::query_scalar(
        "SELECT payload_json FROM audit_record WHERE payload_json LIKE '%audit.feed.queried%' \
         ORDER BY created_at DESC LIMIT 1",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert!(payload.contains("\"worker\""));
    assert!(payload.contains("\"q\""));
    assert!(payload.contains("\"range\""));
}

/// Mission `web-approvals-decider-role-on-timeline`: when the
/// underlying audit_record payload carries `decider_role` (stamped
/// by #1956), the feed projector surfaces it as a top-level field.
/// Pre-#1956 rows have no `decider_role` and must continue to render
/// without one — covered by the existing `seed_minimal_two` rows
/// that omit the field entirely.
#[tokio::test]
async fn projects_decider_role_when_payload_carries_it() {
    let store = mem_store().await;
    let mut payload = row(
        "w-rbac",
        "web",
        "approval.decided",
        "succeeded",
        Some("a1"),
        Some(0.0),
        "internal",
        "approved with role stamp",
    );
    payload["decider_role"] = serde_json::Value::String("admin".into());
    seed_row(&store, payload).await;
    let (status, body) =
        parse_body(list_response(&get("/api/v1/audit?worker=w-rbac"), Some(&store)).await).await;
    assert_eq!(status, "200 OK");
    let items = body["items"].as_array().expect("items array");
    let row_json = items
        .iter()
        .find(|it| it["worker"]["id"] == "w-rbac")
        .expect("seeded rbac row present");
    assert_eq!(row_json["decider_role"], "admin");
}

/// Backward-compat: rows without `decider_role` in payload (pre-#1956)
/// must not have the field on the projected row at all (vs e.g.
/// rendering as `null`), so the frontend's `decider_role?:` optional
/// type guard keeps holding.
#[tokio::test]
async fn omits_decider_role_for_pre_rbac_rows() {
    let store = mem_store().await;
    seed_minimal_two(&store).await;
    let (status, body) =
        parse_body(list_response(&get("/api/v1/audit?worker=w1"), Some(&store)).await).await;
    assert_eq!(status, "200 OK");
    let items = body["items"].as_array().expect("items array");
    let row_json = items
        .iter()
        .find(|it| it["worker"]["id"] == "w1")
        .expect("seeded w1 row present");
    assert!(
        row_json.get("decider_role").is_none(),
        "pre-#1956 row must omit decider_role; got {row_json}"
    );
}

/// Unknown role labels (legacy / malformed) are dropped at the
/// projection layer so the frontend's narrow `"owner" | "admin" |
/// "user" | "guest"` union stays sound.
#[tokio::test]
async fn drops_unknown_decider_role_values() {
    let store = mem_store().await;
    let mut payload = row(
        "w-legacy",
        "web",
        "approval.decided",
        "succeeded",
        Some("a1"),
        None,
        "internal",
        "legacy role value",
    );
    payload["decider_role"] = serde_json::Value::String("wizard".into());
    seed_row(&store, payload).await;
    let (status, body) =
        parse_body(list_response(&get("/api/v1/audit?worker=w-legacy"), Some(&store)).await).await;
    assert_eq!(status, "200 OK");
    let items = body["items"].as_array().expect("items array");
    let row_json = items
        .iter()
        .find(|it| it["worker"]["id"] == "w-legacy")
        .expect("seeded legacy row present");
    assert!(row_json.get("decider_role").is_none());
}
