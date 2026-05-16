//! Provider-neutral extraction types.

use crate::extract::schema::ExtractionSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use thiserror::Error;

/// Error returned by extraction orchestration or provider adapters.
#[derive(Debug, Error, PartialEq)]
pub enum ExtractionError {
    /// The configured provider cannot be called in this environment.
    #[error("provider unavailable: {0}")]
    ProviderUnavailable(String),
    /// Provider returned malformed JSON or a malformed envelope.
    #[error("provider protocol error: {0}")]
    Protocol(String),
    /// The provider returned fields that still failed schema validation.
    #[error("schema validation failed after retry: {0}")]
    SchemaValidation(String),
}

/// Binary document passed to a vision provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentInput {
    /// MIME type such as `image/png`, `image/jpeg`, or `application/pdf`.
    pub mime_type: String,
    /// Raw bytes for the scanned page or document.
    pub bytes: Vec<u8>,
    /// Page count stamped into the audit envelope.
    pub page_count: u32,
}

/// Request sent to an extraction provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtractionProviderRequest {
    /// Operator-provided extraction schema.
    pub schema: ExtractionSchema,
    /// Document bytes to inspect.
    pub document: DocumentInput,
    /// Correction hint present only on the single retry.
    pub correction: Option<String>,
}

/// One extracted field candidate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldCandidate {
    /// Extracted JSON value.
    pub value: Value,
    /// Optional provider confidence before normalization.
    pub confidence: Option<f32>,
}

/// Token/cost metadata returned by the provider.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct ProviderUsage {
    /// Input tokens reported by the model provider.
    pub input_tokens: u32,
    /// Output tokens reported by the model provider.
    pub output_tokens: u32,
    /// Provider-estimated cost in USD.
    pub cost_usd: f64,
}

/// Provider response before schema validation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtractionProviderResponse {
    /// Fields keyed by schema field name.
    pub fields: BTreeMap<String, FieldCandidate>,
    /// Provider usage/cost metadata.
    pub usage: ProviderUsage,
}

/// Provider abstraction for document extraction.
pub trait ExtractionProvider {
    /// Provider name for audit records.
    fn provider_name(&self) -> &str;
    /// Model identifier for audit records.
    fn model(&self) -> &str;
    /// Extract fields from the document in one provider call.
    fn extract(
        &self,
        request: &ExtractionProviderRequest,
    ) -> Result<ExtractionProviderResponse, ExtractionError>;
}
