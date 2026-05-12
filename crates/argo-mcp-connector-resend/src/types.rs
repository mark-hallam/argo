//! Wire types for the Resend connector.
//!
//! `POST /emails` accepts a flat JSON object, `GET /emails/<id>` returns
//! a flat status, `GET /domains` returns `{ data: [<domain>, …] }`. The
//! typed [`ResendError`] enum maps documented failure modes to distinct
//! variants (quarantine a sender on `DomainNotVerified`, back off on
//! `RateLimited`); unknown failures fall through to [`ResendError::Other`]
//! with status + body preserved for audit.

use serde::{Deserialize, Serialize};

/// Resend region. Affects the base URL only; auth + payload schema are
/// identical between regions. Operators choose `Eu` for EU data residency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResendRegion {
    /// US region: `https://api.resend.com`.
    #[serde(rename = "us")]
    Us,
    /// EU region: `https://api.eu.resend.com`.
    #[serde(rename = "eu")]
    Eu,
}

impl ResendRegion {
    /// Base URL for this region (no trailing slash).
    #[must_use]
    pub fn base_url(self) -> &'static str {
        match self {
            Self::Us => "https://api.resend.com",
            Self::Eu => "https://api.eu.resend.com",
        }
    }

    /// Parse `"us"` / `"eu"` (case-insensitive). Empty input → `Us`.
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "" | "us" => Some(Self::Us),
            "eu" => Some(Self::Eu),
            _ => None,
        }
    }
}

/// Resend-assigned message ID — the key for looking up delivery status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ResendMessageId(pub String);

impl ResendMessageId {
    /// Borrow the underlying string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Operator-supplied tag pair (Resend caps at 10 server-side).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResendTag {
    /// Tag name.
    pub name: String,
    /// Tag value.
    pub value: String,
}

/// Outbound email payload. At least one of `html` / `text` must be set;
/// the connector surfaces missing-body as [`ResendError::InvalidEmail`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResendEmail {
    /// `From:` header (must use a verified Resend domain).
    pub from: String,
    /// Primary recipients.
    pub to: Vec<String>,
    /// `Subject:` header.
    pub subject: String,
    /// HTML body.
    pub html: Option<String>,
    /// Plain-text body.
    pub text: Option<String>,
    /// Optional `Reply-To:` header.
    pub reply_to: Option<String>,
    /// Cc recipients.
    pub cc: Vec<String>,
    /// Bcc recipients.
    pub bcc: Vec<String>,
    /// Operator analytics tags.
    pub tags: Vec<ResendTag>,
}

/// Delivery status (response of `GET /emails/<id>`). `last_event` is
/// kept as `String` so new Resend event kinds do not break decode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResendEmailStatus {
    /// Resend message ID.
    pub id: String,
    /// `From:` header on the sent message.
    pub from: String,
    /// Recipients on the sent message.
    pub to: Vec<String>,
    /// `Subject:` header on the sent message.
    pub subject: String,
    /// Most recent webhook event Resend recorded.
    pub last_event: String,
    /// Send-time timestamp (ISO-8601).
    pub created_at: String,
}

/// One sender domain in the operator's Resend account.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResendDomain {
    /// Resend-assigned domain ID.
    pub id: String,
    /// Domain name.
    pub name: String,
    /// Verification status (`pending` / `verified` / `failed` / …).
    pub status: String,
    /// Region the domain lives in.
    pub region: String,
    /// Creation timestamp (ISO-8601).
    pub created_at: String,
    /// DNS records the operator must publish to verify the domain.
    #[serde(default)]
    pub records: Vec<DnsRecord>,
}

/// A single DNS record for domain verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsRecord {
    /// Record kind (`MX`, `TXT`, `CNAME`, …).
    pub record: String,
    /// Record name (left side).
    pub name: String,
    /// Record value (right side).
    pub value: String,
    /// Reported verification status of this record.
    #[serde(default)]
    pub status: String,
    /// TTL hint.
    pub ttl: Option<String>,
    /// Priority (MX records).
    pub priority: Option<u32>,
}

/// Typed failure modes for the Resend connector.
#[derive(Debug, thiserror::Error)]
pub enum ResendError {
    /// Bearer auth failed (HTTP 401).
    #[error("resend: API key authentication failed")]
    AuthFailed,
    /// HTTP 429 — `retry_after_secs` is the `Retry-After` header (default `1`).
    #[error("resend: rate-limited; retry after {retry_after_secs}s")]
    RateLimited {
        /// Seconds to wait before retrying.
        retry_after_secs: u64,
    },
    /// Sender domain not verified in the operator's Resend dashboard.
    #[error("resend: sender domain not verified: {domain}")]
    DomainNotVerified {
        /// Domain Resend rejected.
        domain: String,
    },
    /// Email payload invalid (missing body, bad address, etc.).
    #[error("resend: invalid email: {reason}")]
    InvalidEmail {
        /// Why the email was rejected.
        reason: String,
    },
    /// Recipient is on Resend's suppression list.
    #[error("resend: recipient blocked: {recipient}")]
    RecipientBlocked {
        /// Address Resend declined to deliver to.
        recipient: String,
    },
    /// Batch send exceeded Resend's per-call cap.
    #[error("resend: batch too large: {actual} (max {max})")]
    BatchTooLarge {
        /// Number of emails the caller supplied.
        actual: usize,
        /// Hard cap per `POST /emails/batch`.
        max: usize,
    },
    /// Catch-all for unrecognized 4xx/5xx responses.
    #[error("resend: HTTP {status}: {body}")]
    Other {
        /// HTTP status code.
        status: u16,
        /// Raw response body.
        body: String,
    },
    /// Local transport failure.
    #[error("resend: transport error: {0}")]
    Transport(String),
    /// Response body could not be decoded.
    #[error("resend: decode error: {0}")]
    Decode(String),
    /// API key was not present in the environment.
    #[error("resend: missing API key (set {env})")]
    MissingApiKey {
        /// Env var that should hold the key.
        env: &'static str,
    },
}
