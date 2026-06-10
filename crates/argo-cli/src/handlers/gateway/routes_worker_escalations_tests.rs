//! Unit + integration tests for `routes_worker_escalations`.
//!
//! The pure helpers (body-parse, TOML round-trip) pin the wire
//! schema; the integration tests exercise the full HTTP path against
//! an in-memory store, asserting the PATCH→GET round-trip persists the
//! operator's rules (not a fixture), a write audit row is appended,
//! and the cross-tenant 404 is byte-identical to the unknown-worker
//! 404 (the worker-policies invariant).

use super::*;
use tempfile::tempdir;

fn id() -> WorkerId {
    "00000000-0000-0000-0000-000000000001"
        .parse::<WorkerId>()
        .expect("valid test worker_id")
}

#[test]
fn parse_patch_body_accepts_valid_input() {
    let body = r#"{
        "rules": [
            {"action":"Issue a refund","condition":"Refund exceeds $200","routes_to":"Finance","channel":"Email"},
            {"action":"Close a ticket","condition":"Customer mentions churn","routes_to":"Account manager"}
        ],
        "operator_note":"tightened the refund threshold"
    }"#;
    let parsed = parse_patch_body(body).expect("valid body parses");
    assert_eq!(parsed.rules.len(), 2);
    assert_eq!(parsed.rules[0].action, "Issue a refund");
    assert_eq!(parsed.rules[0].channel.as_deref(), Some("Email"));
    assert_eq!(parsed.rules[1].channel, None);
    assert_eq!(parsed.operator_note, "tightened the refund threshold");
}

#[test]
fn parse_patch_body_accepts_empty_rule_set() {
    let body = r#"{"rules":[],"operator_note":"cleared all escalations"}"#;
    let parsed = parse_patch_body(body).expect("empty rules is valid");
    assert!(parsed.rules.is_empty());
}

#[test]
fn parse_patch_body_requires_rules_array() {
    let body = r#"{"operator_note":"oops"}"#;
    let err = parse_patch_body(body).expect_err("missing rules rejects");
    assert!(err.contains("`rules`"), "got: {err}");
}

#[test]
fn parse_patch_body_rejects_rule_missing_required_field() {
    let body = r#"{"rules":[{"action":"x","condition":"y"}],"operator_note":"n"}"#;
    let err = parse_patch_body(body).expect_err("missing routes_to rejects");
    assert!(err.contains("routes_to"), "got: {err}");
}

#[test]
fn parse_patch_body_rejects_blank_field() {
    let body =
        r#"{"rules":[{"action":"  ","condition":"y","routes_to":"z"}],"operator_note":"n"}"#;
    let err = parse_patch_body(body).expect_err("blank action rejects");
    assert!(err.contains("non-empty"), "got: {err}");
}

#[test]
fn parse_patch_body_requires_non_empty_operator_note() {
    let missing = r#"{"rules":[]}"#;
    let e1 = parse_patch_body(missing).expect_err("missing note rejects");
    assert!(e1.contains("operator_note"), "got: {e1}");

    let blank = r#"{"rules":[],"operator_note":"   "}"#;
    let e2 = parse_patch_body(blank).expect_err("whitespace-only note rejects");
    assert!(e2.contains("non-empty"), "got: {e2}");
}

#[test]
fn read_rules_returns_empty_when_file_absent() {
    let dir = tempdir().expect("tempdir");
    let rules = read_rules(dir.path(), id()).expect("missing file is not an error");
    assert!(rules.is_empty());
}

#[test]
fn write_then_read_round_trips_the_rule_table() {
    let dir = tempdir().expect("tempdir");
    let worker = id();
    let rules = vec![
        EscalationRule {
            action: "Re-price a listing".to_string(),
            condition: "Price drops below floor".to_string(),
            routes_to: "Seller".to_string(),
            channel: Some("Email".to_string()),
        },
        EscalationRule {
            action: "Approve photos".to_string(),
            condition: "Edited-photo detector flags it".to_string(),
            routes_to: "Owner".to_string(),
            channel: None,
        },
    ];
    write_rules(dir.path(), worker, &rules).expect("write");

    let path = worker_escalation_path(dir.path(), worker);
    assert!(path.exists(), "file written at {}", path.display());

    let round = read_rules(dir.path(), worker).expect("read");
    assert_eq!(round, rules, "rule set round-trips through TOML");
}

