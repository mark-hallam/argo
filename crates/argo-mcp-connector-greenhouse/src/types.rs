//! Greenhouse Harvest DTOs.
//!
//! Public types (`GhCandidate`, `GhApplication`, `GhJob`, ...) are the
//! caller-facing shape. The `*Wire` structs are the JSON-deserializable
//! shape that mirrors Greenhouse's response envelope; conversion happens
//! at the serde boundary via `From` impls so callers never see wire
//! oddities like nested `value`/`type` ref objects.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Greenhouse candidate (subset Argo uses).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GhCandidate {
    /// Greenhouse candidate id.
    pub id: i64,
    /// Given name.
    pub first_name: String,
    /// Family name.
    pub last_name: String,
    /// All email addresses on file (primary first, by Greenhouse convention).
    pub email_addresses: Vec<String>,
    /// Ids of every application owned by this candidate.
    pub applications: Vec<i64>,
}

/// Greenhouse application — one candidate × one job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GhApplication {
    /// Application id.
    pub id: i64,
    /// Candidate this application belongs to.
    pub candidate_id: i64,
    /// Job this application is against.
    pub job_id: i64,
    /// Current pipeline stage.
    pub current_stage: GhStage,
    /// Active / Hired / Rejected / Converted.
    pub status: GhApplicationStatus,
    /// Free-text source (e.g. "LinkedIn", "Referral").
    pub source: Option<String>,
}

/// Pipeline stage reference on an application.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GhStage {
    /// Stage id (unique per job-stage pair).
    pub id: i64,
    /// Human-facing stage name (e.g. "Phone Screen").
    pub name: String,
}

/// Greenhouse job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GhJob {
    /// Job id.
    pub id: i64,
    /// Job name (req title).
    pub name: String,
    /// Open / Closed / Draft.
    pub status: GhJobStatus,
    /// Department names attached to the job.
    pub departments: Vec<String>,
    /// Office names attached to the job.
    pub offices: Vec<String>,
}

/// Application lifecycle status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GhApplicationStatus {
    /// In pipeline.
    Active,
    /// Hired and converted to employee.
    Hired,
    /// Rejected (with reason on the application record).
    Rejected,
    /// Converted (e.g. prospect → applicant).
    Converted,
}

impl GhApplicationStatus {
    /// Parse a Greenhouse status string. Unknown values map to `Active`
    /// (the API default) — callers concerned about strictness should
    /// inspect the original wire payload.
    #[must_use]
    pub fn from_wire(raw: &str) -> Self {
        match raw {
            "hired" => Self::Hired,
            "rejected" => Self::Rejected,
            "converted" => Self::Converted,
            _ => Self::Active,
        }
    }
}

/// Job posting status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GhJobStatus {
    /// Accepting applications.
    Open,
    /// No longer accepting applications.
    Closed,
    /// Draft (not yet posted).
    Draft,
}

impl GhJobStatus {
    /// Wire-format string Greenhouse expects on `status=` query params.
    #[must_use]
    pub fn as_query_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Closed => "closed",
            Self::Draft => "draft",
        }
    }

    /// Parse from Greenhouse response payload.
    #[must_use]
    pub fn from_wire(raw: &str) -> Self {
        match raw {
            "closed" => Self::Closed,
            "draft" => Self::Draft,
            _ => Self::Open,
        }
    }
}

/// Visibility scope for a Greenhouse note.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GhNoteVisibility {
    /// Only Site Admins can see this note.
    Admin,
    /// Visible to anyone who can see the candidate.
    Public,
    /// Only the author can see this note.
    Private,
}

impl GhNoteVisibility {
    /// Greenhouse expects lower-case visibility on the wire.
    #[must_use]
    pub fn as_wire_str(self) -> &'static str {
        match self {
            Self::Admin => "admin_only",
            Self::Public => "public",
            Self::Private => "private",
        }
    }
}

// -- wire shapes -------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub(crate) struct CandidateWire {
    pub id: i64,
    #[serde(default)]
    pub first_name: String,
    #[serde(default)]
    pub last_name: String,
    #[serde(default)]
    pub email_addresses: Vec<EmailWire>,
    #[serde(default)]
    pub applications: Vec<ApplicationRefWire>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct EmailWire {
    pub value: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub r#type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ApplicationRefWire {
    pub id: i64,
}

