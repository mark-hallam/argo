//! `GET /skills?risk=...` + `GET /skills/<name>` — layered
//! skill lookup routes for the worker runtime and review UI.
use argo_skill::{LayeredLookup, LayeredSkill, SkillStore, TrustGrade};
use argo_store::SessionStore;
use std::sync::Arc;

pub(super) async fn dispatch_route(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if req.starts_with("GET /skills ") || req.starts_with("GET /skills?") {
        let tail = req.strip_prefix("GET ").unwrap_or(req);
        let risk = super::routes_query::parse_query_kv(tail, "risk");
        return Some(skills_list_response(store, risk.as_deref()).await);
    }

    let rest = req.strip_prefix("GET /skills/")?;
    let (name_with_query, _) = rest.split_once(' ')?;
    let name = name_with_query
        .split_once('?')
        .map(|(name, _)| name)
        .unwrap_or(name_with_query);
    if name == "count" || name.contains('/') {
        return None;
    }
    Some(skill_show_response(store, name).await)
}

async fn skills_list_response(
    store: Option<&Arc<SessionStore>>,
    raw_risk: Option<&str>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let skills = SkillStore::new(Arc::clone(store));
    let lookup = LayeredLookup::workspace_only(&skills);
    // TODO(H-4b follow-up): replace workspace_only with with_library
    // once the gateway owns a distinct read-only library store.
    list_response_for_lookup(&lookup, raw_risk).await
}

async fn skill_show_response(
    store: Option<&Arc<SessionStore>>,
    name: &str,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let skills = SkillStore::new(Arc::clone(store));
    let lookup = LayeredLookup::workspace_only(&skills);
    show_response_for_lookup(&lookup, name).await
}

async fn list_response_for_lookup(
    lookup: &LayeredLookup<'_>,
    raw_risk: Option<&str>,
) -> (&'static str, String) {
    let (risk, risk_defaulted) = match parse_risk(raw_risk) {
        Ok(r) => r,
        Err(msg) => return ("400 Bad Request", json_error(&msg)),
    };
    match lookup.list_for_action(risk).await {
        Ok(rows) => {
            let skills: Vec<serde_json::Value> = rows.iter().map(layered_skill_json).collect();
            let payload = serde_json::json!({
                "risk": risk,
                "risk_defaulted": risk_defaulted,
                "count": skills.len(),
                "skills": skills,
            });
            (
                "200 OK",
                serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            json_error(&format!("layered skills list: {e}")),
        ),
    }
}

async fn show_response_for_lookup(
    lookup: &LayeredLookup<'_>,
    name: &str,
) -> (&'static str, String) {
    match lookup.find(name).await {
        Ok(Some(row)) => (
            "200 OK",
            serde_json::to_string(&layered_skill_json(&row)).unwrap_or_else(|_| "{}".to_string()),
        ),
        Ok(None) => (
            "404 Not Found",
            json_error(&format!(
                "no active skill named '{name}' in workspace or library"
            )),
        ),
        Err(e) => (
            "500 Internal Server Error",
            json_error(&format!("layered skill show: {e}")),
        ),
    }
}

fn parse_risk(raw: Option<&str>) -> Result<(&'static str, bool), String> {
    match raw {
        None => Ok(("medium", true)),
        Some("low") => Ok(("low", false)),
        Some("medium") => Ok(("medium", false)),
        Some("high") => Ok(("high", false)),
        Some(other) => Err(format!("invalid risk '{other}'; use low|medium|high")),
    }
}

fn layered_skill_json(row: &LayeredSkill) -> serde_json::Value {
    let skill = &row.skill;
    serde_json::json!({
        "id": skill.id.0.to_string(),
        "name": skill.frontmatter.name,
        "version": skill.frontmatter.version,
        "description": skill.frontmatter.description,
        "layer": row.layer,
        "provenance_source": skill.frontmatter.provenance.source,
        "trust_grade": TrustGrade::from_source(&skill.frontmatter.provenance.source),
        "lock": format!("{:?}", skill.frontmatter.lock),
        "hash": hex::encode(skill.hash),
        "body": skill.body,
        "created_at": skill.created_at.to_rfc3339(),
    })
}

fn json_error(msg: &str) -> String {
    serde_json::json!({ "error": msg }).to_string()
}

#[cfg(test)]
mod tests {
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

        let (status, body) =
            dispatch_route("GET /skills?risk=high HTTP/1.1\r\n\r\n", Some(&session))
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
}
