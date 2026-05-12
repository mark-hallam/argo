//! Lever ATS DTOs.
//!
//! Public types (`LeverCandidate`, `LeverPosting`, `LeverStage`) are the
//! caller-facing shape. The `*Wire` structs mirror Lever's JSON envelopes
//! (`emails: []`, `headline: ""`, `stage: "id"`, etc.) and are converted to
//! the public form at the serde boundary via `From` impls so callers never
//! see wire oddities like millisecond-precision unix timestamps.

use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};

/// Lever opportunity (Lever's name for a candidate × posting pairing).
///
/// Lever uses "opportunity" rather than "candidate" because the same person
/// applying to two roles produces two opportunity records. The brief and
/// this connector preserve the `Candidate` naming for parity with the
/// Greenhouse connector — the underlying record is Lever's opportunity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeverCandidate {
    /// Lever opportunity id (UUID-shaped).
    pub id: String,
    /// Display name (Lever's `name` field).
    pub name: String,
    /// Short headline (current title + company), often empty for fresh leads.
    pub headline: String,
    /// All email addresses Lever has on file.
    pub emails: Vec<String>,
    /// All phone numbers, raw as Lever stores them.
    pub phones: Vec<String>,
    /// Current stage id (per-tenant identifier — resolve via [`crate::LeverConnector::list_stages`]).
    pub stage_id: Option<String>,
    /// Posting ids this opportunity is attached to.
    pub posting_ids: Vec<String>,
    /// When Lever last updated this record.
    pub updated_at: Option<DateTime<Utc>>,
}

/// Lever posting (a single open req).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeverPosting {
    /// Posting id.
    pub id: String,
    /// Posting title (e.g. "Senior Rust Engineer").
    pub text: String,
    /// Posting state: `published`, `internal`, `closed`, `draft`, `pending`,
    /// or `rejected` (Lever's wire vocabulary, propagated verbatim).
    pub state: String,
    /// Team / department name attached to the posting.
    pub team: Option<String>,
    /// Office location label.
    pub location: Option<String>,
}

/// Lever pipeline stage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeverStage {
    /// Stage id (per-tenant). Use this when calling [`crate::LeverConnector::advance_stage`].
    pub id: String,
    /// Human-facing name (e.g. "Phone Screen", "Onsite").
    pub text: String,
}

// -- wire envelopes ----------------------------------------------------------

/// Lever wraps every list response in `{ data, hasNext, next }`.
#[derive(Debug, Deserialize)]
pub(crate) struct LeverPage<T> {
    pub data: Vec<T>,
    #[serde(default, rename = "hasNext")]
    pub has_next: bool,
    #[serde(default)]
    pub next: Option<String>,
}

/// Single-resource wrapper used by `GET /v1/opportunities/{id}`.
#[derive(Debug, Deserialize)]
pub(crate) struct LeverEnvelope<T> {
    pub data: T,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OpportunityWire {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub headline: String,
    #[serde(default)]
    pub emails: Vec<String>,
    #[serde(default)]
    pub phones: Vec<PhoneWire>,
    #[serde(default)]
    pub stage: Option<String>,
    #[serde(default)]
    pub postings: Vec<String>,
    #[serde(default, rename = "updatedAt")]
    pub updated_at_ms: Option<i64>,
}

/// Lever phone records can come as bare strings or `{value: "..."}` objects;
/// handle both via untagged deserialization.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum PhoneWire {
    /// `"+15551234567"`
    Bare(String),
    /// `{"value": "+15551234567", "type": "mobile"}`
    Tagged {
        /// E.164 or raw string, per Lever account configuration.
        value: String,
    },
}

impl PhoneWire {
    fn into_value(self) -> String {
        match self {
            Self::Bare(v) => v,
            Self::Tagged { value } => value,
        }
    }
}

impl From<OpportunityWire> for LeverCandidate {
    fn from(w: OpportunityWire) -> Self {
        Self {
            id: w.id,
            name: w.name,
            headline: w.headline,
            emails: w.emails,
            phones: w.phones.into_iter().map(PhoneWire::into_value).collect(),
            stage_id: w.stage.filter(|s| !s.is_empty()),
            posting_ids: w.postings,
            updated_at: w.updated_at_ms.and_then(ms_to_datetime),
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct PostingWire {
    pub id: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub categories: Option<CategoriesWire>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CategoriesWire {
    #[serde(default)]
    pub team: Option<String>,
    #[serde(default)]
    pub location: Option<String>,
}

impl From<PostingWire> for LeverPosting {
    fn from(w: PostingWire) -> Self {
        let (team, location) = w
            .categories
            .map(|c| (c.team, c.location))
            .unwrap_or((None, None));
        Self {
            id: w.id,
            text: w.text,
            state: w.state,
            team: team.filter(|s| !s.is_empty()),
            location: location.filter(|s| !s.is_empty()),
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct StageWire {
    pub id: String,
    #[serde(default)]
    pub text: String,
}

impl From<StageWire> for LeverStage {
    fn from(w: StageWire) -> Self {
        Self { id: w.id, text: w.text }
    }
}

// -- outbound bodies ---------------------------------------------------------

#[derive(Debug, Serialize)]
pub(crate) struct NoteBody<'a> {
    pub value: &'a str,
}

#[derive(Debug, Serialize)]
pub(crate) struct StageMoveBody<'a> {
    pub stage: &'a str,
}

// -- helpers -----------------------------------------------------------------

/// Format `since` for Lever's `updated_at_start` query parameter — Lever
/// accepts unix millis as a decimal string.
pub(crate) fn format_since(ts: DateTime<Utc>) -> String {
    ts.timestamp_millis().to_string()
}

fn ms_to_datetime(ms: i64) -> Option<DateTime<Utc>> {
    Utc.timestamp_millis_opt(ms).single()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opportunity_wire_handles_bare_and_tagged_phones() {
        let json = serde_json::json!({
            "id": "opp-1",
            "name": "Ada Lovelace",
            "emails": ["ada@example.test"],
            "phones": ["+15551234567", {"value": "+15557654321", "type": "mobile"}],
            "stage": "stage-phone-screen",
            "postings": ["posting-1"],
            "updatedAt": 1_715_000_000_000_i64
        });
        let wire: OpportunityWire = serde_json::from_value(json).unwrap();
        let public: LeverCandidate = wire.into();
        assert_eq!(public.id, "opp-1");
        assert_eq!(public.phones, vec!["+15551234567", "+15557654321"]);
        assert_eq!(public.stage_id.as_deref(), Some("stage-phone-screen"));
        assert!(public.updated_at.is_some());
    }

    #[test]
    fn opportunity_wire_empty_stage_becomes_none() {
        let json = serde_json::json!({"id": "opp-2", "stage": ""});
        let wire: OpportunityWire = serde_json::from_value(json).unwrap();
        let public: LeverCandidate = wire.into();
        assert!(public.stage_id.is_none());
    }

    #[test]
    fn posting_wire_flattens_categories_and_drops_empties() {
        let json = serde_json::json!({
            "id": "posting-1",
            "text": "Senior Rust Engineer",
            "state": "published",
            "categories": {"team": "Platform", "location": ""}
        });
        let wire: PostingWire = serde_json::from_value(json).unwrap();
        let public: LeverPosting = wire.into();
        assert_eq!(public.team.as_deref(), Some("Platform"));
        assert!(public.location.is_none());
    }

    #[test]
    fn format_since_emits_unix_millis_decimal() {
        let ts = Utc.with_ymd_and_hms(2026, 5, 1, 0, 0, 0).unwrap();
        assert_eq!(format_since(ts), "1777593600000");
    }
}
