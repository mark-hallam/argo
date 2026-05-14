use super::provider::{
    DocumentInput, ExtractionError, FieldCandidate, ProviderRequest, ProviderResponse,
    ProviderUsage,
};
use super::tables::{extract_tables, row_accuracy, TableExtractionRequest};
use serde_json::{json, Value};
use std::cell::RefCell;
use std::collections::BTreeMap;

struct ScriptedProvider {
    responses: RefCell<Vec<ProviderResponse>>,
    requests: RefCell<Vec<ProviderRequest>>,
}

impl ScriptedProvider {
    fn new(response: ProviderResponse) -> Self {
        Self {
            responses: RefCell::new(vec![response]),
            requests: RefCell::new(Vec::new()),
        }
    }
}

impl super::provider::ExtractionProvider for ScriptedProvider {
    fn provider_name(&self) -> &str {
        "fixture-provider"
    }

    fn model(&self) -> &str {
        "fixture-model"
    }

    fn extract(&self, request: &ProviderRequest) -> Result<ProviderResponse, ExtractionError> {
        self.requests.borrow_mut().push(request.clone());
        self.responses
            .borrow_mut()
            .pop()
            .ok_or_else(|| ExtractionError::Protocol("script exhausted".into()))
    }
}

fn response(tables: Value) -> ProviderResponse {
    ProviderResponse {
        fields: BTreeMap::from([(
            "tables".into(),
            FieldCandidate {
                value: tables,
                confidence: Some(0.99),
            },
        )]),
        usage: ProviderUsage {
            input_tokens: 100,
            output_tokens: 50,
            cost_usd: 0.002,
        },
    }
}

fn document(mime_type: &str, page_count: u32) -> DocumentInput {
    DocumentInput {
        mime_type: mime_type.into(),
        bytes: b"fixture document bytes".to_vec(),
        page_count,
    }
}

fn request(expected_headers: &[&str], mime_type: &str, page_count: u32) -> TableExtractionRequest {
    TableExtractionRequest {
        document: document(mime_type, page_count),
        expected_headers: expected_headers
            .iter()
            .map(|header| (*header).into())
            .collect(),
        table_hint: None,
    }
}

fn row(pairs: &[(&str, Value)]) -> BTreeMap<String, Value> {
    pairs
        .iter()
        .map(|(key, value)| ((*key).into(), value.clone()))
        .collect()
}

#[test]
fn bank_statement_fixture_extracts_rows_above_accuracy_threshold() {
    let provider = ScriptedProvider::new(response(json!([{
        "title": "Statement activity",
        "headers": ["date", "description", "debit", "credit", "balance"],
        "page": 1,
        "confidence": 0.98,
        "rows": [
            ["2026-04-01", "Opening balance", null, null, 1200.00],
            ["2026-04-03", "Payroll", null, 2400.00, 3600.00],
            ["2026-04-04", "Rent", 1500.00, null, 2100.00]
        ]
    }])));
    let expected = vec![
        row(&[
            ("date", json!("2026-04-01")),
            ("description", json!("Opening balance")),
            ("debit", Value::Null),
            ("credit", Value::Null),
            ("balance", json!(1200.00)),
        ]),
        row(&[
            ("date", json!("2026-04-03")),
            ("description", json!("Payroll")),
            ("debit", Value::Null),
            ("credit", json!(2400.00)),
            ("balance", json!(3600.00)),
        ]),
        row(&[
            ("date", json!("2026-04-04")),
            ("description", json!("Rent")),
            ("debit", json!(1500.00)),
            ("credit", Value::Null),
            ("balance", json!(2100.00)),
        ]),
    ];
    let outcome = extract_tables(
        &provider,
        request(
            &["date", "description", "debit", "credit", "balance"],
            "image/png",
            1,
        ),
    )
    .unwrap();
    assert_eq!(outcome.row_count, 3);
    assert!(row_accuracy(&outcome.tables[0].rows, &expected) > 0.95);
    assert_eq!(outcome.audit.provider, "fixture-provider");
}

#[test]
fn ar_aging_fixture_infers_headers_from_object_rows() {
    let provider = ScriptedProvider::new(response(json!([{
        "title": "AR aging",
        "page": 1,
        "rows": [
            {"customer": "Acme Co", "current": 800.00, "30_days": 120.00, "60_days": 0.00},
            {"customer": "Globex", "current": 0.00, "30_days": 450.00, "60_days": 90.00}
        ]
    }])));
    let expected = vec![
        row(&[
            ("30_days", json!(120.00)),
            ("60_days", json!(0.00)),
            ("current", json!(800.00)),
            ("customer", json!("Acme Co")),
        ]),
        row(&[
            ("30_days", json!(450.00)),
            ("60_days", json!(90.00)),
            ("current", json!(0.00)),
            ("customer", json!("Globex")),
        ]),
    ];
    let outcome = extract_tables(&provider, request(&[], "application/pdf", 1)).unwrap();
    assert_eq!(
        outcome.tables[0].headers,
        vec!["30_days", "60_days", "current", "customer"]
    );
    assert!(row_accuracy(&outcome.tables[0].rows, &expected) > 0.95);
}

#[test]
fn multi_page_invoice_line_items_merge_continuation_chunks() {
    let provider = ScriptedProvider::new(response(json!([
        {
            "title": "Invoice line items",
            "headers": ["sku", "description", "qty", "amount"],
            "page_start": 1,
            "page_end": 1,
            "rows": [
                ["A-100", "Discovery workshop", 1, 750.00],
                ["A-200", "Implementation", 2, 1800.00]
            ]
        },
        {
            "title": "Invoice line items",
            "continued_from_previous": true,
            "page_start": 2,
            "page_end": 2,
            "rows": [
                ["A-300", "Support retainer", 1, 500.00]
            ]
        }
    ])));
    let expected = vec![
        row(&[
            ("sku", json!("A-100")),
            ("description", json!("Discovery workshop")),
            ("qty", json!(1)),
            ("amount", json!(750.00)),
        ]),
        row(&[
            ("sku", json!("A-200")),
            ("description", json!("Implementation")),
            ("qty", json!(2)),
            ("amount", json!(1800.00)),
        ]),
        row(&[
            ("sku", json!("A-300")),
            ("description", json!("Support retainer")),
            ("qty", json!(1)),
            ("amount", json!(500.00)),
        ]),
    ];
    let outcome = extract_tables(
        &provider,
        request(
            &["sku", "description", "qty", "amount"],
            "application/pdf",
            2,
        ),
    )
    .unwrap();
    assert_eq!(outcome.tables.len(), 1);
    assert_eq!(outcome.tables[0].page_end, 2);
    assert!(row_accuracy(&outcome.tables[0].rows, &expected) > 0.95);
}

#[test]
fn provider_receives_table_schema_through_base_extractor() {
    let provider = ScriptedProvider::new(response(json!([])));
    let outcome = extract_tables(&provider, request(&[], "image/jpeg", 1)).unwrap();
    assert_eq!(outcome.row_count, 0);
    let request = provider.requests.borrow();
    assert!(request[0].schema.fields["tables"].required);
    assert_eq!(request[0].document.mime_type, "image/jpeg");
}
