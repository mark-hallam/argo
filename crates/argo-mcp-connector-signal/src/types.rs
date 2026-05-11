//! Wire and public DTOs for the Signal connector.
//!
//! signal-cli's JSON-RPC envelope shape is verbose; we mirror only the fields
//! the v1 connector exposes. Unknown fields are silently dropped — serde's
//! default is forward-compatible against signal-cli minor-version bumps.

use serde::{Deserialize, Serialize};

/// Server-side receipt for a `send_message` / `send_group_message` call.
///
/// `timestamp` is the millisecond-precision Signal envelope timestamp the
/// daemon assigned; it is the stable identifier for the outbound message and
/// is echoed back in delivery receipts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignalMessageReceipt {
    /// Signal envelope timestamp (ms since epoch).
    pub timestamp: i64,
}

/// A single inbound envelope as surfaced by `receive`.
///
/// `body` is `Option<String>` because some envelopes are receipt /
/// typing-indicator / sync-message frames with no text payload — preserve
/// nullability so callers can filter cleanly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignalIncomingMessage {
    /// Envelope timestamp (ms since epoch).
    pub envelope_timestamp: i64,
    /// E.164 phone number of the sender, e.g. `"+15551234567"`.
    pub source_number: String,
    /// Present iff the envelope was delivered through a Signal group; absent
    /// for direct messages.
    pub group_id: Option<String>,
    /// Textual body. Absent for non-text envelopes (receipts, typing, etc.).
    pub body: Option<String>,
    /// Attachment metadata. Empty for messages without attachments. v1 does
    /// not download attachment bytes.
    pub attachments: Vec<SignalAttachmentMeta>,
}

/// A Signal group as reported by `list_groups`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignalGroup {
    /// signal-cli group id (base64 group-v2 id).
    pub id: String,
    /// Display name.
    pub name: String,
    /// Member phone numbers in E.164 format.
    pub members: Vec<String>,
}

/// Metadata for a single inbound attachment.
///
/// The bytes themselves are NOT downloaded in v1 — the operator can fetch
/// them out-of-band via signal-cli if needed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignalAttachmentMeta {
    /// signal-cli attachment id (used to fetch bytes via the daemon).
    pub id: String,
    /// MIME content type, e.g. `"image/jpeg"`.
    pub content_type: String,
    /// Size in bytes.
    pub size: u64,
}

// ---------- wire shapes ----------

#[derive(Debug, Serialize)]
pub(crate) struct SendDirectArg<'a> {
    pub number: &'a str,
    pub recipient: &'a str,
    pub message: &'a str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SendGroupArg<'a> {
    pub number: &'a str,
    pub group_id: &'a str,
    pub message: &'a str,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SendResp {
    pub timestamp: i64,
}

impl From<SendResp> for SignalMessageReceipt {
    fn from(w: SendResp) -> Self {
        Self { timestamp: w.timestamp }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct EnvelopeWire {
    pub envelope: EnvelopeBody,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EnvelopeBody {
    #[serde(default)]
    pub timestamp: i64,
    #[serde(default)]
    pub source_number: String,
    #[serde(default)]
    pub data_message: Option<DataMessageWire>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DataMessageWire {
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub group_info: Option<GroupInfoWire>,
    #[serde(default)]
    pub attachments: Vec<AttachmentWire>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GroupInfoWire {
    pub group_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AttachmentWire {
    pub id: String,
    pub content_type: String,
    #[serde(default)]
    pub size: u64,
}

impl From<AttachmentWire> for SignalAttachmentMeta {
    fn from(w: AttachmentWire) -> Self {
        Self { id: w.id, content_type: w.content_type, size: w.size }
    }
}

impl EnvelopeWire {
    pub(crate) fn into_public(self) -> SignalIncomingMessage {
        let (body, group_id, attachments) = match self.envelope.data_message {
            Some(d) => (
                d.message,
                d.group_info.map(|g| g.group_id),
                d.attachments.into_iter().map(Into::into).collect(),
            ),
            None => (None, None, Vec::new()),
        };
        SignalIncomingMessage {
            envelope_timestamp: self.envelope.timestamp,
            source_number: self.envelope.source_number,
            group_id,
            body,
            attachments,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GroupWire {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub members: Vec<GroupMemberWire>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum GroupMemberWire {
    /// Plain E.164 number.
    Number(String),
    /// Object form with `number` field — newer signal-cli releases.
    Object {
        #[serde(default)]
        number: String,
    },
}

impl GroupMemberWire {
    pub(crate) fn into_number(self) -> String {
        match self {
            Self::Number(n) => n,
            Self::Object { number } => number,
        }
    }
}

impl From<GroupWire> for SignalGroup {
    fn from(w: GroupWire) -> Self {
        let members = w.members.into_iter().map(GroupMemberWire::into_number).collect();
        Self { id: w.id, name: w.name, members }
    }
}
