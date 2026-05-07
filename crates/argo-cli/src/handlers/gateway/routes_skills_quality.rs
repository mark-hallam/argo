//! `GET /skills/<name>/quality` — single-skill quality view
//! (`claude-skill-quality-score-vis`).
//!
//! Returns the latest score by default; `?limit=N` widens to a
//! history block. Mirrors `argo skill quality <name> --limit N`
//! shape so dashboards + the CLI render identical data.
//!
//! Schema (always JSON; gateway is JSON-by-default for read
//! routes):
//!
//! ```json
//! {
//!   "skill_name": "alpha",
//!   "skill_id": "<uuid>",
//!   "latest": { "score": 82, "computed_at": "...", "inputs": {...} } | null,
//!   "history": [ ... up to limit-1 prior scores, newest-first ... ],
//!   "history_len": <int>
//! }
//! ```
use argo_skill::{QualityScore, SkillQualityStore, SkillStore};
use argo_store::SessionStore;
use std::sync::Arc;

/// Top-level handler — returns `(status, body)` per gateway
/// dispatch convention.
pub(super) async fn skill_quality_response(
    name: &str,
    store: Option<&Arc<SessionStore>>,
    limit: Option<usize>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let limit = limit.unwrap_or(1).max(1);
    let skills = SkillStore::new(Arc::clone(store));
    let skill = match skills.find_active_by_name(name).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            return (
                "404 Not Found",
                json_error(&format!("no active skill named '{name}'")),
            );
        }
        Err(e) => {
            return (
                "500 Internal Server Error",
                json_error(&format!("skill lookup: {e}")),
            );
        }
    };
    let qstore = SkillQualityStore::new(Arc::clone(store));
    if let Err(e) = qstore.ensure_schema().await {
        return (
            "500 Internal Server Error",
            json_error(&format!("quality schema: {e}")),
        );
    }
    let scores = match load_scores(&qstore, skill.id, limit).await {
        Ok(s) => s,
        Err(e) => {
            return (
                "500 Internal Server Error",
                json_error(&format!("quality fetch: {e}")),
            );
        }
    };
    (
        "200 OK",
        render_payload(name, &skill.id.0.to_string(), &scores).to_string(),
    )
}

async fn load_scores(
    qstore: &SkillQualityStore,
    skill_id: argo_types::SkillId,
    limit: usize,
) -> Result<Vec<QualityScore>, argo_types::ArgoError> {
    if limit <= 1 {
        return Ok(qstore.latest(skill_id).await?.into_iter().collect());
    }
    let mut hist = qstore.history(skill_id).await?;
    hist.sort_by_key(|b| std::cmp::Reverse(b.computed_at));
    if hist.len() > limit {
        hist.truncate(limit);
    }
    Ok(hist)
}

fn render_payload(name: &str, skill_id: &str, scores: &[QualityScore]) -> serde_json::Value {
    let latest = scores.first();
    let history: Vec<&QualityScore> = scores.iter().skip(1).collect();
    serde_json::json!({
        "skill_name": name,
        "skill_id": skill_id,
        "latest": latest,
        "history": history,
        "history_len": history.len(),
    })
}

fn json_error(msg: &str) -> String {
    serde_json::json!({ "error": msg }).to_string()
}

#[cfg(test)]
#[path = "routes_skills_quality_tests.rs"]
mod tests;
