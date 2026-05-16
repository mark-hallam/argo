use super::*;
use crate::extract::claude_vision::{
    anthropic_text_response, ClaudeVisionProvider, ClaudeVisionTransport,
};
use crate::extract::provider::{
    ExtractionError, ExtractionProviderRequest, ExtractionProviderResponse,
};
use serde_json::{json, Value};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

#[derive(Default)]
struct ScriptedProvider {
    responses: RefCell<Vec<ExtractionProviderResponse>>,
    corrections: RefCell<Vec<Option<String>>>,
}

impl ScriptedProvider {
    fn new(responses: Vec<ExtractionProviderResponse>) -> Self {
        Self {
            responses: RefCell::new(responses.into_iter().rev().collect()),
            corrections: RefCell::new(Vec::new()),
        }
    }
}

impl provider::ExtractionProvider for ScriptedProvider {
    fn provider_name(&self) -> &str {
        "test-provider"
    }

    fn model(&self) -> &str {
        "test-model"
    }

    fn extract(
        &self,
        request: &ExtractionProviderRequest,
    ) -> Result<ExtractionProviderResponse, ExtractionError> {
        self.corrections
            .borrow_mut()
            .push(request.correction.clone());
        self.responses
            .borrow_mut()
            .pop()
            .ok_or_else(|| ExtractionError::Protocol("script exhausted".into()))
    }
}

struct StaticTransport {
    response: Value,
    seen_payload: Rc<RefCell<Option<Value>>>,
}

impl ClaudeVisionTransport for StaticTransport {
    fn post_messages(&self, payload: Value) -> Result<Value, ExtractionError> {
        *self.seen_payload.borrow_mut() = Some(payload);
        Ok(self.response.clone())
    }
}

fn schema(value: Value) -> schema::ExtractionSchema {
    schema::ExtractionSchema::from_json(&value).unwrap()
}

fn document() -> provider::DocumentInput {
    provider::DocumentInput {
        mime_type: "image/png".into(),
        bytes: b"fake image bytes".to_vec(),
        page_count: 1,
    }
}

fn response(fields: Vec<(&str, Value, Option<f32>)>) -> ExtractionProviderResponse {
    ExtractionProviderResponse {
        fields: fields
            .into_iter()
            .map(|(name, value, confidence)| {
                (name.into(), provider::FieldCandidate { value, confidence })
            })
            .collect::<BTreeMap<_, _>>(),
        usage: provider::ProviderUsage {
            input_tokens: 10,
            output_tokens: 5,
            cost_usd: 0.01,
        },
    }
}

fn invoice_schema() -> schema::ExtractionSchema {
    schema(json!({
        "type": "object",
        "required": ["invoice_number", "total", "line_items"],
        "properties": {
            "invoice_number": {"type": "string"},
            "total": {"type": "number"},
            "line_items": {"type": "array"}
        }
    }))
}

#[test]
fn parses_json_schema_subset() {
    let parsed = invoice_schema();
    assert_eq!(parsed.fields.len(), 3);
    assert!(parsed.fields["invoice_number"].required);
}

#[test]
fn rejects_non_object_schema_root() {
    let err = schema::ExtractionSchema::from_json(&json!(["bad"])).unwrap_err();
    assert_eq!(err, schema::SchemaError::RootNotObject);
}

#[test]
fn validation_reports_missing_required_field() {
    let violations = invoice_schema().validate_values(&BTreeMap::new());
    assert!(violations.iter().any(|v| matches!(
        v,
        schema::SchemaViolation::MissingRequired { field } if field == "total"
    )));
}

#[test]
fn validation_reports_wrong_type() {
    let mut values = BTreeMap::new();
    values.insert("invoice_number".into(), json!("INV-1"));
    values.insert("total".into(), json!("12.00"));
    values.insert("line_items".into(), json!([]));
    let violations = invoice_schema().validate_values(&values);
    assert_eq!(violations.len(), 1);
}

#[test]
fn schema_failure_retries_exactly_once() {
    let provider = ScriptedProvider::new(vec![
        response(vec![
            ("invoice_number", json!("INV-100"), Some(0.9)),
            ("total", json!("12.00"), Some(0.7)),
            ("line_items", json!([]), Some(0.8)),
        ]),
        response(vec![
            ("invoice_number", json!("INV-100"), Some(0.9)),
            ("total", json!(12.0), Some(0.91)),
            (
                "line_items",
                json!([{"description": "paper", "amount": 12.0}]),
                Some(0.88),
            ),
        ]),
    ]);
    let outcome = extract_document(
        &provider,
        DocumentExtractionRequest {
            schema: invoice_schema(),
            document: document(),
        },
    )
    .unwrap();
    assert_eq!(outcome.audit.attempts, 2);
    assert_eq!(provider.corrections.borrow().len(), 2);
    assert!(provider.corrections.borrow()[1]
        .as_ref()
        .unwrap()
        .contains("total"));
}

#[test]
fn schema_failure_after_retry_is_returned() {
    let provider = ScriptedProvider::new(vec![
        response(vec![("total", json!("bad"), Some(0.5))]),
        response(vec![("total", json!("still bad"), Some(0.5))]),
    ]);
    let err = extract_document(
        &provider,
        DocumentExtractionRequest {
            schema: invoice_schema(),
            document: document(),
        },
    )
    .unwrap_err();
    assert!(format!("{err}").contains("schema validation failed after retry"));
    assert_eq!(provider.corrections.borrow().len(), 2);
}

