//! Tests for [`super::routes_skills`] — extracted to keep the parent
//! file under HARD RULE 6's 500-LOC ceiling now that the skill-
//! discovery marketplace surface added a `/skills/<name>/usage` route
//! and a `/skills/learned-this-month` filter endpoint.
use super::*;

async fn session_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("store");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

async fn skill_store() -> SkillStore {
    SkillStore::new(session_store().await)
}

async fn install(store: &SkillStore, name: &str, source: &str) {
    store
        .install(&skill_md(name, source), None)
        .await
        .expect("install skill");
}

fn skill_md(name: &str, source: &str) -> String {
    format!(
        "---\n\
         name: {name}\n\
         version: \"0.1.0\"\n\
         description: {name} description\n\
         provenance:\n  \
         source: {source}\n  \
         hash: sha256-deadbeef\n\
         ---\n\
         body of {name}\n"
    )
}

#[tokio::test]
async fn dispatch_lists_workspace_skills_with_risk_filter_and_shape() {
    let session = session_store().await;
    let skills = SkillStore::new(Arc::clone(&session));
    install(&skills, "workspace-verified", "published:verified:team").await;
    install(&skills, "workspace-learned", "learned").await;

    let (status, body) = dispatch_route("GET /skills?risk=high HTTP/1.1\r\n\r\n", Some(&session))
        .await
        .expect("route");
    assert_eq!(status, "200 OK");
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(parsed["risk"], "high");
    assert_eq!(parsed["risk_defaulted"], false);
    let rows = parsed["skills"].as_array().expect("skills");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["name"], "workspace-verified");
    assert_eq!(rows[0]["layer"], "workspace");
    assert_eq!(rows[0]["trust_grade"], "published_verified");

    let (status, body) = dispatch_route(
        "GET /skills/workspace-verified HTTP/1.1\r\n\r\n",
        Some(&session),
    )
    .await
    .expect("route");
    assert_eq!(status, "200 OK");
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(parsed["name"], "workspace-verified");
    assert_eq!(parsed["layer"], "workspace");
}

#[tokio::test]
async fn layered_lookup_adds_library_fallback_and_workspace_override() {
    let workspace = skill_store().await;
    let library = skill_store().await;
    install(&workspace, "shared", "published:community:team").await;
    install(&library, "shared", "published:verified:market").await;
    install(&library, "library-only", "published:verified:market").await;

    let lookup = LayeredLookup::with_library(&workspace, &library);
    let (status, body) = list_response_for_lookup(&lookup, None).await;
    assert_eq!(status, "200 OK");
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(parsed["risk"], "medium");
    assert_eq!(parsed["risk_defaulted"], true);
    let rows = parsed["skills"].as_array().expect("skills");
    assert_eq!(rows.len(), 2);
    let shared = rows
        .iter()
        .find(|row| row["name"] == "shared")
        .expect("shared row");
    assert_eq!(shared["layer"], "workspace");
    let library_only = rows
        .iter()
        .find(|row| row["name"] == "library-only")
        .expect("library row");
    assert_eq!(library_only["layer"], "library");

    let (status, body) = show_response_for_lookup(&lookup, "library-only").await;
    assert_eq!(status, "200 OK");
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(parsed["name"], "library-only");
    assert_eq!(parsed["layer"], "library");
}

#[tokio::test]
async fn invalid_risk_returns_400() {
    let workspace = skill_store().await;
    let lookup = LayeredLookup::workspace_only(&workspace);
    let (status, body) = list_response_for_lookup(&lookup, Some("urgent")).await;
    assert_eq!(status, "400 Bad Request");
    assert!(body.contains("invalid risk 'urgent'"));
}

#[tokio::test]
async fn non_layered_skill_routes_fall_through() {
    assert!(dispatch_route("GET /skills/count HTTP/1.1\r\n\r\n", None)
        .await
        .is_none());
    assert!(
        dispatch_route("GET /skills/example/versions HTTP/1.1\r\n\r\n", None)
            .await
            .is_none()
    );
}

#[test]
fn author_label_classification_is_skycot_or_org() {
    assert_eq!(author_label_for_source("builtin"), "Authored by SKYCOT");
    assert_eq!(author_label_for_source("verticals"), "Authored by SKYCOT");
    assert_eq!(
        author_label_for_source("published:verified:market"),
        "Authored by SKYCOT"
    );
    assert_eq!(
        author_label_for_source("published:community:market"),
        "Authored by SKYCOT"
    );
    assert_eq!(author_label_for_source("learned"), "Authored by your org");
    assert_eq!(
        author_label_for_source("published:verified:team"),
        "Authored by your org"
    );
}

#[test]
fn month_start_iso_pins_to_first_of_month_utc() {
    let mid = chrono::DateTime::parse_from_rfc3339("2026-05-21T15:30:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);
    assert!(month_start_iso(mid).starts_with("2026-05-01T00:00:00"));
}

#[tokio::test]
async fn skill_usage_endpoint_returns_provenance_and_workers() {
    let session = session_store().await;
    let skills = SkillStore::new(Arc::clone(&session));
    install(&skills, "approval.route", "published:verified:market").await;

    let (status, body) = dispatch_route(
        "GET /skills/approval.route/usage HTTP/1.1\r\n\r\n",
        Some(&session),
    )
    .await
    .expect("route");
    assert_eq!(status, "200 OK");
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(parsed["skill_name"], "approval.route");
    assert_eq!(parsed["author_label"], "Authored by SKYCOT");
    assert_eq!(parsed["trust_grade"], "published_verified");
    assert_eq!(parsed["workers_using_count"], 0);
    assert_eq!(parsed["adoption_count"], 0);
    assert!(parsed["workers_using"].is_array());
}

#[tokio::test]
async fn learned_this_month_endpoint_returns_envelope() {
    let session = session_store().await;
    let (status, body) = dispatch_route(
        "GET /skills/learned-this-month HTTP/1.1\r\n\r\n",
        Some(&session),
    )
    .await
    .expect("route");
    assert_eq!(status, "200 OK");
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert!(parsed["since"].is_string());
    assert!(parsed["skill_ids"].is_array());
    assert_eq!(parsed["count"], 0);
}
