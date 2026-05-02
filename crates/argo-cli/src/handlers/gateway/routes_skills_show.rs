//! `GET /skills/<name>` + `GET /skills/<name>/versions` — show
//! handlers extracted from `routes_meta.rs` so that file stays
//! under the 500-LOC budget. Pure store reads + JSON / markdown
//! rendering, no dispatch.
use argo_store::SessionStore;
use std::sync::Arc;

/// `GET /skills/<name>/versions[?format=markdown]` — list every
/// row matching `<name>` (active + superseded) as a JSON array
/// sorted by `created_at` ASC. Useful for visualising the
/// supersede chain after a `forge optimize` or `skill refine`
/// cycle. Markdown renders a 4-column table with active row
/// emphasised.
pub(super) async fn skill_versions_response(
    name: &str,
    store: Option<&Arc<SessionStore>>,
    format: Option<&str>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let skills = argo_skill::SkillStore::new(Arc::clone(store));
    match skills.versions_for_name(name).await {
        Ok(rows) if rows.is_empty() => (
            "404 Not Found",
            format!("{{\"error\":\"no skill named '{name}'\"}}"),
        ),
        Ok(rows) => {
            if matches!(format, Some("markdown")) {
                let n = rows.len();
                let active = rows
                    .iter()
                    .find(|s| s.superseded_by.is_none())
                    .map(|s| s.frontmatter.version.as_str())
                    .unwrap_or("(none)");
                let mut out = format!(
                    "# Skill `{name}` — {n} version{} (active: `{active}`)\n\n",
                    if n == 1 { "" } else { "s" }
                );
                out.push_str("| version | status | hash | created_at |\n");
                out.push_str("| --- | :---: | --- | --- |\n");
                for s in &rows {
                    let status = if s.superseded_by.is_some() {
                        "superseded"
                    } else {
                        "**active**"
                    };
                    let hash_short: String = hex::encode(s.hash).chars().take(12).collect();
                    out.push_str(&format!(
                        "| `{}` | {status} | `{hash_short}…` | {} |\n",
                        s.frontmatter.version,
                        s.created_at.to_rfc3339(),
                    ));
                }
                return ("200 OK", out);
            }
            let arr: Vec<serde_json::Value> = rows
                .iter()
                .map(|s| {
                    serde_json::json!({
                        "id": s.id.0.to_string(),
                        "version": s.frontmatter.version,
                        "hash": hex::encode(s.hash),
                        "lock": format!("{:?}", s.frontmatter.lock),
                        "superseded_by": s.superseded_by.map(|i| i.0.to_string()),
                        "created_at": s.created_at.to_rfc3339(),
                    })
                })
                .collect();
            (
                "200 OK",
                serde_json::to_string(&arr).unwrap_or_else(|_| "[]".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"versions_for_name: {e}\"}}"),
        ),
    }
}

/// `GET /skills/<name>[?format=markdown]` — single skill row
/// with body. Default JSON shape (`{id, name, version,
/// description, lock, hash, body, created_at}`). With
/// `?format=markdown`, reconstructs the original SKILL.md
/// document (YAML frontmatter + body) — paste-able straight
/// into `argo skill install`.
pub(super) async fn skill_show_response_with_format(
    name: &str,
    store: Option<&Arc<SessionStore>>,
    format: Option<&str>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let skills = argo_skill::SkillStore::new(Arc::clone(store));
    match skills.find_active_by_name(name).await {
        Ok(Some(s)) => {
            if matches!(format, Some("markdown")) {
                let mut out = String::from("---\n");
                out.push_str(&format!("name: {}\n", s.frontmatter.name));
                out.push_str(&format!("version: \"{}\"\n", s.frontmatter.version));
                out.push_str(&format!(
                    "description: {}\n",
                    s.frontmatter.description
                ));
                out.push_str(&format!("lock: {:?}\n", s.frontmatter.lock));
                out.push_str("provenance:\n");
                out.push_str(&format!("  hash: sha256-{}\n", hex::encode(s.hash)));
                out.push_str("---\n\n");
                out.push_str(&s.body);
                if !s.body.ends_with('\n') {
                    out.push('\n');
                }
                return ("200 OK", out);
            }
            let body = serde_json::json!({
                "id": s.id.0.to_string(),
                "name": s.frontmatter.name,
                "version": s.frontmatter.version,
                "description": s.frontmatter.description,
                "lock": format!("{:?}", s.frontmatter.lock),
                "hash": hex::encode(s.hash),
                "body": s.body,
                "created_at": s.created_at.to_rfc3339(),
            });
            (
                "200 OK",
                serde_json::to_string(&body).unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Ok(None) => (
            "404 Not Found",
            format!("{{\"error\":\"no active skill named '{name}'\"}}"),
        ),
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"skill_show: {e}\"}}"),
        ),
    }
}
