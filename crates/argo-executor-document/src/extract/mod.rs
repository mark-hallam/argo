//! Document extraction over scanned or image-based documents.
//!
//! This module turns an operator JSON Schema plus document bytes into
//! typed extracted fields with confidence and audit metadata. The
//! provider abstraction keeps Anthropic Claude vision as the primary
//! implementation while leaving an explicit seam for PaddleOCR in M2.

pub mod claude_vision;
pub mod provider;
pub mod schema;
pub mod tables;

use crate::extract::provider::{
    DocumentInput, ExtractionError, ExtractionProvider, ExtractionProviderRequest, FieldCandidate,
    ProviderUsage,
};
use crate::extract::schema::{ExtractionSchema, SchemaViolation};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[cfg(test)]
mod tables_tests;
#[cfg(test)]
mod tests;

/// High-level extraction request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentExtractionRequest {
    /// Operator schema describing the desired output fields.
    pub schema: ExtractionSchema,
    /// Document bytes to inspect.
    pub document: DocumentInput,
}

/// Validated extracted field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtractedField {
    /// Extracted JSON value.
    pub value: Value,
    /// Confidence clamped to `[0.0, 1.0]`.
    pub confidence: f32,
}

/// Audit metadata emitted with every extraction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtractionAudit {
    /// Provider name, e.g. `anthropic`.
    pub provider: String,
    /// Provider model identifier.
    pub model: String,
    /// Page count from the document input.
    pub page_count: u32,
    /// Number of provider calls made.
    pub attempts: u8,
    /// Token and cost metadata.
    pub usage: ProviderUsage,
}

/// Validated extraction outcome.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtractionOutcome {
    /// Extracted fields keyed by schema name.
    pub fields: BTreeMap<String, ExtractedField>,
    /// Audit envelope for persistence by the caller.
    pub audit: ExtractionAudit,
}

/// Run extraction and retry exactly once when schema validation fails.
pub fn extract_document<P: ExtractionProvider>(
    provider: &P,
    request: DocumentExtractionRequest,
) -> Result<ExtractionOutcome, ExtractionError> {
    let first = call_provider(provider, &request, None)?;
    match validate_response(provider, &request, first, 1) {
        Ok(outcome) => Ok(outcome),
        Err(ValidationFailure { hint, .. }) => {
            let retry = call_provider(provider, &request, Some(hint.clone()))?;
            validate_response(provider, &request, retry, 2)
                .map_err(|failure| ExtractionError::SchemaValidation(failure.violations.join("; ")))
        }
    }
}

fn call_provider<P: ExtractionProvider>(
    provider: &P,
    request: &DocumentExtractionRequest,
    correction: Option<String>,
) -> Result<crate::extract::provider::ExtractionProviderResponse, ExtractionError> {
    provider.extract(&ExtractionProviderRequest {
        schema: request.schema.clone(),
        document: request.document.clone(),
        correction,
    })
}

struct ValidationFailure {
    hint: String,
    violations: Vec<String>,
}

fn validate_response<P: ExtractionProvider>(
    provider: &P,
    request: &DocumentExtractionRequest,
    response: crate::extract::provider::ExtractionProviderResponse,
    attempts: u8,
) -> Result<ExtractionOutcome, ValidationFailure> {
    let values = response
        .fields
        .iter()
        .map(|(name, field)| (name.clone(), field.value.clone()))
        .collect::<BTreeMap<_, _>>();
    let violations = request.schema.validate_values(&values);
    if !violations.is_empty() {
        let hints = violations
            .iter()
            .map(SchemaViolation::correction_hint)
            .collect::<Vec<_>>();
        return Err(ValidationFailure {
            hint: hints.join("; "),
            violations: hints,
        });
    }
    let fields = response
        .fields
        .into_iter()
        .map(|(name, field)| (name, normalize_field(field)))
        .collect();
    Ok(ExtractionOutcome {
        fields,
        audit: ExtractionAudit {
            provider: provider.provider_name().into(),
            model: provider.model().into(),
            page_count: request.document.page_count,
            attempts,
            usage: response.usage,
        },
    })
}

fn normalize_field(field: FieldCandidate) -> ExtractedField {
    let confidence = field
        .confidence
        .unwrap_or_else(|| infer_confidence(&field.value));
    ExtractedField {
        value: field.value,
        confidence: confidence.clamp(0.0, 1.0),
    }
}

fn infer_confidence(value: &Value) -> f32 {
    match value {
        Value::Null => 0.0,
        Value::String(s) if s.trim().is_empty() => 0.35,
        Value::Array(a) if a.is_empty() => 0.55,
        Value::Object(o) if o.is_empty() => 0.55,
        _ => 0.82,
    }
}
