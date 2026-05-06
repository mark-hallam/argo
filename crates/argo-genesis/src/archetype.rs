//! 12-archetype taxonomy + offline classifier. Each archetype
//! is a "starting kit" of action vocabulary, tone, and KPIs
//! that Genesis customises for the operator's charter. The
//! classifier picks one archetype per charter; the generator
//! pipeline uses the archetype's seed vocab + system prompt.
//!
//! v1 ships an offline keyword classifier — fast, deterministic,
//! testable without an API key.

use crate::error::GenesisError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Archetype {
    SalesSdr,
    CustomerSupportAgent,
    MarketingIntern,
    RecruiterScreener,
    OpsOnCall,
    ResearchAnalyst,
    CalendarCoordinator,
    SocialMediaDrafter,
    LeadResearcher,
    BookkeepingAssistant,
    ProjectPm,
    GtmEngineer,
}

impl Archetype {
    pub const ALL: &'static [Self] = &[
        Self::SalesSdr,
        Self::CustomerSupportAgent,
        Self::MarketingIntern,
        Self::RecruiterScreener,
        Self::OpsOnCall,
        Self::ResearchAnalyst,
        Self::CalendarCoordinator,
        Self::SocialMediaDrafter,
        Self::LeadResearcher,
        Self::BookkeepingAssistant,
        Self::ProjectPm,
        Self::GtmEngineer,
    ];

    /// Stable slug used in vertical id + manifest defaults.
    #[must_use]
    pub fn slug(self) -> &'static str {
        match self {
            Self::SalesSdr => "sales_sdr",
            Self::CustomerSupportAgent => "customer_support_agent",
            Self::MarketingIntern => "marketing_intern",
            Self::RecruiterScreener => "recruiter_screener",
            Self::OpsOnCall => "ops_on_call",
            Self::ResearchAnalyst => "research_analyst",
            Self::CalendarCoordinator => "calendar_coordinator",
            Self::SocialMediaDrafter => "social_media_drafter",
            Self::LeadResearcher => "lead_researcher",
            Self::BookkeepingAssistant => "bookkeeping_assistant",
            Self::ProjectPm => "project_pm",
            Self::GtmEngineer => "gtm_engineer",
        }
    }

    /// Human-readable label.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::SalesSdr => "Sales SDR",
            Self::CustomerSupportAgent => "Customer Support Agent",
            Self::MarketingIntern => "Marketing Intern",
            Self::RecruiterScreener => "Recruiter Screener",
            Self::OpsOnCall => "Ops On-Call",
            Self::ResearchAnalyst => "Research Analyst",
            Self::CalendarCoordinator => "Calendar Coordinator",
            Self::SocialMediaDrafter => "Social Media Drafter",
            Self::LeadResearcher => "Lead Researcher",
            Self::BookkeepingAssistant => "Bookkeeping Assistant",
            Self::ProjectPm => "Project PM",
            Self::GtmEngineer => "GTM Engineer",
        }
    }

    #[must_use]
    pub fn charter_keywords(self) -> &'static [&'static str] {
        match self {
            Self::SalesSdr => &[
                "sales", "sdr", "outbound", "prospect", "qualify", "cold call",
            ],
            Self::CustomerSupportAgent => &[
                "support", "ticket", "customer", "triage", "help desk", "respond",
            ],
            Self::MarketingIntern => &[
                "marketing", "campaign", "content", "social", "newsletter", "intern",
                "leads",
            ],
            Self::RecruiterScreener => &[
                "recruit", "screen", "candidate", "interview", "hiring", "talent",
            ],
            Self::OpsOnCall => &[
                "oncall", "on-call", "incident", "ops", "alert", "page", "runbook",
            ],
            Self::ResearchAnalyst => &[
                "research", "analyst", "report", "market", "compet", "analysis",
            ],
            Self::CalendarCoordinator => &[
                "calendar", "schedule", "meeting", "book", "coordinate", "availability",
            ],
            Self::SocialMediaDrafter => &[
                "social media", "twitter", "linkedin", "post", "draft", "thread",
            ],
            Self::LeadResearcher => &[
                "lead", "list", "icp", "prospect research", "enrich",
            ],
            Self::BookkeepingAssistant => &[
                "bookkeeping", "accounting", "invoice", "expense", "ledger", "reconcile",
            ],
            Self::ProjectPm => &[
                "project", "pm", "milestone", "sprint", "kanban", "manage",
            ],
            Self::GtmEngineer => &[
                "gtm", "go-to-market", "growth", "automation", "marketplace", "listing",
            ],
        }
    }
}

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

/// Classify a charter into one of the 12 archetypes via offline
/// keyword scoring. Ties broken by declaration order.
///
/// # Example
/// ```
/// use argo_genesis::Archetype;
/// let arch = argo_genesis::classify_charter(
///     "be my marketing intern, run weekly campaigns to cold leads"
/// ).unwrap();
/// assert_eq!(arch, Archetype::MarketingIntern);
/// ```
pub fn classify_charter(charter: &str) -> Result<Archetype, GenesisError> {
    if charter.trim().is_empty() {
        return Err(GenesisError::EmptyCharter);
    }
    let lowered = charter.to_lowercase();
    let mut best: Option<(Archetype, usize)> = None;
    for arch in Archetype::ALL {
        let score = arch
            .charter_keywords()
            .iter()
            .map(|kw| count_occurrences(&lowered, &kw.to_lowercase()))
            .sum::<usize>();
        if score > 0 {
            match best {
                Some((_, prev_score)) if prev_score >= score => {}
                _ => best = Some((*arch, score)),
            }
        }
    }
    best.map(|(a, _)| a).ok_or_else(|| GenesisError::UnknownArchetype {
        available: Archetype::ALL
            .iter()
            .map(|a| a.label())
            .collect::<Vec<_>>()
            .join(", "),
    })
}

fn count_occurrences(haystack: &str, needle: &str) -> usize {
    if needle.is_empty() {
        return 0;
    }
    haystack.matches(needle).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marketing_intern_charter_classifies_correctly() {
        let a = classify_charter(
            "be my marketing intern, run weekly campaigns to cold leads",
        )
        .unwrap();
        assert_eq!(a, Archetype::MarketingIntern);
    }

    #[test]
    fn ops_oncall_charter_classifies_correctly() {
        let a = classify_charter(
            "you're my oncall ops engineer, page me when an incident fires",
        )
        .unwrap();
        assert_eq!(a, Archetype::OpsOnCall);
    }

    #[test]
    fn empty_charter_errors() {
        let err = classify_charter("   ").expect_err("empty charter");
        assert!(matches!(err, GenesisError::EmptyCharter));
    }

    #[test]
    fn unrelated_charter_errors_with_archetype_list() {
        let err = classify_charter("xyzzy frobnicate the bazquux")
            .expect_err("nonsense charter");
        let msg = err.to_string();
        assert!(msg.contains("Marketing Intern"), "got: {msg}");
    }

    #[test]
    fn each_archetype_has_a_unique_slug() {
        let mut seen = std::collections::HashSet::new();
        for a in Archetype::ALL {
            assert!(seen.insert(a.slug()), "duplicate slug: {}", a.slug());
        }
    }
}
