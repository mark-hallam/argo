use argo_types::TenantId;
use serde::{Deserialize, Serialize};

/// Lifecycle states an [`ApprovalRequest`] can occupy. Stored as a
/// lower-case string in the `status` column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    /// Awaiting a human grant/deny decision.
    Pending,
    /// A grantor approved the request.
    Granted,
    /// A grantor denied the request.
    Denied,
    /// `expire_overdue` flipped the row past its deadline.
    Expired,
}

impl ApprovalStatus {
    /// Lower-case label used in the `status` column.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Granted => "granted",
            Self::Denied => "denied",
            Self::Expired => "expired",
        }
    }

    /// Parse the column label back to a status.
    #[must_use]
    pub fn from_str_lossy(s: &str) -> Self {
        match s {
            "granted" => Self::Granted,
            "denied" => Self::Denied,
            "expired" => Self::Expired,
            _ => Self::Pending,
        }
    }
}

/// Default decision recorded at request-creation time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DefaultDecision {
    /// Treat the unattended request as approved.
    Allow,
    /// Treat the unattended request as still requiring a human.
    Ask,
    /// Treat the unattended request as denied.
    Deny,
}

impl DefaultDecision {
    /// Lower-case label used in the `default_decision` column.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Ask => "ask",
            Self::Deny => "deny",
        }
    }

    /// Parse the column label back to a default decision.
    #[must_use]
    pub fn from_str_lossy(s: &str) -> Self {
        match s {
            "allow" => Self::Allow,
            "ask" => Self::Ask,
            _ => Self::Deny,
        }
    }
}

/// One persisted approval-request row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalRequest {
    /// UUIDv7 id (BLOB in SQLite, hex-encoded in JSON).
    pub id: String,
    /// Action this request is gating. Opaque to the store.
    pub action_id: Vec<u8>,
    /// Owning tenant.
    pub tenant_id: TenantId,
    /// Requested-at micros since epoch.
    pub requested_at: i64,
    /// Free-text identifier for the requester.
    pub requested_by: String,
    /// Deadline-at micros since epoch.
    pub deadline_at: i64,
    /// The decision the timeout sweep applies on expiry.
    pub default_decision: DefaultDecision,
    /// Lifecycle state.
    pub status: ApprovalStatus,
    /// Decided-at micros since epoch.
    pub decided_at: Option<i64>,
    /// Free-text identifier for the grantor.
    pub decided_by: Option<String>,
    /// Final decision label.
    pub decision: Option<String>,
    /// Reminder/escalation sent-at micros since epoch.
    pub escalation_sent_at: Option<i64>,
    /// Optional executor-side preview of the draft body.
    pub draft_content: Option<String>,
}

/// Insert payload for `create_request`.
#[derive(Debug, Clone)]
pub struct NewApprovalRequest<'a> {
    /// Action this request is gating.
    pub action_id: Vec<u8>,
    /// Owning tenant.
    pub tenant_id: TenantId,
    /// Free-text identifier for the requester.
    pub requested_by: &'a str,
    /// Deadline-at micros since epoch.
    pub deadline_at: i64,
    /// The decision the timeout sweep applies on expiry.
    pub default_decision: DefaultDecision,
    /// Optional executor-side draft preview attached at creation time.
    pub draft_content: Option<&'a str>,
}