#[test]
fn missing_confidence_is_inferred_and_present() {
    let provider = ScriptedProvider::new(vec![response(vec![
        ("invoice_number", json!("INV-100"), None),
        ("total", json!(12.0), None),
        ("line_items", json!([{"description": "paper"}]), None),
    ])]);
    let outcome = extract_document(
        &provider,
        DocumentExtractionRequest {
            schema: invoice_schema(),
            document: document(),
        },
    )
    .unwrap();
    assert!(outcome
        .fields
        .values()
        .all(|field| field.confidence > 0.0 && field.confidence <= 1.0));
}

#[test]
fn audit_records_provider_model_page_count_and_cost() {
    let provider = ScriptedProvider::new(vec![response(vec![
        ("invoice_number", json!("INV-100"), Some(2.0)),
        ("total", json!(12.0), Some(0.9)),
        ("line_items", json!([]), Some(0.8)),
    ])]);
    let outcome = extract_document(
        &provider,
        DocumentExtractionRequest {
            schema: invoice_schema(),
            document: document(),
        },
    )
    .unwrap();
    assert_eq!(outcome.audit.provider, "test-provider");
    assert_eq!(outcome.audit.model, "test-model");
    assert_eq!(outcome.audit.page_count, 1);
    assert_eq!(outcome.audit.usage.cost_usd, 0.01);
    assert_eq!(outcome.fields["invoice_number"].confidence, 1.0);
}

#[test]
fn invoice_fixture_extracts_required_fields() {
    let provider = ScriptedProvider::new(vec![response(vec![
        ("invoice_number", json!("INV-2026-0042"), Some(0.99)),
        ("total", json!(1542.75), Some(0.98)),
        (
            "line_items",
            json!([
                {"description": "Implementation", "amount": 1200.0},
                {"description": "Support", "amount": 342.75}
            ]),
            Some(0.96),
        ),
    ])]);
    let outcome = extract_document(
        &provider,
        DocumentExtractionRequest {
            schema: invoice_schema(),
            document: document(),
        },
    )
    .unwrap();
    assert_eq!(
        outcome.fields["invoice_number"].value,
        json!("INV-2026-0042")
    );
    assert_eq!(outcome.fields["total"].value, json!(1542.75));
    assert!(outcome.fields["line_items"].confidence >= 0.95);
}

#[test]
fn receipt_fixture_extracts_merchant_tax_total_and_items() {
    let receipt_schema = schema(json!({
        "required": ["merchant", "total", "tax", "items"],
        "properties": {
            "merchant": {"type": "string"},
            "total": {"type": "number"},
            "tax": {"type": "number"},
            "items": {"type": "array"}
        }
    }));
    let provider = ScriptedProvider::new(vec![response(vec![
        ("merchant", json!("North Market"), Some(0.98)),
        ("total", json!(47.30), Some(0.97)),
        ("tax", json!(3.10), Some(0.96)),
        ("items", json!(["coffee", "notebook"]), Some(0.95)),
    ])]);
    let outcome = extract_document(
        &provider,
        DocumentExtractionRequest {
            schema: receipt_schema,
            document: document(),
        },
    )
    .unwrap();
    assert_eq!(outcome.fields["merchant"].value, json!("North Market"));
    assert_eq!(outcome.fields["items"].value.as_array().unwrap().len(), 2);
}

#[test]
fn scanned_contract_fixture_extracts_parties_and_dates() {
    let contract_schema = schema(json!({
        "required": ["party_a", "party_b", "effective_date", "renewal_date"],
        "properties": {
            "party_a": {"type": "string"},
            "party_b": {"type": "string"},
            "effective_date": {"type": "string"},
            "renewal_date": {"type": "string"}
        }
    }));
    let provider = ScriptedProvider::new(vec![response(vec![
        ("party_a", json!("Skycot Pty Ltd"), Some(0.97)),
        ("party_b", json!("Northwind Services"), Some(0.97)),
        ("effective_date", json!("2026-05-14"), Some(0.96)),
        ("renewal_date", json!("2027-05-14"), Some(0.96)),
    ])]);
    let outcome = extract_document(
        &provider,
        DocumentExtractionRequest {
            schema: contract_schema,
            document: provider::DocumentInput {
                page_count: 3,
                ..document()
            },
        },
    )
    .unwrap();
    assert_eq!(outcome.audit.page_count, 3);
    assert_eq!(outcome.fields["party_a"].value, json!("Skycot Pty Ltd"));
}

#[test]
fn claude_payload_contains_image_and_schema_prompt() {
    let transport = StaticTransport {
        response: anthropic_text_response(
            r#"{"fields":{"total":{"value":12.0,"confidence":0.9}}}"#,
        ),
        seen_payload: Rc::new(RefCell::new(None)),
    };
    let seen_payload = Rc::clone(&transport.seen_payload);
    let provider = ClaudeVisionProvider::new("claude-sonnet-4-5", transport);
    let schema = schema(json!({
        "required": ["total"],
        "properties": {"total": {"type": "number"}}
    }));
    let outcome = extract_document(
        &provider,
        DocumentExtractionRequest {
            schema,
            document: document(),
        },
    )
    .unwrap();
    assert_eq!(outcome.audit.provider, "anthropic");
    let payload = seen_payload.borrow();
    let payload = payload.as_ref().unwrap();
    assert_eq!(payload["model"], "claude-sonnet-4-5");
    assert_eq!(payload["messages"][0]["content"][0]["type"], "image");
    assert!(payload["messages"][0]["content"][1]["text"]
        .as_str()
        .unwrap()
        .contains("total"));
}
