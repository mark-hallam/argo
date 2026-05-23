//! FINDINGS-ONLY tests for [`super::routes_skill_refine`]
//! (`eval-argo-cli-gateway-module-sweep`, 2026-05-23).
//!
//! The audit found three missing wires on `POST /skills/<name>/refine`:
//!
//! 1. No RBAC gate — any caller that passes the upstream bearer gate
//!    can mutate skill rows. Other mutating gateway routes call
//!    `enforce_role(req, Action::<X>)`; `skill_refine_response` does
//!    not.
//! 2. No audit row — the handler supersedes the active skill row but
//!    writes nothing to `audit_record`. Mission claim 6 requires
//!    "audit chain on every action".
//! 3. No prior test coverage — the source file has no `#[cfg(test)]`
//!    module, so the missing wires never tripped CI.
//!
//! The tests below are `#[ignore]`'d (FINDINGS-ONLY pattern from the
//! brief): they document the post-fix expected behaviour so that the
//! conductor can un-ignore them once
//! `mission-skill-refine-add-rbac-audit-and-test-coverage` lands.

use super::skill_refine_response;
use argo_skill::SkillStore;
use argo_store::SessionStore;
use serde_json::json;
use std::sync::Arc;

async fn session_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("open mem db");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

fn skill_md(name: &str, source: &str) -> String {
    format!(
        "---\nname: {name}\nversion: \"0.1.0\"\ndescription: {name} description\n\
         provenance:\n  source: {source}\n  hash: sha256-deadbeef\n---\nbody of {name}\n"
    )
}

async fn install(store: &Arc<SessionStore>, name: &str, source: &str) {
    SkillStore::new(Arc::clone(store))
        .install(&skill_md(name, source), None)
        .await
        .expect("install skill");
}

fn refine_post(name: &str, role_header: Option<&str>, hint: &str) -> String {
    let body = json!({ "hint": hint }).to_string();
    let role_line = role_header
        .map(|r| format!("X-Argo-Role: {r}\r\n"))
        .unwrap_or_default();
    format!(
        "POST /skills/{name}/refine HTTP/1.1\r\nHost: x\r\n{role_line}\
         Content-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

async fn audit_row_count(store: &Arc<SessionStore>) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM audit_record")
        .fetch_one(store.pool())
        .await
        .unwrap_or(0)
}

/// FINDINGS-ONLY: today `routes_skill_refine` has no
/// `enforce_role(req, Action::<X>)` call. A guest-role caller can
/// successfully mutate skill rows. Fix-brief:
/// `mission-skill-refine-add-rbac-audit-and-test-coverage`.
///
/// Expected post-fix: 403 with the standard rbac_denied envelope from
/// `role_gate::deny_response`.
#[tokio::test]
#[ignore = "FINDINGS-ONLY: routes_skill_refine has no RBAC gate today"]
async fn refine_with_guest_role_returns_403() {
    let store = session_store().await;
    install(&store, "alpha", "learned").await;

    let req = refine_post("alpha", Some("guest"), "tighten the close phrase");
    let (status, body) = skill_refine_response("alpha", &req, Some(&store)).await;

    assert_eq!(
        status, "403 Forbidden",
        "guest must be refused (post-fix); today body={body}"
    );
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(parsed["rbac"]["denied"], true);
}

/// FINDINGS-ONLY: today a successful refine writes zero rows to
/// `audit_record`. Mission claim 6 requires an audit row per action.
/// Fix-brief: `mission-skill-refine-add-rbac-audit-and-test-coverage`.
///
/// Expected post-fix: ≥1 audit row with `endpoint=skill.refined`
/// (kind=HttpRequest) carrying skill_id + prev_version + new_version.
#[tokio::test]
#[ignore = "FINDINGS-ONLY: routes_skill_refine emits no audit_record row today"]
async fn refine_emits_audit_row() {
    let store = session_store().await;
    install(&store, "alpha", "learned").await;
    let before = audit_row_count(&store).await;

    let req = refine_post("alpha", None, "tighten the close phrase");
    let (status, _body) = skill_refine_response("alpha", &req, Some(&store)).await;
    assert_eq!(status, "200 OK");

    let after = audit_row_count(&store).await;
    assert!(
        after > before,
        "successful refine must append ≥1 audit row (post-fix); before={before} after={after}"
    );
}
