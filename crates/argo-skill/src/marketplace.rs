//! Cross-tenant skill marketplace search.
//!
//! V1 uses a deterministic manifest index shaped like an OCI
//! registry catalog. The search surface is intentionally pure so
//! the CLI and later registry-backed refresh jobs can share ranking
//! and trust filtering.

use crate::TrustGrade;
use serde::Serialize;

/// Default number of marketplace rows returned by [`search`].
pub const DEFAULT_SEARCH_LIMIT: usize = 20;

/// One searchable skill manifest entry from the marketplace index.
///
/// ```
/// use argo_skill::marketplace::{search, SearchOptions};
/// let rows = search("listing", &SearchOptions::default());
/// assert!(rows.iter().any(|row| row.name.contains("listing")));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MarketplaceSkill {
    /// Skill package name.
    pub name: String,
    /// Tenant or publisher namespace that published the skill.
    pub tenant: String,
    /// One-line human-readable summary.
    pub description: String,
    /// Published semantic version.
    pub version: String,
    /// Trust grade inferred from publisher provenance.
    pub trust_grade: TrustGrade,
    /// RFC-3339 timestamp from the manifest index.
    pub last_published_at: String,
    /// Canonical OCI reference for pulling the skill bundle.
    pub oci_ref: String,
}

/// Filters and pagination controls for marketplace search.
///
/// ```
/// use argo_skill::{marketplace::{search, SearchOptions}, TrustGrade};
/// let options = SearchOptions {
///     trust_min: Some(TrustGrade::PublishedCommunity),
///     tenant: Some("acme".into()),
///     limit: 1,
/// };
/// assert_eq!(search("listing", &options).len(), 1);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchOptions {
    /// Minimum acceptable trust grade; more-trusted grades still pass.
    pub trust_min: Option<TrustGrade>,
    /// Optional case-insensitive tenant/publisher namespace filter.
    pub tenant: Option<String>,
    /// Maximum number of ranked results to return.
    pub limit: usize,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            trust_min: None,
            tenant: None,
            limit: DEFAULT_SEARCH_LIMIT,
        }
    }
}

/// Search the mocked OCI manifest index and return ranked matches.
///
/// The ranking favours query hits in the skill name, then
/// description and OCI reference. Ties prefer higher trust, newer
/// publication time, then lexical name for deterministic output.
///
/// ```
/// use argo_skill::marketplace::{search, SearchOptions};
/// let rows = search("follow up", &SearchOptions::default());
/// assert_eq!(rows[0].name, "marketplace-follow-up");
/// ```
#[must_use]
pub fn search(query: &str, options: &SearchOptions) -> Vec<MarketplaceSkill> {
    let terms = query_terms(query);
    let tenant = options.tenant.as_deref().map(str::to_ascii_lowercase);
    let mut scored = Vec::new();

    for entry in MANIFEST_INDEX {
        if !passes_trust(entry.trust_grade, options.trust_min) {
            continue;
        }
        if let Some(ref tenant_filter) = tenant {
            if !entry.tenant.to_ascii_lowercase().contains(tenant_filter) {
                continue;
            }
        }
        let score = score_entry(entry, &terms);
        if score == 0 {
            continue;
        }
        scored.push((score, entry));
    }

    scored.sort_by(|(score_a, a), (score_b, b)| {
        score_b
            .cmp(score_a)
            .then_with(|| a.trust_grade.cmp(&b.trust_grade))
            .then_with(|| b.last_published_at.cmp(a.last_published_at))
            .then_with(|| a.name.cmp(b.name))
    });

    scored
        .into_iter()
        .take(options.limit)
        .map(|(_, entry)| entry.to_public())
        .collect()
}

/// Parse a trust-grade CLI label into the enum used by search.
///
/// Accepts snake_case and kebab-case labels.
///
/// ```
/// use argo_skill::{marketplace::parse_trust_grade, TrustGrade};
/// assert_eq!(parse_trust_grade("published-verified").unwrap(), TrustGrade::PublishedVerified);
/// assert!(parse_trust_grade("unknown").is_err());
/// ```
pub fn parse_trust_grade(input: &str) -> Result<TrustGrade, String> {
    let label = input.trim().to_ascii_lowercase().replace(['-', ' '], "_");
    match label.as_str() {
        "builtin" => Ok(TrustGrade::Builtin),
        "published_verified" => Ok(TrustGrade::PublishedVerified),
        "tenant" => Ok(TrustGrade::Tenant),
        "published_community" => Ok(TrustGrade::PublishedCommunity),
        "learned_unreviewed" => Ok(TrustGrade::LearnedUnreviewed),
        _ => Err(format!(
            "unknown trust grade '{input}' - use builtin|published_verified|tenant|published_community|learned_unreviewed"
        )),
    }
}

fn query_terms(query: &str) -> Vec<String> {
    query
        .split_whitespace()
        .map(str::to_ascii_lowercase)
        .filter(|term| !term.is_empty())
        .collect()
}

fn passes_trust(grade: TrustGrade, min: Option<TrustGrade>) -> bool {
    match min {
        Some(required) => grade <= required,
        None => true,
    }
}

