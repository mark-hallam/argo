//! `POST /forge/runs/<id>/promote` — promote a forge run's
//! candidate_version. Lifted out of `routes_post_writes.rs`
//! so that file stays under the 500-LOC ceiling.
use super::store_unavailable;
use argo_store::SessionStore;
use std::sync::Arc;

/// Convenience wrapper over `POST /skills/<name>/promote` —
/// the gateway looks up `(skill_name, candidate_version)`
/// from the forge_run row. 200 + `{promoted, name, version,
/// skill_id, run_id}`. 400 on bad uuid; 404 when no row
/// matches; 409 when the run hasn't produced a candidate
/// yet.
pub(super) async fn forge_promote_response(
    raw_id: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let id = match uuid::Uuid::parse_str(raw_id) {
        Ok(u) => u,
        Err(_) => {
            return (
                "400 Bad Request",
                format!("{{\"error\":\"invalid forge run id '{raw_id}'\"}}"),
            );
        }
    };
    let row = match store.get_forge_run(id).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            return (
                "404 Not Found",
                format!("{{\"error\":\"no forge run with id {raw_id}\"}}"),
            );
        }
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"get_forge_run: {e}\"}}"),
            );
        }
    };
    let Some(version) = row.candidate_version.clone() else {
        return (
            "409 Conflict",
            format!(
                "{{\"error\":\"forge run {raw_id} has no candidate_version yet — \
                 wait for at least one iteration to complete\"}}"
            ),
        );
    };
    let skills = argo_skill::SkillStore::new(Arc::clone(store));
    match skills.promote(&row.skill_name, &version).await {
        Ok(s) => {
            let resp = serde_json::json!({
                "promoted": true,
                "name": row.skill_name,
                "version": version,
                "skill_id": s.id.0.to_string(),
                "run_id": raw_id,
            });
            (
                "200 OK",
                serde_json::to_string(&resp).unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Err(e) => (
            "400 Bad Request",
            format!("{{\"error\":\"promote: {e}\"}}"),
        ),
    }
}
