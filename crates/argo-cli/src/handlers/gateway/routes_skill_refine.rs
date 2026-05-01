//! `POST /skills/<name>/refine` — append a `## Refinement`
//! block to the active skill's body and supersede it with a
//! bumped version. Mirrors `argo skill refine` (#271). Lives
//! in its own file so `routes_post_admin.rs` stays under the
//! 500-LOC budget.
use super::{body_from_request, store_unavailable};
use argo_skill::{PrincipalOverride as SkillPrincipal, SkillStore};
use argo_store::SessionStore;
use sha2::{Digest, Sha256};
use std::sync::Arc;

/// Body `{hint, version?}`. Returns 200 + `{refined, name,
/// prev_version, new_version, skill_id}`. 400 on missing hint
/// or supersede failure; 404 when the named skill has no
/// active row.
pub(super) async fn skill_refine_response(
    name: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let parsed: serde_json::Value = match serde_json::from_str(body_from_request(req)) {
        Ok(v) => v,
        Err(e) => {
            return (
                "400 Bad Request",
                format!("{{\"error\":\"parse JSON body: {e}\"}}"),
            );
        }
    };
    let Some(hint) = parsed
        .get("hint")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    else {
        return (
            "400 Bad Request",
            "{\"error\":\"missing or empty `hint`\"}".to_string(),
        );
    };
    let version_override = parsed
        .get("version")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(str::to_string);
    let skills = SkillStore::new(Arc::clone(store));
    let active = match skills.find_active_by_name(name).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            return (
                "404 Not Found",
                format!("{{\"error\":\"no active skill named '{name}'\"}}"),
            );
        }
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"find_active_by_name: {e}\"}}"),
            );
        }
    };
    let new_version =
        version_override.unwrap_or_else(|| bump_skill_version(&active.frontmatter.version));
    let new_body = format!("{}\n\n## Refinement\n{}\n", active.body.trim_end(), hint);
    let raw = format!(
        "---\nname: {name}\nversion: \"{new_version}\"\ndescription: {desc}\n\
         lock: none\nprovenance:\n  source: refined-from-{prev}\n  hash: sha256-{hash}\n---\n{body}",
        desc = active.frontmatter.description,
        prev = active.frontmatter.version,
        hash = sha256_hex(&new_body),
        body = new_body,
    );
    match skills.supersede(&raw, None, SkillPrincipal::Force).await {
        Ok(new_skill) => {
            let payload = serde_json::json!({
                "refined": true,
                "name": name,
                "prev_version": active.frontmatter.version,
                "new_version": new_version,
                "skill_id": new_skill.id.0.to_string(),
            });
            (
                "200 OK",
                serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Err(e) => (
            "400 Bad Request",
            format!("{{\"error\":\"supersede: {e}\"}}"),
        ),
    }
}

fn bump_skill_version(v: &str) -> String {
    let mut parts: Vec<String> = v.split('.').map(String::from).collect();
    if let Some(last) = parts.last_mut() {
        if let Ok(n) = last.parse::<u64>() {
            *last = (n + 1).to_string();
            return parts.join(".");
        }
    }
    format!("{v}-refined")
}

fn sha256_hex(s: &str) -> String {
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    hex::encode(h.finalize())
}
