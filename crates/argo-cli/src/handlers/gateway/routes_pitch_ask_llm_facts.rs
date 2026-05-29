//! Operator-reviewed company-facts loading for the Pitch Agent LLM
//! path. Extracted from [`super::routes_pitch_ask_llm`] to keep that
//! module under the 500-LOC ceiling. The facts are the only grounding
//! the LLM is permitted to draw on (see `render_system_prompt`).

use argo_types::{ArgoError, ArgoResult};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::OnceLock;

/// Operator-reviewed company facts loaded from
/// `packs/pitch-agent/facts.toml`.
#[derive(Debug, Clone, serde::Deserialize)]
pub(super) struct CompanyFacts {
    #[serde(default)]
    pub company: toml::Table,
    #[serde(default)]
    pub product: toml::Table,
    #[serde(default)]
    pub architecture: toml::Table,
    #[serde(default)]
    pub differentiation: toml::Table,
    #[serde(default)]
    pub deployment: toml::Table,
    #[serde(default)]
    pub fundraising_stance: toml::Table,
}

impl CompanyFacts {
    /// Render every section to Markdown — one `## Section` heading per
    /// table, one `- key: value` bullet per fact. Tests pin the shape.
    pub(super) fn as_markdown(&self) -> String {
        let mut out = String::new();
        for (heading, table) in [
            ("Company", &self.company),
            ("Product", &self.product),
            ("Architecture", &self.architecture),
            ("Differentiation", &self.differentiation),
            ("Deployment", &self.deployment),
            ("Fundraising stance", &self.fundraising_stance),
        ] {
            if table.is_empty() {
                continue;
            }
            out.push_str(&format!("## {heading}\n\n"));
            for (k, v) in table {
                let value_str = match v {
                    toml::Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                out.push_str(&format!("- {k}: {value_str}\n"));
            }
            out.push('\n');
        }
        out
    }

    /// Load the operator-reviewed facts from disk, with caching. Errors
    /// surface as `ArgoError::Schema` so the gateway returns 503.
    pub(super) fn load_cached() -> ArgoResult<Arc<CompanyFacts>> {
        let cell = FACTS.get_or_init(load_facts_from_disk);
        cell.as_ref().map(Arc::clone).map_err(|e| ArgoError::Schema(e.clone()))
    }
}

static FACTS: OnceLock<Result<Arc<CompanyFacts>, String>> = OnceLock::new();

fn load_facts_from_disk() -> Result<Arc<CompanyFacts>, String> {
    let path = facts_path();
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| format!("read {}: {e}", path.display()))?;
    let parsed: CompanyFacts =
        toml::from_str(&raw).map_err(|e| format!("parse {}: {e}", path.display()))?;
    Ok(Arc::new(parsed))
}

fn facts_path() -> PathBuf {
    if let Ok(p) = std::env::var("ARGO_PITCH_FACTS_PATH") {
        if !p.trim().is_empty() {
            return PathBuf::from(p);
        }
    }
    workspace_root().join("packs/pitch-agent/facts.toml")
}

fn workspace_root() -> PathBuf {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut cursor: &Path = &cwd;
    loop {
        if cursor.join("Cargo.toml").exists() && cursor.join("crates").is_dir() {
            return cursor.to_path_buf();
        }
        match cursor.parent() {
            Some(parent) => cursor = parent,
            None => return cwd,
        }
    }
}
