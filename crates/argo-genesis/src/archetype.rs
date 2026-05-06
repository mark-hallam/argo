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
