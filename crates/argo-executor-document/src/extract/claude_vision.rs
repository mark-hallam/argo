//! Anthropic Claude vision adapter for document extraction.

use crate::extract::provider::{
    ExtractionError, ExtractionProvider, ExtractionProviderRequest, ExtractionProviderResponse,
    FieldCandidate, ProviderUsage,
};
use base64::Engine;
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// Transport seam used by tests and by future async gateway wiring.
pub trait ClaudeVisionTransport {
    /// Post a complete Anthropic Messages payload and return the JSON response.
    fn post_messages(&self, payload: Value) -> Result<Value, ExtractionError>;
}

/// Claude vision extraction provider.
pub struct ClaudeVisionProvider<T> {
    model: String,
    transport: T,
}

impl<T> ClaudeVisionProvider<T> {
    /// Create a provider with an explicit model and transport.
    pub fn new(model: impl Into<String>, transport: T) -> Self {
        Self {
            model: model.into(),
            transport,
        }
    }
}

impl<T: ClaudeVisionTransport> ClaudeVisionProvider<T> {
    /// Build the one-call Anthropic Messages payload for a document.
    pub fn build_messages_payload(&self, request: &ExtractionProviderRequest) -> Value {
        let encoded = base64::engine::general_purpose::STANDARD.encode(&request.document.bytes);
        let correction = request
            .correction
            .as_deref()
            .map(|hint| format!("\nPrevious output failed validation: {hint}. Correct it."))
            .unwrap_or_default();
        json!({
            "model": self.model,
            "max_tokens": 2048,
            "messages": [{
                "role": "user",
                "content": [
                    {
                        "type": "image",
                        "source": {
                            "type": "base64",
                            "media_type": request.document.mime_type,
                            "data": encoded
                        }
                    },
                    {
                        "type": "text",
                        "text": format!(
                            "Extract document fields as strict JSON. Fields: {}. \
                             Return {{\"fields\":{{name:{{\"value\":...,\"confidence\":0.0}}}}}} only.{}",
                            request.schema.prompt_fields(),
                            correction
                        )
                    }
                ]
            }]
        })
    }

    fn parse_messages_response(
        &self,
        response: Value,
    ) -> Result<ExtractionProviderResponse, ExtractionError> {
        let usage = parse_usage(&response);
        let text = response
            .get("content")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .find_map(|part| {
                (part.get("type").and_then(Value::as_str) == Some("text"))
                    .then(|| part.get("text").and_then(Value::as_str))
                    .flatten()
            })
            .ok_or_else(|| ExtractionError::Protocol("missing text content".into()))?;
        let body: Value = serde_json::from_str(text)
            .map_err(|e| ExtractionError::Protocol(format!("invalid extraction JSON: {e}")))?;
        parse_provider_body(body, usage)
    }
}

impl<T: ClaudeVisionTransport> ExtractionProvider for ClaudeVisionProvider<T> {
    fn provider_name(&self) -> &str {
        "anthropic"
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn extract(
        &self,
        request: &ExtractionProviderRequest,
    ) -> Result<ExtractionProviderResponse, ExtractionError> {
        let payload = self.build_messages_payload(request);
        let response = self.transport.post_messages(payload)?;
        self.parse_messages_response(response)
    }
}

fn parse_usage(response: &Value) -> ProviderUsage {
    let input_tokens = response
        .pointer("/usage/input_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0) as u32;
    let output_tokens = response
        .pointer("/usage/output_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0) as u32;
    ProviderUsage {
        input_tokens,
        output_tokens,
        cost_usd: estimate_claude_cost(input_tokens, output_tokens),
    }
}

fn estimate_claude_cost(input_tokens: u32, output_tokens: u32) -> f64 {
    let input = input_tokens as f64 / 1_000_000.0 * 3.0;
    let output = output_tokens as f64 / 1_000_000.0 * 15.0;
    input + output
}

fn parse_provider_body(
    body: Value,
    usage: ProviderUsage,
) -> Result<ExtractionProviderResponse, ExtractionError> {
    let fields_obj = body
        .get("fields")
        .and_then(Value::as_object)
        .ok_or_else(|| ExtractionError::Protocol("missing fields object".into()))?;
    let mut fields = BTreeMap::new();
    for (name, field) in fields_obj {
        let value = field.get("value").cloned().unwrap_or_else(|| field.clone());
        let confidence = field
            .get("confidence")
            .and_then(Value::as_f64)
            .map(|v| v as f32);
        fields.insert(name.clone(), FieldCandidate { value, confidence });
    }
    Ok(ExtractionProviderResponse { fields, usage })
}

#[cfg(test)]
pub(crate) fn anthropic_text_response(text: &str) -> Value {
    json!({
        "content": [{"type": "text", "text": text}],
        "usage": {"input_tokens": 1000, "output_tokens": 200}
    })
}