#[test]
fn read_rules_skips_rows_missing_required_fields() {
    // Hand-edited TOML on disk may carry a malformed row. The reader
    // drops rows that don't carry the three required fields so the UI
    // never surfaces a half-formed rule.
    let dir = tempdir().expect("tempdir");
    let worker = id();
    let path = worker_escalation_path(dir.path(), worker);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(
        &path,
        "[[rule]]\naction = \"good\"\ncondition = \"when\"\nroutes_to = \"who\"\n\n[[rule]]\naction = \"orphan\"\n",
    )
    .unwrap();
    let rules = read_rules(dir.path(), worker).expect("read");
    assert_eq!(rules.len(), 1, "only the complete row survives");
    assert_eq!(rules[0].action, "good");
}

/// Cross-tenant existence is hidden from the HTTP response. Both
/// `list_response` (GET /escalations) and `update_response`
/// (PATCH /escalations) must return byte-identical 404 for the
/// cross-tenant and nonexistent branches, mirroring the
/// worker-policies invariant. The round-trip + audit assertions also
/// live here so the full HTTP path is covered.
mod integration {
    use super::super::{list_response, update_response};
    use argo_identity::{IdentityStore, Persona, Role, Worker, WorkerId};
    use argo_store::SessionStore;
    use argo_types::TenantId;
    use std::path::PathBuf;
    use std::sync::Arc;