fn score_entry(entry: &IndexedSkill, terms: &[String]) -> u32 {
    if terms.is_empty() {
        return 1;
    }
    let name = entry.name.to_ascii_lowercase();
    let description = entry.description.to_ascii_lowercase();
    let tenant = entry.tenant.to_ascii_lowercase();
    let oci_ref = entry.oci_ref.to_ascii_lowercase();

    terms
        .iter()
        .map(|term| {
            let mut score = 0;
            if name.contains(term) {
                score += 8;
            }
            if description.contains(term) {
                score += 3;
            }
            if tenant.contains(term) {
                score += 1;
            }
            if oci_ref.contains(term) {
                score += 1;
            }
            score
        })
        .sum()
}

#[derive(Debug, Clone, Copy)]
struct IndexedSkill {
    name: &'static str,
    tenant: &'static str,
    description: &'static str,
    version: &'static str,
    trust_grade: TrustGrade,
    last_published_at: &'static str,
    oci_ref: &'static str,
}

impl IndexedSkill {
    fn to_public(self) -> MarketplaceSkill {
        MarketplaceSkill {
            name: self.name.into(),
            tenant: self.tenant.into(),
            description: self.description.into(),
            version: self.version.into(),
            trust_grade: self.trust_grade,
            last_published_at: self.last_published_at.into(),
            oci_ref: self.oci_ref.into(),
        }
    }
}

const MANIFEST_INDEX: &[IndexedSkill] = &[
    IndexedSkill {
        name: "marketplace-listing-draft",
        tenant: "acme",
        description: "Drafts buyer-safe marketplace listings from item notes and photos.",
        version: "1.4.0",
        trust_grade: TrustGrade::PublishedVerified,
        last_published_at: "2026-05-04T10:15:00Z",
        oci_ref: "ghcr.io/skycot/skills/marketplace-listing-draft:1.4.0",
    },
    IndexedSkill {
        name: "marketplace-follow-up",
        tenant: "acme",
        description: "Writes polite post-listing follow up messages with approval gates.",
        version: "1.2.1",
        trust_grade: TrustGrade::PublishedCommunity,
        last_published_at: "2026-05-05T08:30:00Z",
        oci_ref: "ghcr.io/skycot/skills/marketplace-follow-up:1.2.1",
    },
    IndexedSkill {
        name: "listing-price-check",
        tenant: "demo-team",
        description: "Compares marketplace listing prices before a publish action.",
        version: "0.8.3",
        trust_grade: TrustGrade::Tenant,
        last_published_at: "2026-05-01T18:00:00Z",
        oci_ref: "ghcr.io/skycot/skills/listing-price-check:0.8.3",
    },
    IndexedSkill {
        name: "support-ticket-summarizer",
        tenant: "globex",
        description: "Summarizes inbound support tickets and suggests next actions.",
        version: "2.0.0",
        trust_grade: TrustGrade::PublishedVerified,
        last_published_at: "2026-04-27T11:45:00Z",
        oci_ref: "ghcr.io/skycot/skills/support-ticket-summarizer:2.0.0",
    },
    IndexedSkill {
        name: "recruiter-reply-draft",
        tenant: "northstar",
        description: "Drafts recruiter replies in the worker persona voice.",
        version: "0.9.0",
        trust_grade: TrustGrade::PublishedCommunity,
        last_published_at: "2026-04-29T09:20:00Z",
        oci_ref: "ghcr.io/skycot/skills/recruiter-reply-draft:0.9.0",
    },
    IndexedSkill {
        name: "listing-photo-alt-text",
        tenant: "lab",
        description: "Learns alt text patterns for marketplace listing photos.",
        version: "0.3.0",
        trust_grade: TrustGrade::LearnedUnreviewed,
        last_published_at: "2026-05-02T16:10:00Z",
        oci_ref: "ghcr.io/skycot/skills/listing-photo-alt-text:0.3.0",
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_returns_ranked_matches() {
        let rows = search("marketplace", &SearchOptions::default());
        assert_eq!(rows[0].name, "marketplace-listing-draft");
        assert!(rows.iter().any(|row| row.name == "marketplace-follow-up"));
    }

    #[test]
    fn trust_min_filters_less_trusted_rows() {
        let options = SearchOptions {
            trust_min: Some(TrustGrade::PublishedCommunity),
            tenant: None,
            limit: DEFAULT_SEARCH_LIMIT,
        };
        let rows = search("listing", &options);
        assert!(rows
            .iter()
            .all(|row| row.trust_grade <= TrustGrade::PublishedCommunity));
        assert!(!rows
            .iter()
            .any(|row| row.trust_grade == TrustGrade::LearnedUnreviewed));
    }

    #[test]
    fn tenant_and_limit_filters_apply_after_ranking() {
        let options = SearchOptions {
            trust_min: None,
            tenant: Some("acme".into()),
            limit: 1,
        };
        let rows = search("marketplace", &options);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].tenant, "acme");
        assert_eq!(rows[0].name, "marketplace-listing-draft");
    }

    #[test]
    fn parse_trust_grade_accepts_cli_labels() {
        assert_eq!(
            parse_trust_grade("published-verified"),
            Ok(TrustGrade::PublishedVerified)
        );
        assert_eq!(
            parse_trust_grade("learned_unreviewed"),
            Ok(TrustGrade::LearnedUnreviewed)
        );
        assert!(parse_trust_grade("made-up").is_err());
    }
}
