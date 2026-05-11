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
pub async fn classify_llm(
    charter: &str,
    llm: &dyn ProviderClient,
) -> ArgoResult<Archetype> {
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
            Ok(a) => Self { archetype: Some(a), confidence: 1.0 },
            Err(_) => Self { archetype: None, confidence: 0.0 },
        }
    }
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

#[cfg(test)]
#[path = "archetype_tests.rs"]
mod llm_tests;