    fn workspace_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .expect("workspace root")
            .to_path_buf()
    }

    fn set_test_env(tmp: &std::path::Path) {
        std::env::set_var("ARGO_HOME", tmp.join("home"));
        std::fs::create_dir_all(tmp.join("home")).expect("home dir");
        std::env::set_var("ARGO_VERTICALS_DIR", workspace_root().join("verticals"));
    }

    async fn mem_store() -> Arc<SessionStore> {
        // Force the audit sink onto its synchronous per-row insert path
        // so a write is immediately readable in the same test (the
        // handler builds a short-lived `AuditSink` we cannot flush from
        // here). The value is always "0" wherever any test sets it.
        std::env::set_var("ARGO_STORE_BATCHED_WRITES", "0");
        let store = SessionStore::open(":memory:").await.expect("open mem db");
        store.migrate().await.expect("migrate");
        Arc::new(store)
    }

    async fn workspace_tenant(store: &Arc<SessionStore>) -> TenantId {
        let cwd = std::env::current_dir().expect("cwd");
        let name = cwd
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("workspace");
        let (tenant, _) = store
            .ensure_workspace(name, &cwd.to_string_lossy())
            .await
            .expect("ensure workspace");
        tenant
    }

    async fn add_worker(store: &Arc<SessionStore>, tenant: TenantId, name: &str) -> Worker {
        let worker = Worker::draft(
            tenant,
            "support",
            Persona::named(name, "human:mark"),
            Role::new("test."),
        );
        IdentityStore::new(store)
            .create_worker(&worker)
            .await
            .expect("create worker");
        worker
    }

    fn get_req(id: &WorkerId) -> String {
        format!("GET /api/v1/workers/{id}/escalations HTTP/1.1\r\nHost: x\r\n\r\n")
    }

    fn patch_req(id: &WorkerId, body: &str) -> String {
        let len = body.len();
        format!(
            "PATCH /api/v1/workers/{id}/escalations HTTP/1.1\r\nHost: x\r\nContent-Length: {len}\r\n\r\n{body}"
        )
    }

    /// PATCH a rule set for a worker in the caller's tenant, then GET
    /// it back: the persisted rules (not a fixture) must round-trip,
    /// and a `worker.escalation_changed` audit row must be appended.
    #[tokio::test]
    async fn patch_then_get_round_trips_and_audits() {
        let _g = crate::handlers::gateway::shared_env_lock().await;
        let tmp = tempfile::tempdir().expect("tempdir");
        set_test_env(tmp.path());
        // Run from a clean cwd so the on-disk TOML lands under tmp and
        // does not collide with the real repo's .skycot dir.
        let prev_cwd = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(tmp.path()).expect("chdir");

        let store = mem_store().await;
        let tenant = workspace_tenant(&store).await;
        let worker = add_worker(&store, tenant, "Jamal").await;

        let body = r#"{
            "rules":[
                {"action":"Issue a refund","condition":"Refund exceeds $200","routes_to":"Finance","channel":"Email"}
            ],
            "operator_note":"set the refund escalation"
        }"#;
        let (status, resp) =
            update_response(&worker.id.to_string(), &patch_req(&worker.id, body), Some(&store))
                .await;
        assert_eq!(status, "200 OK", "patch body: {resp}");

        let (gstatus, gbody) =
            list_response(&worker.id.to_string(), &get_req(&worker.id), Some(&store)).await;
        assert_eq!(gstatus, "200 OK", "get body: {gbody}");
        let parsed: serde_json::Value = serde_json::from_str(&gbody).expect("json");
        assert_eq!(parsed["is_default"], serde_json::json!(false));
        let rules = parsed["rules"].as_array().expect("rules array");
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0]["action"], "Issue a refund");
        assert_eq!(rules[0]["routes_to"], "Finance");
        assert_eq!(rules[0]["channel"], "Email");

        // A write audit row was appended (synchronous path forced via
        // ARGO_STORE_BATCHED_WRITES=0 in `mem_store`).
        let payloads: Vec<String> = sqlx::query_scalar(
            "SELECT payload_json FROM audit_record WHERE kind = 'http.request'",
        )
        .fetch_all(store.pool())
        .await
        .expect("query audit rows");
        assert!(
            payloads
                .iter()
                .any(|p| p.contains("worker.escalation_changed")),
            "expected a worker.escalation_changed audit row, got: {payloads:?}"
        );

        std::env::set_current_dir(prev_cwd).expect("restore cwd");
    }

    #[tokio::test]
    async fn cross_tenant_get_returns_same_404_as_unknown_worker() {
        let _g = crate::handlers::gateway::shared_env_lock().await;
        let tmp = tempfile::tempdir().expect("tempdir");
        set_test_env(tmp.path());
        let store = mem_store().await;
        let _ = workspace_tenant(&store).await;
        let other = TenantId::new();
        let alien = add_worker(&store, other, "Alien").await;
        let ghost = WorkerId::new();
        let (status_alien, body_alien) =
            list_response(&alien.id.to_string(), &get_req(&alien.id), Some(&store)).await;
        let (status_ghost, body_ghost) =
            list_response(&ghost.to_string(), &get_req(&ghost), Some(&store)).await;
        assert_eq!(status_alien, "404 Not Found", "body: {body_alien}");
        assert_eq!(status_alien, status_ghost);
        assert_eq!(
            body_alien, body_ghost,
            "GET /escalations: cross-tenant body must equal unknown-worker body"
        );
        assert!(!body_alien.contains(&alien.id.to_string()));
    }

    #[tokio::test]
    async fn cross_tenant_update_returns_same_404_as_unknown_worker() {
        let _g = crate::handlers::gateway::shared_env_lock().await;
        let tmp = tempfile::tempdir().expect("tempdir");
        set_test_env(tmp.path());
        let store = mem_store().await;
        let _ = workspace_tenant(&store).await;
        let other = TenantId::new();
        let alien = add_worker(&store, other, "Alien").await;
        let ghost = WorkerId::new();
        let body = r#"{"rules":[],"operator_note":"why"}"#;
        let (status_alien, body_alien) =
            update_response(&alien.id.to_string(), &patch_req(&alien.id, body), Some(&store)).await;
        let (status_ghost, body_ghost) =
            update_response(&ghost.to_string(), &patch_req(&ghost, body), Some(&store)).await;
        assert_eq!(status_alien, "404 Not Found", "body: {body_alien}");
        assert_eq!(status_alien, status_ghost);
        assert_eq!(
            body_alien, body_ghost,
            "PATCH /escalations: cross-tenant body must equal unknown-worker body"
        );
        assert!(!body_alien.contains(&alien.id.to_string()));
    }
}
