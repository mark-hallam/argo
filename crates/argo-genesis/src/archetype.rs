//! 12-archetype taxonomy + offline classifier. Each archetype
//! is a "starting kit" of action vocabulary, tone, and KPIs
//! that Genesis customises for the operator's charter. The
//! classifier picks one archetype per charter; the generator
//! pipeline uses the archetype's seed vocab + system prompt.
//!
//! Two classifiers coexist:
//! - `classify_charter` — offline keyword scorer. Fast, deterministic,
//!   no API key. Always available; doubles as the safety net under
//!   the LLM path.
//! - `classify_llm` — model-driven via the narrow [`ProviderClient`]
//!   trait below. Falls back to the keyword scorer on any error
//!   (network, parse, empty response, unknown slug) so a transient
//!   model hiccup never breaks Genesis.

use crate::error::GenesisError;
use argo_types::ArgoResult;
use serde::{Deserialize, Serialize};

pub use crate::archetype_meta::{
    list_archetypes, show_archetype, ArchetypeActionMeta, ArchetypeMeta,
};

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

    /// Lookup an archetype by its canonical snake_case slug.
    /// Tolerates leading/trailing whitespace, uppercase, and `-` in
    /// place of `_` so model output like `"Sales-SDR"` still resolves.
    #[must_use]
    pub fn from_slug(slug: &str) -> Option<Self> {
        let canon = slug.trim().to_lowercase().replace('-', "_");
        Self::ALL.iter().copied().find(|a| a.slug() == canon)
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
                "sales",
                "sdr",
                "outbound",
                "prospect",
                "qualify",
                "cold call",
            ],
            Self::CustomerSupportAgent => &[
                "support",
                "ticket",
                "customer",
                "triage",
                "help desk",
                "respond",
            ],
            Self::MarketingIntern => &[
                "marketing",
                "campaign",
                "content",
                "social",
                "newsletter",
                "intern",
                "leads",
            ],
            Self::RecruiterScreener => &[
                "recruit",
                "screen",
                "candidate",
                "interview",
                "hiring",
                "talent",
            ],
            Self::OpsOnCall => &[
                "oncall", "on-call", "incident", "ops", "alert", "page", "runbook",
            ],
            Self::ResearchAnalyst => &[
                "research", "analyst", "report", "market", "compet", "analysis",
            ],
            Self::CalendarCoordinator => &[
                "calendar",
                "schedule",
                "meeting",
                "book",
                "coordinate",
                "availability",
            ],
            Self::SocialMediaDrafter => &[
                "social media",
                "twitter",
                "linkedin",
                "post",
                "draft",
                "thread",
            ],
            Self::LeadResearcher => &["lead", "list", "icp", "prospect research", "enrich"],
            Self::BookkeepingAssistant => &[
                "bookkeeping",
                "accounting",
                "invoice",
                "expense",
                "ledger",
                "reconcile",
            ],
            Self::ProjectPm => &["project", "pm", "milestone", "sprint", "kanban", "manage"],
            Self::GtmEngineer => &[
                "gtm",
                "go-to-market",
                "growth",
                "automation",
                "marketplace",
                "listing",
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
    best.map(|(a, _)| a)
        .ok_or_else(|| GenesisError::UnknownArchetype {
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

/// Narrow provider abstraction used by [`classify_llm`]. Kept in this
/// crate so `argo-genesis` does not have to depend on the much larger
/// `argo-provider` crate just to call one model. A real impl (e.g.
/// adapting `argo-provider`'s router) ships in a follow-up brief; for
/// now, tests inject a mock and production callers can wrap whatever
/// client they already hold.
#[async_trait::async_trait]
pub trait ProviderClient: Send + Sync {
    /// Send a one-shot classification prompt and return the raw model
    /// response text. Implementations MUST set temperature to 0 (or
    /// the model's nearest deterministic equivalent) so a charter
    /// classifies the same way on every call.
    async fn classify(&self, prompt: &str) -> ArgoResult<String>;
}

/// Build the deterministic classifier prompt. Lists the 12 valid
/// slugs and asks for exactly one back. The prompt is intentionally
/// small so it stays under any reasonable token cap.
fn build_classifier_prompt(charter: &str) -> String {
    let slugs = Archetype::ALL
        .iter()
        .map(|a| a.slug())
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "You are an archetype classifier for a worker-agent charter. \
         Respond with EXACTLY ONE slug from this list and nothing else \
         (no punctuation, no prose, no quotes): {slugs}.\n\nCharter: {charter}",
    )
}

/// Classify a charter into one of the 12 archetypes via the LLM.
/// Falls back to [`classify_charter`] on **any** failure path —
/// network/provider error, empty response, or a slug the model
/// returned that doesn't match a known archetype. The fallback is
/// load-bearing: a transient model hiccup must never break Genesis.
///
/// Returns `Err` only if the keyword fallback itself errors (e.g.
/// the charter is empty), so callers may treat this as a strict
/// upgrade over `classify_charter`.
pub async fn classify_llm(charter: &str, llm: &dyn ProviderClient) -> ArgoResult<Archetype> {
    if charter.trim().is_empty() {
        return Err(GenesisError::EmptyCharter.into());
    }
    let prompt = build_classifier_prompt(charter);
    match llm.classify(&prompt).await {
        Ok(raw) => match Archetype::from_slug(&raw) {
            Some(a) => Ok(a),
            None => fallback_to_keyword(charter),
        },
        Err(_) => fallback_to_keyword(charter),
    }
}

fn fallback_to_keyword(charter: &str) -> ArgoResult<Archetype> {
    classify_charter(charter).map_err(Into::into)
}

/// Result of LLM-driven archetype classification with a confidence
/// signal. `archetype` is `None` when the model signals no archetype
/// is a good fit (literal `"none"` slug). `confidence` is the model's
/// self-reported certainty, clamped to `[0.0, 1.0]`.
///
/// Mission `mission-genesis-non-archetype-roles`: callers compare
/// against `broad_role::BROAD_ROLE_CONFIDENCE_THRESHOLD` (default 0.6)
/// to decide whether to route to the broader-generation path. The
/// sibling that produces a `ClassificationResult` lives at
/// [`crate::broad_role::classify_llm_with_confidence`] to keep this
/// file under the 500-LOC ceiling.
#[derive(Debug, Clone)]
pub struct ClassificationResult {
    pub archetype: Option<Archetype>,
    pub confidence: f32,
}

impl ClassificationResult {
    /// Build a result by running the deterministic keyword
    /// classifier; used as the fallback path when the LLM is
    /// unavailable or errors. Successful classifications carry
    /// `confidence = 1.0`; "no match" carries `0.0`.
    #[must_use]
    pub fn keyword_fallback(charter: &str) -> Self {
        match classify_charter(charter) {
            Ok(a) => Self {
                archetype: Some(a),
                confidence: 1.0,
            },
            Err(_) => Self {
                archetype: None,
                confidence: 0.0,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marketing_intern_charter_classifies_correctly() {
        let a =
            classify_charter("be my marketing intern, run weekly campaigns to cold leads").unwrap();
        assert_eq!(a, Archetype::MarketingIntern);
    }

    #[test]
    fn ops_oncall_charter_classifies_correctly() {
        let a = classify_charter("you're my oncall ops engineer, page me when an incident fires")
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
        let err = classify_charter("xyzzy frobnicate the bazquux").expect_err("nonsense charter");
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

#[cfg(test)]
#[path = "archetype_tests.rs"]
mod llm_tests;
