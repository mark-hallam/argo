//! Archetype metadata + listing surface.
//!
//! Extracted from `archetype.rs` to keep that file under the
//! 500-LOC ceiling. The metadata layer turns each [`Archetype`]
//! into an [`ArchetypeMeta`] with seed actions, persona traits,
//! recommended channels, and keyword vocab; the CLI's
//! `genesis list-archetypes` / `show-archetype` commands and the
//! setup wizard consume it via [`list_archetypes`] /
//! [`show_archetype`].
//!
//! Public surface is re-exported from [`crate::archetype`] so
//! `argo_genesis::archetype::list_archetypes` etc. continue to work
//! for existing callers.

use crate::archetype::Archetype;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchetypeActionMeta {
    pub name: String,
    pub risk: String,
    pub requires_approval: bool,
    pub description: String,
    pub sensitive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchetypeMeta {
    pub name: String,
    pub slug: String,
    pub description: String,
    pub sensitive_verb_count: usize,
    pub base_action_count: usize,
    pub seed_actions: Vec<ArchetypeActionMeta>,
    pub default_persona_traits: Vec<String>,
    pub recommended_channels: Vec<String>,
    pub keywords: Vec<String>,
}

/// Return every built-in Genesis archetype in stable label order.
#[must_use]
pub fn list_archetypes() -> Vec<ArchetypeMeta> {
    let mut out: Vec<_> = Archetype::ALL.iter().copied().map(meta_for).collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Lookup an archetype by label or slug. Matching is case-insensitive
/// and treats spaces, hyphens, and underscores as equivalent.
#[must_use]
pub fn show_archetype(name: &str) -> Option<ArchetypeMeta> {
    let needle = normalise_name(name);
    list_archetypes()
        .into_iter()
        .find(|m| normalise_name(&m.name) == needle || normalise_name(&m.slug) == needle)
}

fn meta_for(archetype: Archetype) -> ArchetypeMeta {
    let seed_actions = seed_actions_for(archetype);
    let sensitive_verb_count = seed_actions.iter().filter(|a| a.sensitive).count();
    ArchetypeMeta {
        name: archetype.label().to_string(),
        slug: archetype.slug().to_string(),
        description: archetype_description(archetype).to_string(),
        base_action_count: seed_actions.len(),
        sensitive_verb_count,
        seed_actions,
        default_persona_traits: vec![
            "professional".to_string(),
            "warm".to_string(),
            "concise".to_string(),
            "escalates outside charter".to_string(),
        ],
        recommended_channels: recommended_channels(archetype),
        keywords: archetype
            .charter_keywords()
            .iter()
            .map(|s| (*s).to_string())
            .collect(),
    }
}

fn seed_actions_for(archetype: Archetype) -> Vec<ArchetypeActionMeta> {
    let manifest = match crate::manifest_writer::build_manifest(
        "operator discovery",
        archetype,
        "operator",
        archetype.slug(),
    ) {
        Ok(manifest) => manifest,
        Err(_) => return Vec::new(),
    };
    let Some(actions) = manifest.actions else {
        return Vec::new();
    };
    actions
        .actions
        .into_iter()
        .map(|(name, decl)| {
            let sensitive = crate::safety_judge::matches_sensitive_verb(&name);
            ArchetypeActionMeta {
                name,
                risk: risk_label(decl.risk).to_string(),
                requires_approval: decl.requires_approval,
                description: decl.description,
                sensitive,
            }
        })
        .collect()
}

fn risk_label(risk: argo_vertical::ActionRiskLevel) -> &'static str {
    match risk {
        argo_vertical::ActionRiskLevel::Low => "low",
        argo_vertical::ActionRiskLevel::Medium => "medium",
        argo_vertical::ActionRiskLevel::High => "high",
    }
}

fn archetype_description(archetype: Archetype) -> &'static str {
    match archetype {
        Archetype::SalesSdr => "Outbound sales worker for prospect research, email drafts, and operator-approved sends.",
        Archetype::CustomerSupportAgent => "Support worker for ticket triage, response drafting, and human-approved customer replies.",
        Archetype::MarketingIntern => "Marketing worker for research, campaign drafting, and approval-gated publishing.",
        Archetype::RecruiterScreener => "Recruiting worker for resume review, screening questions, candidate messages, and interviews.",
        Archetype::OpsOnCall => "Operations worker for incident acknowledgement, status drafting, and escalation handoffs.",
        Archetype::ResearchAnalyst => "Research worker for source collection, summaries, and approval-gated sharing.",
        Archetype::CalendarCoordinator => "Scheduling worker for availability coordination, invite drafts, and approved calendar changes.",
        Archetype::SocialMediaDrafter => "Social worker for post drafting, publishing review, and delete safeguards.",
        Archetype::LeadResearcher => "Prospecting worker for lead enrichment, scoring, and approved list sharing.",
        Archetype::BookkeepingAssistant => "Finance helper for expense classification, invoice drafts, and approved payment actions.",
        Archetype::ProjectPm => "Project worker for status updates, standup drafts, and approved team posts.",
        Archetype::GtmEngineer => "Go-to-market worker for market research, marketplace listings, and follow-up drafts.",
    }
}

fn recommended_channels(archetype: Archetype) -> Vec<String> {
    let actions = seed_actions_for(archetype);
    let mut channels = BTreeSet::new();
    channels.insert("webinbox".to_string());
    for action in actions {
        let text = format!("{} {}", action.name, action.description).to_lowercase();
        if text.contains("email") || text.contains("invoice") {
            channels.insert("email".to_string());
        }
        if text.contains("post") || text.contains("status") || text.contains("incident") {
            channels.insert("slack".to_string());
        }
        if text.contains("page") || text.contains("schedule") {
            channels.insert("sms".to_string());
        }
    }
    channels.into_iter().collect()
}

fn normalise_name(raw: &str) -> String {
    raw.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}
