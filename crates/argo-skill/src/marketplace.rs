//! Cross-tenant skill marketplace search.
//!
//! Two layers:
//!
//! - [`search`] / [`SearchOptions`] — the original pure search over a
//!   baked-in [`STATIC_INDEX_DATA`] table. Kept verbatim for back-compat
//!   (existing tests assert exact catalog contents).
//! - [`MarketplaceSource`] / [`MarketplaceClient`] — composable layer
//!   that merges the static index with live OCI registry scans
//!   (see [`oci::OciRegistryScan`]). The CLI catalog handler uses this
//!   layer when `~/.argo/marketplace.toml` declares registries; absent
//!   that file, the client falls back to a static-only path.
//!
//! Mission ref: `mission-marketplace-oci-registry-scan` — replaces the
//! hardcoded `MANIFEST_INDEX` (now [`STATIC_INDEX_DATA`]) with a real
//! OCI `_catalog` walk while preserving offline / dev-mode behavior.

use crate::TrustGrade;
use serde::Serialize;
use thiserror::Error;

#[path = "marketplace_oci.rs"]
pub mod oci;

#[cfg(test)]
#[path = "marketplace_tests.rs"]
mod marketplace_tests;

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
    let rows: Vec<MarketplaceSkill> =
        STATIC_INDEX_DATA.iter().map(|e| e.to_public()).collect();
    rank_marketplace_skills(rows, query, options)
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

const STATIC_INDEX_DATA: &[IndexedSkill] = &[
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

/// Error returned by a [`MarketplaceSource`] when a list fetch fails.
///
/// ```
/// use argo_skill::marketplace::MarketplaceError;
/// let err = MarketplaceError::Network { url: "https://r.example".into(), reason: "timeout".into() };
/// assert!(err.to_string().contains("timeout"));
/// ```
#[derive(Debug, Error)]
pub enum MarketplaceError {
    /// HTTP / network failure while reaching a source.
    #[error("network error from {url}: {reason}")]
    Network {
        /// Origin URL of the failing request.
        url: String,
        /// Short failure reason.
        reason: String,
    },
    /// Marketplace config (TOML) could not be loaded / parsed.
    #[error("marketplace config: {0}")]
    Config(String),
}

/// Async source of [`MarketplaceSkill`] rows.
///
/// Implementations are composable behind [`MarketplaceClient`]. The
/// default [`StaticIndex`] wraps the baked-in [`STATIC_INDEX_DATA`]
/// table; [`oci::OciRegistryScan`] walks a live OCI registry catalog.
///
/// ```no_run
/// use argo_skill::marketplace::{MarketplaceSource, StaticIndex};
/// # async fn _doc() {
/// let rows = StaticIndex.list_all().await.unwrap();
/// assert!(!rows.is_empty());
/// # }
/// ```
#[async_trait::async_trait]
pub trait MarketplaceSource: Send + Sync {
    /// Return every skill the source knows about. Filtering and
    /// ranking are applied by the consumer (e.g.
    /// [`MarketplaceClient::search`]).
    async fn list_all(&self) -> Result<Vec<MarketplaceSkill>, MarketplaceError>;
}

/// [`MarketplaceSource`] wrapping the in-binary [`STATIC_INDEX_DATA`].
///
/// Always succeeds; carries the offline / dev-mode catalog.
#[derive(Debug, Default, Clone, Copy)]
pub struct StaticIndex;

#[async_trait::async_trait]
impl MarketplaceSource for StaticIndex {
    async fn list_all(&self) -> Result<Vec<MarketplaceSkill>, MarketplaceError> {
        Ok(STATIC_INDEX_DATA.iter().map(|e| e.to_public()).collect())
    }
}

/// Composes multiple [`MarketplaceSource`] implementations.
///
/// `list_all` merges every source's rows and dedupes by skill `name`
/// (the v1 identity column). `search` applies the same ranking +
/// trust + tenant filters as the legacy [`search`] free function.
/// Sources that error are logged with `tracing::warn!` and excluded
/// from the merge so a single registry outage cannot fail the
/// command.
///
/// ```no_run
/// use argo_skill::marketplace::{MarketplaceClient, SearchOptions};
/// # async fn _doc() {
/// let client = MarketplaceClient::with_static();
/// let rows = client.search("listing", &SearchOptions::default()).await;
/// assert!(!rows.is_empty());
/// # }
/// ```
pub struct MarketplaceClient {
    sources: Vec<Box<dyn MarketplaceSource>>,
}

impl MarketplaceClient {
    /// Wrap a pre-built source list.
    #[must_use]
    pub fn new(sources: Vec<Box<dyn MarketplaceSource>>) -> Self {
        Self { sources }
    }

    /// Convenience for tests / offline mode — static index only.
    #[must_use]
    pub fn with_static() -> Self {
        Self::new(vec![Box::new(StaticIndex)])
    }

    /// Fetch from every source, log + drop errors, dedupe by `name`.
    /// First-source-wins on duplicates so the static index (added
    /// first by convention) anchors well-known names.
    pub async fn list_all(&self) -> Vec<MarketplaceSkill> {
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut out: Vec<MarketplaceSkill> = Vec::new();
        for src in &self.sources {
            match src.list_all().await {
                Ok(rows) => {
                    for row in rows {
                        if seen.insert(row.name.clone()) {
                            out.push(row);
                        }
                    }
                }
                Err(err) => {
                    tracing::warn!(
                        target: "argo_skill::marketplace",
                        error = %err,
                        "marketplace source list_all failed; excluding from merge"
                    );
                }
            }
        }
        out
    }

    /// Search merged sources with the legacy ranking + filters.
    pub async fn search(
        &self,
        query: &str,
        options: &SearchOptions,
    ) -> Vec<MarketplaceSkill> {
        rank_marketplace_skills(self.list_all().await, query, options)
    }
}

fn rank_marketplace_skills(
    rows: Vec<MarketplaceSkill>,
    query: &str,
    options: &SearchOptions,
) -> Vec<MarketplaceSkill> {
    let terms = query_terms(query);
    let tenant = options.tenant.as_deref().map(str::to_ascii_lowercase);
    let mut scored = Vec::new();
    for row in rows {
        if !passes_trust(row.trust_grade, options.trust_min) {
            continue;
        }
        if let Some(ref tenant_filter) = tenant {
            if !row.tenant.to_ascii_lowercase().contains(tenant_filter) {
                continue;
            }
        }
        let score = score_marketplace_skill(&row, &terms);
        if score == 0 {
            continue;
        }
        scored.push((score, row));
    }
    scored.sort_by(|(score_a, a), (score_b, b)| {
        score_b
            .cmp(score_a)
            .then_with(|| a.trust_grade.cmp(&b.trust_grade))
            .then_with(|| b.last_published_at.cmp(&a.last_published_at))
            .then_with(|| a.name.cmp(&b.name))
    });
    scored
        .into_iter()
        .take(options.limit)
        .map(|(_, r)| r)
        .collect()
}

fn score_marketplace_skill(row: &MarketplaceSkill, terms: &[String]) -> u32 {
    if terms.is_empty() {
        return 1;
    }
    let name = row.name.to_ascii_lowercase();
    let description = row.description.to_ascii_lowercase();
    let tenant = row.tenant.to_ascii_lowercase();
    let oci_ref = row.oci_ref.to_ascii_lowercase();
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