impl From<CandidateWire> for GhCandidate {
    fn from(wire: CandidateWire) -> Self {
        Self {
            id: wire.id,
            first_name: wire.first_name,
            last_name: wire.last_name,
            email_addresses: wire.email_addresses.into_iter().map(|e| e.value).collect(),
            applications: wire.applications.into_iter().map(|a| a.id).collect(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct ApplicationWire {
    pub id: i64,
    pub candidate_id: i64,
    #[serde(default)]
    pub jobs: Vec<JobRefWire>,
    pub current_stage: Option<StageWire>,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub source: Option<SourceWire>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct JobRefWire {
    pub id: i64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct StageWire {
    pub id: i64,
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SourceWire {
    #[serde(default)]
    pub public_name: Option<String>,
}

impl From<ApplicationWire> for GhApplication {
    fn from(wire: ApplicationWire) -> Self {
        Self {
            id: wire.id,
            candidate_id: wire.candidate_id,
            job_id: wire.jobs.first().map(|j| j.id).unwrap_or_default(),
            current_stage: wire
                .current_stage
                .map(|s| GhStage { id: s.id, name: s.name })
                .unwrap_or_default(),
            status: GhApplicationStatus::from_wire(&wire.status),
            source: wire.source.and_then(|s| s.public_name),
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct JobWire {
    pub id: i64,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub departments: Vec<NamedWire>,
    #[serde(default)]
    pub offices: Vec<NamedWire>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct NamedWire {
    #[serde(default)]
    pub name: String,
}

impl From<JobWire> for GhJob {
    fn from(wire: JobWire) -> Self {
        Self {
            id: wire.id,
            name: wire.name,
            status: GhJobStatus::from_wire(&wire.status),
            departments: wire.departments.into_iter().map(|n| n.name).collect(),
            offices: wire.offices.into_iter().map(|n| n.name).collect(),
        }
    }
}

// -- outbound bodies ---------------------------------------------------------

#[derive(Debug, Serialize)]
pub(crate) struct AdvanceBody {
    pub from_stage_id: Option<i64>,
    pub to_stage_id: i64,
}

#[derive(Debug, Serialize)]
pub(crate) struct RejectBody<'a> {
    pub rejection_reason_id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<&'a str>,
}

#[derive(Debug, Serialize)]
pub(crate) struct NoteBody<'a> {
    pub user_id: i64,
    pub body: &'a str,
    pub visibility: &'static str,
}

/// Format `since` as an ISO-8601 timestamp for the `updated_after` query
/// parameter. Greenhouse accepts RFC3339; we emit UTC seconds precision
/// which matches the API's documented format.
pub(crate) fn format_since(ts: DateTime<Utc>) -> String {
    ts.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_parses_known_variants() {
        assert_eq!(GhApplicationStatus::from_wire("hired"), GhApplicationStatus::Hired);
        assert_eq!(GhApplicationStatus::from_wire("rejected"), GhApplicationStatus::Rejected);
        assert_eq!(GhApplicationStatus::from_wire("converted"), GhApplicationStatus::Converted);
        assert_eq!(GhApplicationStatus::from_wire("active"), GhApplicationStatus::Active);
        // Unknown maps to Active.
        assert_eq!(GhApplicationStatus::from_wire("???"), GhApplicationStatus::Active);
    }

    #[test]
    fn job_status_round_trips_query_strings() {
        assert_eq!(GhJobStatus::Open.as_query_str(), "open");
        assert_eq!(GhJobStatus::Closed.as_query_str(), "closed");
        assert_eq!(GhJobStatus::Draft.as_query_str(), "draft");
        assert_eq!(GhJobStatus::from_wire("closed"), GhJobStatus::Closed);
        assert_eq!(GhJobStatus::from_wire("draft"), GhJobStatus::Draft);
        assert_eq!(GhJobStatus::from_wire("open"), GhJobStatus::Open);
    }

    #[test]
    fn note_visibility_maps_to_greenhouse_strings() {
        assert_eq!(GhNoteVisibility::Admin.as_wire_str(), "admin_only");
        assert_eq!(GhNoteVisibility::Public.as_wire_str(), "public");
        assert_eq!(GhNoteVisibility::Private.as_wire_str(), "private");
    }
}
