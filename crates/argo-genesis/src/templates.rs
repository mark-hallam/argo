//! Built-in Genesis vertical templates.

use crate::archetype::Archetype;
use argo_types::{ArgoError, ArgoResult};
use include_dir::{include_dir, Dir};
use serde::Deserialize;
use std::collections::BTreeSet;

static TEMPLATE_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates");

/// Template metadata used for matching and display.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct TemplateMeta {
    /// Stable template id.
    pub id: String,
    /// Human-readable template name.
    pub name: String,
    /// Existing Genesis archetype slug used by the pipeline.
    pub archetype: String,
    /// Role-description keywords that select this template.
    pub match_keywords: Vec<String>,
}

/// One starter eval case bundled with a template.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct TemplateSeed {
    /// Stable seed id.
    pub id: String,
    /// Task prompt for the generated worker.
    pub task: String,
    /// Expected high-level output.
    pub expected_summary: String,
    /// Difficulty label.
    pub difficulty: String,
    /// Lowercase rubric keywords.
    pub rubric_keywords: Vec<String>,
}

/// Parsed vertical template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerticalTemplate {
    /// Template metadata.
    pub meta: TemplateMeta,
    /// System prompt skeleton body.
    pub prompt_skeleton: String,
    /// Suggested action names.
    pub actions: Vec<String>,
    /// Memory layer names.
    pub memory_layers: Vec<String>,
    /// Starter eval seeds.
    pub eval_seeds: Vec<TemplateSeed>,
}

/// Merged template output before Genesis writes a vertical.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedVertical {
    /// Prompt skeleton after applying operator override hints.
    pub prompt_skeleton: String,
    /// Suggested action names.
    pub actions: Vec<String>,
    /// Memory layer names.
    pub memory_layers: Vec<String>,
    /// Starter eval seeds.
    pub eval_seeds: Vec<TemplateSeed>,
}

#[derive(Debug, Deserialize)]
struct TemplateDoc {
    meta: TemplateMeta,
    prompt_skeleton: PromptSkeleton,
    actions: Vec<TemplateAction>,
    memory_layers: MemoryLayers,
    eval_seeds: Vec<TemplateSeed>,
}

#[derive(Debug, Deserialize)]
struct PromptSkeleton {
    body: String,
}

#[derive(Debug, Deserialize)]
struct TemplateAction {
    name: String,
}

#[derive(Debug, Deserialize)]
struct MemoryLayers {
    names: Vec<String>,
}

/// Load every bundled Genesis template.
pub fn load_all() -> ArgoResult<Vec<VerticalTemplate>> {
    let mut templates = Vec::new();
    for file in TEMPLATE_DIR.files() {
        if file.path().extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        let body = file.contents_utf8().ok_or_else(|| {
            ArgoError::Schema(format!("template {} is not utf-8", file.path().display()))
        })?;
        let parsed: TemplateDoc = toml::from_str(body).map_err(|e| {
            ArgoError::Schema(format!(
                "template {} did not parse: {e}",
                file.path().display()
            ))
        })?;
        let template = VerticalTemplate {
            meta: parsed.meta,
            prompt_skeleton: parsed.prompt_skeleton.body.trim().to_string(),
            actions: parsed.actions.into_iter().map(|a| a.name).collect(),
            memory_layers: parsed.memory_layers.names,
            eval_seeds: parsed.eval_seeds,
        };
        validate_template(&template)?;
        templates.push(template);
    }
    templates.sort_by(|a, b| a.meta.id.cmp(&b.meta.id));
    Ok(templates)
}

/// Match a role description to the first template with at least two
/// keyword hits.
#[must_use]
pub fn match_template(role_description: &str) -> Option<VerticalTemplate> {
    let role = normalise(role_description);
    load_all().ok()?.into_iter().find(|template| {
        template
            .meta
            .match_keywords
            .iter()
            .filter(|keyword| contains_keyword(&role, keyword))
            .take(2)
            .count()
            >= 2
    })
}

/// Merge a template with explicit override hints from the role
/// description.
pub fn merge_with_overrides(
    template: &VerticalTemplate,
    role_description: &str,
) -> ArgoResult<GeneratedVertical> {
    validate_template(template)?;
    let mut prompt_skeleton = template.prompt_skeleton.clone();
    if let Some(hint) = override_hint(role_description) {
        prompt_skeleton.push_str("\n\n## Operator override\n");
        prompt_skeleton.push_str(hint);
    }
    Ok(GeneratedVertical {
        prompt_skeleton,
        actions: template.actions.clone(),
        memory_layers: template.memory_layers.clone(),
        eval_seeds: template.eval_seeds.clone(),
    })
}

impl VerticalTemplate {
    /// Return the existing Genesis archetype mapped by this template.
    #[must_use]
    pub fn archetype(&self) -> Option<Archetype> {
        archetype_from_slug(&self.meta.archetype)
    }
}

fn validate_template(template: &VerticalTemplate) -> ArgoResult<()> {
    let mut missing = Vec::new();
    if template.meta.id.trim().is_empty() {
        missing.push("meta.id");
    }
    if template.prompt_skeleton.trim().is_empty() {
        missing.push("prompt_skeleton");
    }
    if template.actions.is_empty() {
        missing.push("actions");
    }
    if template.memory_layers.is_empty() {
        missing.push("memory_layers");
    }
    if template.eval_seeds.is_empty() {
        missing.push("eval_seeds");
    }
    if missing.is_empty() {
        return Ok(());
    }
    Err(ArgoError::Schema(format!(
        "template {} missing {}",
        template.meta.id,
        missing.join(", ")
    )))
}

fn contains_keyword(role: &str, keyword: &str) -> bool {
    let keyword = normalise(keyword);
    if keyword.is_empty() {
        return false;
    }
    let role_words: BTreeSet<_> = role.split_whitespace().collect();
    let keyword_words: Vec<_> = keyword.split_whitespace().collect();
    keyword_words.iter().all(|word| role_words.contains(word))
}

fn normalise(raw: &str) -> String {
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

fn override_hint(role_description: &str) -> Option<&str> {
    role_description
        .split_once("override:")
        .map(|(_, hint)| hint.trim())
        .filter(|hint| !hint.is_empty())
}

fn archetype_from_slug(slug: &str) -> Option<Archetype> {
    match slug {
        "sales_sdr" => Some(Archetype::SalesSdr),
        "customer_support_agent" => Some(Archetype::CustomerSupportAgent),
        "marketing_intern" => Some(Archetype::MarketingIntern),
        "bookkeeping_assistant" => Some(Archetype::BookkeepingAssistant),
        "gtm_engineer" => Some(Archetype::GtmEngineer),
        _ => None,
    }
}

#[cfg(test)]
#[path = "templates_tests.rs"]
mod tests;
