//! Public DTOs and private wire shapes for the WhatsApp Business Cloud connector.

use serde::{Deserialize, Serialize};

/// Opaque `wamid.<base64>` id returned by WhatsApp for an outbound message.
/// Stable handle for delivery-status webhook reconciliation (v2 brief).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WhatsAppMessageId(pub String);

impl WhatsAppMessageId {
    /// Borrow the underlying string.
    #[must_use]
    pub fn as_str(&self) -> &str { &self.0 }
}

impl std::fmt::Display for WhatsAppMessageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str(&self.0) }
}

/// Media categories accepted by [`crate::WhatsAppCloudClient::send_media`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WhatsAppMediaKind {
    /// JPEG / PNG.
    Image,
    /// PDF and other document MIME types.
    Document,
    /// AAC / MP4 / AMR / MP3 / OGG.
    Audio,
    /// MP4 / 3GPP.
    Video,
    /// WebP sticker.
    Sticker,
}

/// Where the media bytes live. `Url` is fetched by Meta's edge (URL must be
/// publicly reachable); `Id` references media uploaded out-of-band.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WhatsAppMediaSource {
    /// HTTPS URL Meta fetches.
    Url(String),
    /// Pre-uploaded media id.
    Id(String),
}

/// One positional template-body variable. Templates use `{{1}}`, `{{2}}`,
/// … placeholders — emitted in declaration order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WhatsAppTemplateVariable {
    /// Substitution for the next positional placeholder.
    pub value: String,
}

impl WhatsAppTemplateVariable {
    /// Construct a variable from any string-like value.
    pub fn new(value: impl Into<String>) -> Self { Self { value: value.into() } }
}

impl From<&str> for WhatsAppTemplateVariable { fn from(s: &str) -> Self { Self::new(s) } }
impl From<String> for WhatsAppTemplateVariable { fn from(s: String) -> Self { Self::new(s) } }

/// Rate-limit snapshot decoded from `X-Business-Use-Case-Usage` or
/// `Retry-After` fall-back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WhatsAppRateLimitInfo {
    /// Operator-requested back-off in seconds (0 when unknown).
    pub retry_after_secs: u64,
    /// Raw header value preserved for observability.
    pub from_header: String,
}

/// Typed errors for every call surface in this crate.
#[derive(Debug, thiserror::Error)]
pub enum WhatsAppCloudError {
    /// HTTP 401 / 403 or a Meta error code indicating bad credentials.
    #[error("whatsapp-cloud: authentication failed: {0}")]
    AuthFailed(String),
    /// HTTP 429 or a Meta rate-limit error code.
    #[error("whatsapp-cloud: rate limited; retry after {retry_after_secs}s")]
    RateLimited { /// Seconds Meta requested the caller wait.
        retry_after_secs: u64 },
    /// Template `name`/`language` is not approved.
    #[error("whatsapp-cloud: template not approved: {0}")]
    TemplateNotApproved(String),
    /// Recipient has not opted in to messaging.
    #[error("whatsapp-cloud: recipient not opted in: {0}")]
    RecipientNotOptedIn(String),
    /// Free-form / media send attempted >24h since the customer's last
    /// inbound message; only templates are deliverable.
    #[error("whatsapp-cloud: outside 24h messaging window (template required): {0}")]
    OutsideMessagingWindow(String),
    /// Media-by-URL exceeded Meta's size cap or could not be fetched.
    #[error("whatsapp-cloud: media too large or unfetchable: {0}")]
    MediaTooLarge(String),
    /// Recipient was not strict E.164 (`+<country><number>`, digits only).
    #[error("whatsapp-cloud: invalid e164 recipient: {0}")]
    InvalidRecipient(String),
    /// Unmapped Meta error — upstream text preserved.
    #[error("whatsapp-cloud: {0}")]
    Other(String),
    /// Transport-level failure (DNS, connection refused, body decode).
    #[error("whatsapp-cloud transport: {0}")]
    Transport(String),
}

#[derive(Debug, Serialize)]
pub(crate) struct OutboundEnvelope<'a> {
    pub messaging_product: &'static str,
    pub recipient_type: &'static str,
    pub to: &'a str,
    #[serde(flatten)]
    pub payload: OutboundPayload<'a>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum OutboundPayload<'a> {
    Text { text: TextBody<'a> },
    Template { template: TemplatePayload<'a> },
    Image { image: MediaBody<'a> },
    Document { document: MediaBody<'a> },
    Audio { audio: MediaBody<'a> },
    Video { video: MediaBody<'a> },
    Sticker { sticker: MediaBody<'a> },
}

#[derive(Debug, Serialize)]
pub(crate) struct TextBody<'a> { pub body: &'a str }

#[derive(Debug, Serialize)]
pub(crate) struct TemplatePayload<'a> {
    pub name: &'a str,
    pub language: TemplateLanguage<'a>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub components: Vec<TemplateComponent>,
}

#[derive(Debug, Serialize)]
pub(crate) struct TemplateLanguage<'a> { pub code: &'a str }

#[derive(Debug, Serialize)]
pub(crate) struct TemplateComponent {
    #[serde(rename = "type")] pub kind: &'static str,
    pub parameters: Vec<TemplateParameter>,
}

#[derive(Debug, Serialize)]
pub(crate) struct TemplateParameter {
    #[serde(rename = "type")] pub kind: &'static str,
    pub text: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct MediaBody<'a> {
    #[serde(skip_serializing_if = "Option::is_none")] pub link: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")] pub id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")] pub caption: Option<&'a str>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ReadReceipt<'a> {
    pub messaging_product: &'static str,
    pub status: &'static str,
    pub message_id: &'a str,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SendResponse { pub messages: Vec<SendMessageId> }

#[derive(Debug, Deserialize)]
pub(crate) struct SendMessageId { pub id: String }

#[derive(Debug, Deserialize)]
pub(crate) struct ErrorEnvelope { pub error: ErrorBody }

#[derive(Debug, Deserialize)]
pub(crate) struct ErrorBody {
    #[serde(default)] pub message: String,
    #[serde(default)] pub code: i64,
}
