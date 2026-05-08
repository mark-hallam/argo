//! Conflict detection for generated Genesis verticals.

use crate::pipeline::GenesisPlan;
use argo_types::{ArgoError, ArgoResult};
use argo_vertical::VerticalManifest;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const ROLE_WEIGHT: f32 = 0.5;
const ACTION_WEIGHT: f32 = 0.3;
const MEMORY_WEIGHT: f32 = 0.2;
const GENERATED_MEMORY_NAMESPACES: &[&str] = &["working", "episodic", "skills"];

/// Report of existing verticals that overlap a generated plan.
#[derive(Debug, Clone, PartialEq)]
pub struct ConflictReport {
    /// Candidate matches, sorted by descending score.
    pub matches: Vec<ConflictMatch>,
}

/// One existing vertical that shares meaningful role, action, or memory shape.
#[derive(Debug, Clone, PartialEq)]
pub struct ConflictMatch {
    /// Existing vertical id.
    pub existing_vertical: String,
    /// Weighted overlap score in the inclusive range [0.0, 1.0].
    pub score: f32,
    /// Plain-English signals that contributed to the score.
    pub reasons: Vec<String>,
}

struct ExistingVertical {
    id: String,
    role_words: BTreeSet<String>,
    actions: BTreeSet<String>,
    memory_namespaces: BTreeSet<String>,
}

/// Detect overlap between a generated plan and `verticals/*/vertical.toml`.
pub fn detect(generated: &GenesisPlan, verticals_root: &Path) -> ArgoResult<ConflictReport> {
    if !verticals_root.exists() {
        return Err(ArgoError::Io(format!(
            "verticals root not found: {}",
            verticals_root.display()
        )));
    }
    let generated_role = role_words(&[
        &generated.charter,
        &generated.manifest.vertical.id,
        &generated.manifest.vertical.name,
        &generated.manifest.vertical.description,
    ]);
    let generated_actions = action_names(&generated.manifest);
    let generated_memory = GENERATED_MEMORY_NAMESPACES
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let mut matches = Vec::new();
    for path in manifest_paths(verticals_root)? {
        let existing = read_existing(&path)?;
        let role = jaccard(&generated_role, &existing.role_words);
        let actions = jaccard(&generated_actions, &existing.actions);
        let memory = jaccard(&generated_memory, &existing.memory_namespaces);
        let score = ((ROLE_WEIGHT * role) + (ACTION_WEIGHT * actions) + (MEMORY_WEIGHT * memory))
            .clamp(0.0, 1.0);
        if score <= 0.0 {
            continue;
        }
        matches.push(ConflictMatch {
            existing_vertical: existing.id,
            score,
            reasons: reasons(role, actions, memory),
        });
    }
    matches.sort_by(|a, b| b.score.total_cmp(&a.score));
    Ok(ConflictReport { matches })
}

/// Return the highest conflict score in a report.
#[must_use]
pub fn highest_score(report: &ConflictReport) -> Option<f32> {
    report.matches.first().map(|m| m.score)
}

fn manifest_paths(root: &Path) -> ArgoResult<Vec<PathBuf>> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            let path = entry.path().join("vertical.toml");
            if path.exists() {
                out.push(path);
            }
        }
    }
    out.sort();
    Ok(out)
}

fn read_existing(path: &Path) -> ArgoResult<ExistingVertical> {
    let body = std::fs::read_to_string(path)?;
    let manifest: VerticalManifest = toml::from_str(&body)
        .map_err(|e| ArgoError::Schema(format!("{} did not parse: {e}", path.display())))?;
    let role_words = role_words(&[
        &manifest.vertical.id,
        &manifest.vertical.name,
        &manifest.vertical.description,
        &manifest.vertical.tags.join(" "),
    ]);
    let memory_namespaces = read_memory_namespaces(path);
    Ok(ExistingVertical {
        id: manifest.vertical.id.clone(),
        role_words,
        actions: action_names(&manifest),
        memory_namespaces,
    })
}

fn action_names(manifest: &VerticalManifest) -> BTreeSet<String> {
    manifest
        .actions
        .as_ref()
        .map(|actions| actions.actions.keys().map(|s| normalise_token(s)).collect())
        .unwrap_or_default()
}

fn read_memory_namespaces(manifest_path: &Path) -> BTreeSet<String> {
    let Some(dir) = manifest_path.parent() else {
        return BTreeSet::new();
    };
    let Ok(body) = std::fs::read_to_string(dir.join("memory/schema.toml")) else {
        return BTreeSet::new();
    };
    let Ok(value) = body.parse::<toml::Value>() else {
        return BTreeSet::new();
    };
    let mut out = BTreeSet::new();
    if let Some(namespaces) = value.get("namespaces").and_then(toml::Value::as_table) {
        out.extend(namespaces.keys().map(|s| normalise_token(s)));
    }
    if let Some(scopes) = value.get("scope").and_then(toml::Value::as_array) {
        for scope in scopes {
            if let Some(id) = scope.get("id").and_then(toml::Value::as_str) {
                out.insert(normalise_token(id));
            }
        }
    }
    out
}

fn role_words(parts: &[&str]) -> BTreeSet<String> {
    parts
        .iter()
        .flat_map(|part| part.split_whitespace())
        .map(normalise_token)
        .filter(|word| word.len() >= 3 && !is_stopword(word))
        .collect()
}

fn normalise_token(raw: &str) -> String {
    raw.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_stopword(word: &str) -> bool {
    matches!(
        word,
        "the" | "and" | "for" | "with" | "that" | "this" | "role" | "agent" | "from" | "into"
    )
}

fn jaccard(a: &BTreeSet<String>, b: &BTreeSet<String>) -> f32 {
    if a.is_empty() && b.is_empty() {
        return 0.0;
    }
    let intersection = a.intersection(b).count() as f32;
    let union = a.union(b).count() as f32;
    intersection / union
}

fn reasons(role: f32, actions: f32, memory: f32) -> Vec<String> {
    let mut out = Vec::new();
    if role > 0.0 {
        out.push(format!("{:.0}% role keyword overlap", role * 100.0));
    }
    if actions > 0.0 {
        out.push(format!("{:.0}% action vocabulary overlap", actions * 100.0));
    }
    if memory > 0.0 {
        out.push(format!("{:.0}% memory namespace overlap", memory * 100.0));
    }
    out
}

#[cfg(test)]
#[path = "conflict_tests.rs"]
mod tests;
