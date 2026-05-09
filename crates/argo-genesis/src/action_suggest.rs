use crate::error::GenesisError;
use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub struct SuggestedAction {
    pub id: String,
    pub source: ActionSource,
    pub score: f32,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionSource {
    Connector(String),
    Skill(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectorSummary {
    pub id: String,
    pub tools: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillSummary {
    pub id: String,
    pub vertical: String,
}

/// Suggest the top `top_k` action candidates for a role description.
pub fn suggest(role_description: &str, top_k: usize) -> Result<Vec<SuggestedAction>, GenesisError> {
    if top_k == 0 {
        return Ok(Vec::new());
    }
    let role = role_description.trim();
    if role.is_empty() {
        return Err(GenesisError::Validation {
            stage: "action_suggest.suggest",
            detail: "role_description is empty".into(),
        });
    }

    let role_tokens = tokenize(role);
    let mut out = Vec::new();
    for connector in enumerate_connectors()? {
        push_connector(&mut out, &role_tokens, &connector);
    }
    for skill in enumerate_skills(&workspace_root().join("verticals"))? {
        push_skill(&mut out, &role_tokens, &skill);
    }
    out.sort_by(compare_candidates);
    out.retain(|action| action.score > 0.0);
    add_generic_fallbacks(&mut out, top_k);
    out.truncate(top_k);
    Ok(out)
}

/// Enumerate MCP connector crates in the workspace.
pub fn enumerate_connectors() -> Result<Vec<ConnectorSummary>, GenesisError> {
    let mut out = Vec::new();
    for path in child_paths(
        &workspace_root().join("crates"),
        "action_suggest.connectors",
    )? {
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if !name.starts_with("argo-mcp-connector-") {
            continue;
        }
        let cargo = path.join("Cargo.toml");
        if !cargo.is_file() {
            continue;
        }
        let body = read_to_string(&cargo, "action_suggest.connector.cargo")?;
        let value: toml::Value = toml::from_str(&body).map_err(|e| GenesisError::Validation {
            stage: "action_suggest.connector_toml",
            detail: format!("{}: {e}", cargo.display()),
        })?;
        let crate_name = toml_package_str(&value, "name");
        if crate_name.is_empty() {
            continue;
        }
        let id = crate_name
            .strip_prefix("argo-mcp-connector-")
            .unwrap_or(&crate_name)
            .to_string();
        out.push(ConnectorSummary {
            tools: connector_tools(&id),
            id,
        });
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

/// Enumerate skills below `verticals_root/*/skills/`.
///
/// Missing per-vertical `skills/` directories are skipped.
pub fn enumerate_skills(verticals_root: &Path) -> Result<Vec<SkillSummary>, GenesisError> {
    let mut out = Vec::new();
    for vertical_path in child_paths(verticals_root, "action_suggest.skills.root")? {
        if !vertical_path.is_dir() {
            continue;
        }
        let Some(vertical) = vertical_path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        let skills_dir = vertical_path.join("skills");
        if !skills_dir.is_dir() {
            continue;
        }
        for path in child_paths(&skills_dir, "action_suggest.skills.dir")? {
            if path.is_file()
                && matches!(
                    path.extension().and_then(|s| s.to_str()),
                    Some("md") | Some("toml")
                )
            {
                out.push(skill_summary(vertical.to_string(), &path));
            }
        }
    }
    out.sort_by(|a, b| a.vertical.cmp(&b.vertical).then_with(|| a.id.cmp(&b.id)));
    Ok(out)
}

pub(crate) fn manifest_action_type(id: &str) -> String {
    id.to_ascii_lowercase().replace(['.', '-'], "_")
}

fn push_connector(
    out: &mut Vec<SuggestedAction>,
    role_tokens: &BTreeSet<String>,
    connector: &ConnectorSummary,
) {
    let text = format!(
        "{} {} {}",
        connector.id,
        connector.tools.join(" "),
        connector_keywords(&connector.id)
    );
    let (score, matched) = score_text(role_tokens, &text);
    if score > 0.0 {
        out.push(SuggestedAction {
            id: connector
                .tools
                .first()
                .cloned()
                .unwrap_or_else(|| connector.id.clone()),
            source: ActionSource::Connector(connector.id.clone()),
            score,
            reason: reason("connector", &matched),
        });
    }
}

fn push_skill(
    out: &mut Vec<SuggestedAction>,
    role_tokens: &BTreeSet<String>,
    skill: &SkillSummary,
) {
    let text = format!("{} {}", skill.id, skill.vertical);
    let (score, matched) = score_text(role_tokens, &text);
    if score > 0.0 {
        out.push(SuggestedAction {
            id: skill.id.clone(),
            source: ActionSource::Skill(skill.id.clone()),
            score,
            reason: reason("skill", &matched),
        });
    }
}

fn skill_summary(vertical: String, path: &Path) -> SkillSummary {
    let id = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("skill")
        .trim_end_matches(".skill")
        .to_string();
    SkillSummary { id, vertical }
}

fn score_text(role_tokens: &BTreeSet<String>, text: &str) -> (f32, Vec<String>) {
    let text_tokens = tokenize(text);
    let matched: Vec<String> = role_tokens.intersection(&text_tokens).cloned().collect();
    if matched.is_empty() {
        (0.0, matched)
    } else {
        (
            (matched.len() as f32 / role_tokens.len().max(1) as f32).min(1.0),
            matched,
        )
    }
}

fn add_generic_fallbacks(out: &mut Vec<SuggestedAction>, top_k: usize) {
    for (id, reason) in [
        ("web_search", "generic discovery action for unknown roles"),
        (
            "scratchpad_note",
            "generic working-memory action for unknown roles",
        ),
        (
            "operator_handoff",
            "generic escalation action for unknown roles",
        ),
    ] {
        if out.len() >= top_k.min(3) {
            return;
        }
        out.push(SuggestedAction {
            id: id.to_string(),
            source: ActionSource::Skill("generic".to_string()),
            score: 0.01,
            reason: reason.to_string(),
        });
    }
}

fn compare_candidates(a: &SuggestedAction, b: &SuggestedAction) -> Ordering {
    b.score
        .partial_cmp(&a.score)
        .unwrap_or(Ordering::Equal)
        .then_with(|| a.id.cmp(&b.id))
}

fn tokenize(text: &str) -> BTreeSet<String> {
    text.split(|c: char| !c.is_ascii_alphanumeric())
        .filter_map(|raw| {
            let token = raw.trim().to_ascii_lowercase();
            (token.len() >= 2).then(|| stem(token))
        })
        .collect()
}

fn stem(mut token: String) -> String {
    if token.len() > 4 && token.ends_with("ies") {
        token.truncate(token.len() - 3);
        token.push('y');
    } else if token.len() > 3 && token.ends_with('s') {
        token.pop();
    }
    token
}

fn connector_keywords(id: &str) -> &'static str {
    match id {
        "gmail" => "email mail inbox message draft send outbound prospect customer reply",
        "hubspot" => "crm sales sdr lead prospect contact deal pipeline outbound account",
        "stripe" | "stripe-events" => {
            "invoice payment charge refund customer finance bookkeeping ar clerk reconcile"
        }
        "slack" => "chat message channel team incident status post update notify",
        "gcal" => "calendar schedule meeting invite availability time coordinator",
        "notion" => "docs wiki database page knowledge note policy",
        "zendesk" => "support ticket customer triage helpdesk reply",
        _ => "",
    }
}

fn connector_tools(id: &str) -> Vec<String> {
    let tool = match id {
        "gmail" => "gmail.search",
        "gcal" => "gcal.list_events",
        "hubspot" => "hubspot.search_contacts",
        "slack" => "slack.search",
        "notion" => "notion.search",
        "stripe" => "stripe.list_invoices",
        "zendesk" => "zendesk.search_tickets",
        _ => return Vec::new(),
    };
    vec![tool.to_string()]
}

fn reason(kind: &str, matched: &[String]) -> String {
    format!("matched {kind} keywords: {}", matched.join(", "))
}

fn toml_package_str(value: &toml::Value, key: &str) -> String {
    value
        .get("package")
        .and_then(toml::Value::as_table)
        .and_then(|p| p.get(key))
        .and_then(toml::Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn workspace_root() -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(Path::parent)
        .unwrap_or(manifest)
        .to_path_buf()
}

fn child_paths(path: &Path, what: &'static str) -> Result<Vec<PathBuf>, GenesisError> {
    std::fs::read_dir(path)
        .map_err(|source| GenesisError::Io { what, source })?
        .map(|entry| {
            entry
                .map(|e| e.path())
                .map_err(|source| GenesisError::Io { what, source })
        })
        .collect()
}

fn read_to_string(path: &Path, what: &'static str) -> Result<String, GenesisError> {
    std::fs::read_to_string(path).map_err(|source| GenesisError::Io { what, source })
}

#[cfg(test)]
#[path = "action_suggest_tests.rs"]
mod tests;
